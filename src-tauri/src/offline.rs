use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{
        atomic::{AtomicBool, Ordering},
        Arc, LazyLock,
    },
    time::Duration,
};

use base64::Engine;
use axum::{
    body::Body,
    extract::{Json, OriginalUri, Query, State},
    http::{HeaderMap, Method, StatusCode},
    response::{IntoResponse, Response},
    routing::{any, get, post},
    Router,
};
use parking_lot::Mutex;
use regex::Regex;
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::{
    io::{AsyncReadExt, AsyncWriteExt},
    net::TcpListener,
    sync::mpsc,
};
use tokio_native_tls::TlsAcceptor;

const CHAT_DOMAIN: &str = "deceive-localhost.molenzwiebel.xyz";
const CONFIG_URL: &str = "https://clientconfig.rpg.riotgames.com";
const CERT_URL: &str = "https://mln.cx/deceive/localhost.pfx";
const VALID_STATUS: [&str; 4] = ["online", "offline", "away", "mobile"];
const RIOT_PROCESSES: [&str; 4] = [
    "RiotClientServices.exe",
    "VALORANT.exe",
    "VALORANT-Win64-Shipping.exe",
    "RiotClientCrashHandler.exe",
];

static GAMES: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<games\b[^>]*>.*?</games>").expect("games regex"));
static SHOW: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<show>.*?</show>").expect("show regex"));
static SHOW_EMPTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<show\s*/>").expect("show empty regex"));
static STATUS: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?s)<status>.*?</status>").expect("status regex"));
static STATUS_EMPTY: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"<status\s*/>").expect("status empty regex"));
static CAPTURED: LazyLock<Mutex<Option<(Value, std::time::Instant)>>> =
    LazyLock::new(|| Mutex::new(None));
static CAPTURE_PATH: LazyLock<Mutex<Option<std::path::PathBuf>>> =
    LazyLock::new(|| Mutex::new(None));
static LOG_PATH: LazyLock<Mutex<Option<PathBuf>>> = LazyLock::new(|| Mutex::new(None));

fn log(message: &str) {
    if let Some(path) = LOG_PATH.lock().clone() {
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
        {
            use std::io::Write;
            let _ = writeln!(file, "{message}");
        }
    }
}

pub const HELPER_FLAG: &str = "--offline-helper";
pub const DATA_DIR_FLAG: &str = "--data-dir";

pub fn offline_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("offline")
}

pub fn helper_state_path(data_dir: &Path) -> PathBuf {
    offline_dir(data_dir).join("helper.json")
}

pub fn helper_exe_path(data_dir: &Path) -> PathBuf {
    offline_dir(data_dir).join("helper.exe")
}

pub fn capture_path(data_dir: &Path) -> PathBuf {
    offline_dir(data_dir).join("presence.json")
}

pub fn set_capture_path(path: PathBuf) {
    *CAPTURE_PATH.lock() = Some(path);
}

pub fn captured_private() -> Option<Value> {
    {
        let mut captured = CAPTURED.lock();
        if let Some((value, at)) = captured.as_ref() {
            if at.elapsed() < Duration::from_secs(120) {
                return Some(value.clone());
            }
        }
        *captured = None;
    }
    let path = CAPTURE_PATH.lock().clone()?;
    let raw = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&raw).ok()?;
    let at = value.get("at").and_then(Value::as_u64).unwrap_or(0);
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or(0);
    if now.saturating_sub(at) > 120 {
        return None;
    }
    value.get("private").cloned()
}

fn capture_presence(stanza: &str) {
    let Some(start) = stanza.find("<valorant") else {
        return;
    };
    let Some(p_start) = stanza[start..].find("<p>") else {
        return;
    };
    let content_start = start + p_start + 3;
    let Some(p_end) = stanza[content_start..].find("</p>") else {
        return;
    };
    let encoded = &stanza[content_start..content_start + p_end];
    let Ok(bytes) = base64::engine::general_purpose::STANDARD.decode(encoded.trim()) else {
        return;
    };
    let Ok(value) = serde_json::from_slice::<Value>(&bytes) else {
        return;
    };
    if value.is_object() {
        *CAPTURED.lock() = Some((value.clone(), std::time::Instant::now()));
        if let Some(path) = CAPTURE_PATH.lock().clone() {
            let now = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_secs())
                .unwrap_or(0);
            if let Some(parent) = path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(
                path,
                serde_json::to_vec(&json!({ "at": now, "private": value })).unwrap_or_default(),
            );
        }
    }
}

#[derive(Clone)]
struct Target {
    host: Option<String>,
    port: u16,
}

struct Connection {
    upstream: mpsc::UnboundedSender<Vec<u8>>,
    client: mpsc::UnboundedSender<Vec<u8>>,
    last_presence: Mutex<Option<String>>,
    version: Mutex<Option<String>>,
    inserted: AtomicBool,
}

const FAKE_PUUID: &str = "5ca07a5c-0ff1-4c0d-9e00-000000000001";
const FAKE_RES: &str = "RC-Valotrack";
const ROSTER_MARKER: &str = "<query xmlns='jabber:iq:riotgames:roster'>";

fn fake_jid() -> String {
    format!("{FAKE_PUUID}@eu1.pvp.net")
}

fn fake_roster_item() -> Vec<u8> {
    format!(
        "<item jid='{}/{FAKE_RES}' name='&#9;Valotrack Active' subscription='both' puuid='{FAKE_PUUID}'>\
<group priority='9999'>Valotrack</group><state>online</state>\
<id name='&#9;Valotrack Active' tagline='OFFLINE'/>\
<lol name='&#9;Valotrack Active'/>\
<platforms><riot name='&#9;Valotrack Active' tagline='OFFLINE'/></platforms></item>",
        fake_jid()
    )
    .into_bytes()
}

fn fake_presence(status: &str, version: Option<&str>) -> Vec<u8> {
    let stamp = chrono::Utc::now().timestamp_millis();
    let private = json!({
        "isValid": true,
        "isIdle": false,
        "queueId": "competitive",
        "provisioningFlow": "Invalid",
        "partyId": "00000000-0000-0000-0000-000000000000",
        "partySize": 1,
        "maxPartySize": 5,
        "partyOwnerMatchScoreAllyTeam": 0,
        "partyOwnerMatchScoreEnemyTeam": 0,
        "premierPresenceData": {
            "rosterId": "",
            "rosterName": format!("{} Mode Active", capitalize(status)),
            "rosterTag": "Valotrack", "rosterType": "VCT",
            "division": 0, "score": 0, "plating": 0,
            "showAura": false, "showTag": true, "showPlating": false
        },
        "matchPresenceData": {
            "sessionLoopState": "MENUS", "provisioningFlow": "Invalid",
            "matchMap": "", "queueId": "competitive"
        },
        "partyPresenceData": {
            "partyId": "00000000-0000-0000-0000-000000000000",
            "isPartyOwner": true, "partyState": "DEFAULT",
            "partyAccessibility": "CLOSED", "partyLFM": false,
            "partyClientVersion": version.unwrap_or("unknown"),
            "partyVersion": stamp, "partySize": 1, "maxPartySize": 5,
            "queueEntryTime": "0001.01.01-00.00.00",
            "isPartyCrossPlayEnabled": false, "isPlayerCrossPlayEnabled": false,
            "partyPrecisePlatformTypes": 1,
            "customGameName": "Valotrack Active", "customGameTeam": "",
            "tournamentId": "", "rosterId": "",
            "partyOwnerSessionLoopState": "MENUS",
            "partyOwnerMatchMap": "", "partyOwnerProvisioningFlow": "Invalid",
            "partyOwnerMatchScoreAllyTeam": 0, "partyOwnerMatchScoreEnemyTeam": 0
        },
        "playerPresenceData": {
            "playerCardId": "d93ad22d-4db7-b6bc-5e9c-e5959bb9dd76",
            "playerTitleId": "e3ca05a4-4e44-9afe-3791-7d96ca8f71fa",
            "accountLevel": 999, "competitiveTier": 27, "leaderboardPosition": 1
        }
    });
    let encoded = base64::engine::general_purpose::STANDARD
        .encode(serde_json::to_vec(&private).unwrap_or_default());
    format!(
        "<presence from='{}/{FAKE_RES}' id='b-{}'>\
<games><keystone><st>chat</st><s.t>{stamp}</s.t><s.p>keystone</s.p><pty/></keystone>\
<league_of_legends><st>chat</st><s.t>{stamp}</s.t><s.p>league_of_legends</s.p>\
<s.c>live</s.c><p>{{&quot;pty&quot;:true}}</p></league_of_legends>\
<valorant><st>chat</st><s.t>{stamp}</s.t><s.p>valorant</s.p><s.r>PC</s.r><p>{encoded}</p><pty/></valorant>\
<bacon><st>chat</st><s.t>{stamp}</s.t><s.l>bacon_availability_online</s.l><s.p>bacon</s.p></bacon></games>\
<show>chat</show><platform>riot</platform><status/></presence>",
        fake_jid(),
        uuid::Uuid::new_v4()
    )
    .into_bytes()
}

fn fake_message(text: &str) -> Vec<u8> {
    let stamp = chrono::Utc::now().format("%Y-%m-%d %H:%M:%S.000");
    format!(
        "<message from='{}/{FAKE_RES}' stamp='{stamp}' id='valotrack-{}' type='chat'><body>{}</body></message>",
        fake_jid(),
        uuid::Uuid::new_v4(),
        text
    )
    .into_bytes()
}

fn capitalize(value: &str) -> String {
    let mut chars = value.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn strip_fake_stanzas(text: &str) -> String {
    let mut out = String::new();
    let mut cursor = 0;
    while let Some(relative) = text[cursor..].find('<') {
        let start = cursor + relative;
        let Some(tag_end) = text[start..].find('>') else {
            break;
        };
        let tag = &text[start + 1..start + tag_end];
        let name = tag.split_whitespace().next().unwrap_or("");
        if !["message", "iq", "presence"].contains(&name) {
            out.push_str(&text[cursor..start + tag_end + 1]);
            cursor = start + tag_end + 1;
            continue;
        }
        let Some(close) = text[start..].find(&format!("</{name}>")) else {
            out.push_str(&text[cursor..]);
            return out;
        };
        let end = start + close + name.len() + 3;
        let stanza = &text[start..end];
        if !stanza.contains(FAKE_PUUID) {
            out.push_str(&text[cursor..end]);
        }
        cursor = end;
    }
    out.push_str(&text[cursor..]);
    out
}

fn handle_fake_command(engine: &OfflineEngine, text: &str) {
    let Some(start) = text.find("<body>") else {
        return;
    };
    let Some(end) = text[start..].find("</body>") else {
        return;
    };
    let body = text[start + 6..start + end].trim().to_lowercase();
    match body.as_str() {
        "offline" | "away" | "mobile" | "online" => {
            let _ = engine.set_status(&body);
        }
        "status" => {
            let status = engine.status.lock().clone();
            engine.push_client_message(&format!("You're currently appearing {}.", status.to_uppercase()));
        }
        "help" => engine.push_client_message(
            "Commands: online / offline / away / mobile / status / help",
        ),
        _ => engine.push_client_message(
            "Didn't catch that. Try: online / offline / away / mobile / status / help",
        ),
    }
}

pub struct OfflineEngine {
    status: Mutex<String>,
    running: Mutex<bool>,
    connected: Mutex<bool>,
    config_port: Mutex<Option<u16>>,
    chat_port: Mutex<Option<u16>>,
    target: Mutex<Target>,
    affinity_auth: Mutex<Option<String>>,
    connections: Mutex<Vec<Arc<Connection>>>,
    status_path: PathBuf,
    cert_path: PathBuf,
    http: reqwest::Client,
}

impl OfflineEngine {
    pub fn new(data_dir: &Path, http: reqwest::Client) -> Arc<Self> {
        let dir = offline_dir(data_dir);
        let _ = std::fs::create_dir_all(&dir);
        set_capture_path(capture_path(data_dir));
        *LOG_PATH.lock() = Some(dir.join("engine.log"));
        let status = std::fs::read_to_string(dir.join("status.txt"))
            .ok()
            .map(|value| value.trim().to_lowercase())
            .filter(|value| VALID_STATUS.contains(&value.as_str()))
            .unwrap_or_else(|| "offline".into());
        Arc::new(Self {
            status: Mutex::new(status),
            running: Mutex::new(false),
            connected: Mutex::new(false),
            config_port: Mutex::new(None),
            chat_port: Mutex::new(None),
            target: Mutex::new(Target {
                host: None,
                port: 5223,
            }),
            affinity_auth: Mutex::new(None),
            connections: Mutex::new(Vec::new()),
            status_path: dir.join("status.txt"),
            cert_path: dir.join("chat.pfx"),
            http,
        })
    }

    pub fn status(&self) -> Value {
        let status = self.status.lock().clone();
        let running = *self.running.lock();
        let connected = *self.connected.lock();
        json!({
            "running": running,
            "active": running && connected && status != "online",
            "connected": connected,
            "status": status,
            "enabled": status != "online",
            "configPort": *self.config_port.lock(),
            "chatPort": *self.chat_port.lock(),
            "message": if !running {
                "Relay is stopped."
            } else if connected {
                "Connected to Riot chat. Friends see your chosen status."
            } else {
                "Relay running. Launch VALORANT to connect."
            }
        })
    }

    pub async fn start(self: &Arc<Self>) -> Result<(), String> {
        if *self.running.lock() {
            return Ok(());
        }
        *CAPTURED.lock() = None;
        let pfx = self.certificate().await?;
        let identity = native_tls::Identity::from_pkcs12(&pfx, "")
            .or_else(|_| native_tls::Identity::from_pkcs12(&pfx, "deceive"))
            .map_err(|error| format!("Could not load the offline certificate: {error}"))?;
        let acceptor = TlsAcceptor::from(
            native_tls::TlsAcceptor::new(identity)
                .map_err(|error| format!("Could not create the TLS acceptor: {error}"))?,
        );

        let chat_listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("Could not bind the chat relay: {error}"))?;
        let chat_port = chat_listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        *self.chat_port.lock() = Some(chat_port);

        let engine = self.clone();
        tokio::spawn(async move {
            loop {
                let Ok((stream, _)) = chat_listener.accept().await else {
                    continue;
                };
                let engine = engine.clone();
                let acceptor = acceptor.clone();
                tokio::spawn(async move {
                    if let Ok(stream) = acceptor.accept(stream).await {
                        let _ = handle_chat(engine, stream).await;
                    }
                });
            }
        });

        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| format!("Could not bind the config relay: {error}"))?;
        let config_port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        *self.config_port.lock() = Some(config_port);
        let engine = self.clone();
        let router = Router::new()
            .route("/{*path}", any(config_handler))
            .with_state(engine);
        tokio::spawn(async move {
            let _ = axum::serve(listener, router).await;
        });
        *self.running.lock() = true;
        Ok(())
    }

    async fn certificate(&self) -> Result<Vec<u8>, String> {
        if let Ok(bytes) = std::fs::read(&self.cert_path) {
            if !bytes.is_empty() {
                let age = std::fs::metadata(&self.cert_path)
                    .and_then(|meta| meta.modified())
                    .ok()
                    .and_then(|time| time.elapsed().ok())
                    .unwrap_or(Duration::MAX);
                if age < Duration::from_secs(7 * 86400) {
                    return Ok(bytes);
                }
            }
        }
        let response = self
            .http
            .get(CERT_URL)
            .timeout(Duration::from_secs(15))
            .send()
            .await
            .map_err(|error| format!("Could not fetch the offline certificate: {error}"))?;
        if !response.status().is_success() {
            return Err(format!(
                "The offline certificate host returned {}.",
                response.status()
            ));
        }
        let bytes = response
            .bytes()
            .await
            .map_err(|error| error.to_string())?
            .to_vec();
        std::fs::write(&self.cert_path, &bytes).map_err(|error| error.to_string())?;
        Ok(bytes)
    }

    pub fn set_status(&self, status: &str) -> Result<Value, String> {
        let status = status.trim().to_lowercase();
        if !VALID_STATUS.contains(&status.as_str()) {
            return Err(format!("Unknown status '{status}'."));
        }
        *self.status.lock() = status.clone();
        let _ = std::fs::write(&self.status_path, &status);
        for connection in self.connections.lock().iter() {
            if let Some(raw) = connection.last_presence.lock().clone() {
                let payload = if status == "online" {
                    raw
                } else {
                    rewrite_presence(&raw, &status)
                };
                let _ = connection.upstream.send(payload.into_bytes());
            }
            let version = connection.version.lock().clone();
            let _ = connection
                .client
                .send(fake_presence(&status, version.as_deref()));
        }
        self.push_client_message(status_message(&status));
        Ok(json!({
            "ok": true,
            "status": status,
            "enabled": status != "online",
            "message": status_message(&status)
        }))
    }

    fn push_client_message(&self, text: &str) {
        for connection in self.connections.lock().iter() {
            let _ = connection.client.send(fake_message(text));
        }
    }

    pub fn set_enabled(&self, enabled: bool) -> Result<Value, String> {
        if enabled {
            let current = self.status.lock().clone();
            self.set_status(if current == "online" { "offline" } else { &current })
        } else {
            self.set_status("online")
        }
    }

    pub async fn launch(self: &Arc<Self>, status: Option<&str>) -> Result<Value, String> {
        if let Some(status) = status {
            self.set_status(status)?;
        }
        self.start().await?;
        let Some(client) = find_riot_client() else {
            return Err("Couldn't find the Riot Client. Is VALORANT installed?".into());
        };
        kill_riot();
        tokio::time::sleep(Duration::from_secs(2)).await;
        let port = self
            .config_port
            .lock()
            .ok_or_else(|| "Offline relay is not ready.".to_string())?;
        let mut command = Command::new(&client);
        command
            .arg(format!("--client-config-url=http://127.0.0.1:{port}"))
            .arg("--launch-product=valorant")
            .arg("--launch-patchline=live")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        command
            .spawn()
            .map_err(|error| format!("Couldn't start the Riot Client: {error}"))?;
        let status = self.status.lock().clone();
        Ok(json!({
            "ok": true,
            "status": status,
            "message": format!(
                "Launching VALORANT with the offline relay. Sign in as usual; friends will see you as {status}."
            )
        }))
    }
}

async fn config_handler(
    State(engine): State<Arc<OfflineEngine>>,
    method: Method,
    OriginalUri(uri): OriginalUri,
    headers: HeaderMap,
) -> Response {
    let path = uri
        .path_and_query()
        .map(|value| value.as_str())
        .unwrap_or("/");
    let mut request = engine.http.request(method, format!("{CONFIG_URL}{path}"));
    for name in [
        "authorization",
        "x-riot-entitlements-jwt",
        "user-agent",
        "accept",
    ] {
        if let Some(value) = headers.get(name).and_then(|value| value.to_str().ok()) {
            request = request.header(name, value);
        }
    }
    let response = match request.send().await {
        Ok(response) => response,
        Err(error) => {
            return Response::builder()
                .status(StatusCode::BAD_GATEWAY)
                .body(Body::from(error.to_string()))
                .unwrap()
        }
    };
    log(&format!("config {} -> {}", uri.path(), response.status()));
    let status = response.status();
    let content_type = response
        .headers()
        .get("content-type")
        .and_then(|value| value.to_str().ok())
        .unwrap_or("application/json")
        .to_string();
    let bytes = response.bytes().await.unwrap_or_default();
    let body = if content_type.to_lowercase().contains("json") {
        if let Ok(mut value) = serde_json::from_slice::<Value>(&bytes) {
            let auth = headers
                .get("authorization")
                .and_then(|value| value.to_str().ok())
                .map(str::to_string);
            rewrite_config(&engine, &mut value, auth.as_deref()).await;
            serde_json::to_vec(&value).unwrap_or_else(|_| bytes.to_vec())
        } else {
            bytes.to_vec()
        }
    } else {
        bytes.to_vec()
    };
    Response::builder()
        .status(status)
        .header("content-type", content_type)
        .header("content-length", body.len())
        .body(Body::from(body))
        .unwrap()
}

async fn rewrite_config(
    engine: &OfflineEngine,
    config: &mut Value,
    auth: Option<&str>,
) {
    let affinity_host = config
        .get("chat.affinities")
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default();
    let affinity_enabled = config
        .get("chat.affinity.enabled")
        .and_then(Value::as_bool)
        .unwrap_or(false);
    let cached_auth = engine.affinity_auth.lock().clone();
    let mut resolved = None;
    if affinity_enabled {
        if let Some(auth) = auth {
            if cached_auth.as_deref() != Some(auth) {
                if let Some(host) = resolve_affinity(engine, &affinity_host, auth).await {
                    *engine.affinity_auth.lock() = Some(auth.to_string());
                    resolved = Some(host);
                }
            }
        }
    }
    let host = resolved.or_else(|| {
        if cached_auth.is_some() {
            return None;
        }
        config
            .get("chat.host")
            .and_then(Value::as_str)
            .filter(|host| !host.is_empty())
            .map(str::to_string)
    });
    if let Some(host) = host {
        log(&format!("config chat host -> {host}"));
        engine.target.lock().host = Some(host);
    }
    if let Some(port) = config.get("chat.port").and_then(Value::as_u64) {
        engine.target.lock().port = port as u16;
    }
    let chat_port = *engine.chat_port.lock();
    if let Some(port) = chat_port {
        config["chat.port"] = json!(port);
    }
    config["chat.host"] = json!(CHAT_DOMAIN);
    if !affinity_host.is_empty() {
        config["chat.affinities"] = json!(affinity_host
            .keys()
            .map(|key| (key.clone(), json!(CHAT_DOMAIN)))
            .collect::<serde_json::Map<_, _>>());
    }
}

async fn resolve_affinity(
    engine: &OfflineEngine,
    affinities: &serde_json::Map<String, Value>,
    auth: &str,
) -> Option<String> {
    let response = engine
        .http
        .get("https://riot-geo.pas.si.riotgames.com/pas/v1/service/chat")
        .header("Authorization", auth)
        .timeout(Duration::from_secs(15))
        .send()
        .await
        .ok()?;
    let text = response.text().await.ok()?;
    let payload = text.split('.').nth(1)?;
    let bytes = base64::engine::general_purpose::URL_SAFE_NO_PAD
        .decode(payload)
        .ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    let affinity = value.get("affinity").and_then(Value::as_str)?;
    let host = affinities
        .get(affinity)
        .and_then(Value::as_str)
        .map(str::to_string);
    log(&format!("config affinity {affinity} -> {host:?}"));
    host
}

async fn handle_chat(
    engine: Arc<OfflineEngine>,
    stream: tokio_native_tls::TlsStream<tokio::net::TcpStream>,
) -> Result<(), String> {
    let (host, port) = {
        let target = engine.target.lock();
        (target.host.clone(), target.port)
    };
    let Some(host) = host else {
        log("chat connection closed: no upstream host captured");
        return Ok(());
    };
    log(&format!("chat connecting upstream {host}:{port}"));
    let connector = tokio_native_tls::TlsConnector::from(
        native_tls::TlsConnector::builder()
            .build()
            .map_err(|error| error.to_string())?,
    );
    let tcp = match tokio::net::TcpStream::connect((host.as_str(), port)).await {
        Ok(tcp) => tcp,
        Err(error) => {
            log(&format!("chat upstream TCP failed: {error}"));
            return Err(error.to_string());
        }
    };
    let upstream = match connector.connect(&host, tcp).await {
        Ok(stream) => stream,
        Err(error) => {
            log(&format!("chat upstream TLS failed: {error}"));
            return Err(error.to_string());
        }
    };
    log("chat upstream TLS connected");

    let (mut client_read, mut client_write) = tokio::io::split(stream);
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (upstream_tx, mut upstream_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let connection = Arc::new(Connection {
        upstream: upstream_tx.clone(),
        client: client_tx.clone(),
        last_presence: Mutex::new(None),
        version: Mutex::new(None),
        inserted: AtomicBool::new(false),
    });
    engine.connections.lock().push(connection.clone());
    *engine.connected.lock() = true;

    let writer_client = tokio::spawn(async move {
        while let Some(data) = client_rx.recv().await {
            if client_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });
    let writer_upstream = tokio::spawn(async move {
        while let Some(data) = upstream_rx.recv().await {
            if upstream_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let engine_in = engine.clone();
    let connection_pump = connection.clone();
    let client_pump = tokio::spawn(async move {
        let mut buffer = String::new();
        let mut pending_bytes: Vec<u8> = Vec::new();
        let mut chunk = vec![0u8; 65536];
        while let Ok(read) = client_read.read(&mut chunk).await {
            if read == 0 {
                log("c2s closed");
                break;
            }
            pending_bytes.extend_from_slice(&chunk[..read]);
            let valid = match std::str::from_utf8(&pending_bytes) {
                Ok(_) => pending_bytes.len(),
                Err(error) if error.error_len().is_none() => error.valid_up_to(),
                Err(_) => pending_bytes.len(),
            };
            let mut text = String::from_utf8_lossy(&pending_bytes[..valid]).to_string();
            pending_bytes.drain(..valid);
            if text.contains(FAKE_PUUID) {
                handle_fake_command(&engine_in, &text);
                text = strip_fake_stanzas(&text);
            }
            buffer.push_str(&text);
            let status = engine_in.status.lock().clone();
            let (out, remaining, last) = process_presences(&buffer, &status);
            buffer = remaining;
            if let Some(raw) = last {
                if let Some(version) = version_from_presence(&raw) {
                    let mut current = connection_pump.version.lock();
                    if current.as_deref() != Some(version.as_str()) {
                        *current = Some(version.clone());
                        if connection_pump.inserted.load(Ordering::Relaxed) {
                            let status = engine_in.status.lock().clone();
                            let _ = connection_pump
                                .client
                                .send(fake_presence(&status, Some(&version)));
                        }
                    }
                }
                *connection_pump.last_presence.lock() = Some(raw);
            }
            if !out.is_empty() && upstream_tx.send(out.into_bytes()).is_err() {
                break;
            }
        }
    });

    let client_out = client_tx.clone();
    let engine_out = engine.clone();
    let connection_out = connection.clone();
    let upstream_pump = tokio::spawn(async move {
        let mut chunk = vec![0u8; 65536];
        let mut carry: Vec<u8> = Vec::new();
        loop {
            let read = match tokio::time::timeout(
                Duration::from_secs(3),
                upstream_read.read(&mut chunk),
            )
            .await
            {
                Ok(Ok(0)) => {
                    log("s2c closed");
                    break;
                }
                Ok(Ok(read)) => read,
                Ok(Err(_)) => break,
                Err(_) => {
                    if !carry.is_empty() {
                        if client_out.send(std::mem::take(&mut carry)).is_err() {
                            break;
                        }
                    }
                    continue;
                }
            };
            let shape = String::from_utf8_lossy(&chunk[..read]);
            if shape.contains("<failure") {
                log(&format!("s2c failure: {}", shape.chars().take(500).collect::<String>()));
            }
            if !connection_out.inserted.load(Ordering::Relaxed) {
                carry.extend_from_slice(&chunk[..read]);
                if let Some(position) = find_bytes(&carry, ROSTER_MARKER.as_bytes()) {
                    let split = position + ROSTER_MARKER.len();
                    let mut output = carry[..split].to_vec();
                    output.extend(fake_roster_item());
                    output.extend_from_slice(&carry[split..]);
                    if client_out.send(output).is_err() {
                        break;
                    }
                    connection_out.inserted.store(true, Ordering::Relaxed);
                    let status = engine_out.status.lock().clone();
                    let version = connection_out.version.lock().clone();
                    let _ = client_out.send(fake_presence(&status, version.as_deref()));
                    let _ = client_out.send(fake_message(
                        "Valotrack is active. Message offline, away, mobile, online, status or help here anytime.",
                    ));
                    carry.clear();
                } else {
                    let keep = marker_suffix(&carry, ROSTER_MARKER.as_bytes());
                    let split = carry.len().saturating_sub(keep);
                    if split > 0 {
                        if client_out.send(carry[..split].to_vec()).is_err() {
                            break;
                        }
                        carry.drain(..split);
                    }
                }
            } else if client_out.send(chunk[..read].to_vec()).is_err() {
                break;
            }
        }
        if !carry.is_empty() {
            let _ = client_out.send(carry);
        }
    });

    tokio::select! {
        _ = client_pump => {}
        _ = upstream_pump => {}
        _ = writer_client => {}
        _ = writer_upstream => {}
    }
    engine.connections.lock().retain(|item| !Arc::ptr_eq(item, &connection));
    *engine.connected.lock() = !engine.connections.lock().is_empty();
    Ok(())
}

fn process_presences(
    buffer: &str,
    status: &str,
) -> (String, String, Option<String>) {
    let mut out = String::new();
    let mut last = None;
    let mut cursor = 0usize;
    while let Some(relative) = buffer[cursor..].find("<presence") {
        let start = cursor + relative;
        out.push_str(&buffer[cursor..start]);
        let Some(end) = presence_end(&buffer[start..]) else {
            return (out, buffer[start..].to_string(), last);
        };
        let stanza = &buffer[start..start + end];
        let open_end = stanza.find('>').unwrap_or(stanza.len());
        let directed = stanza[..open_end].contains(" to=");
        if !directed {
            capture_presence(stanza);
            last = Some(stanza.to_string());
            if status == "online" {
                out.push_str(stanza);
            } else {
                let rewritten = rewrite_presence(stanza, status);
                out.push_str(&rewritten);
            }
        } else {
            out.push_str(stanza);
        }
        cursor = start + end;
    }
    let tail = &buffer[cursor..];
    let keep = partial_presence_prefix(tail);
    let split = tail.len().saturating_sub(keep);
    out.push_str(&tail[..split]);
    (out, tail[split..].to_string(), last)
}

fn presence_end(stanza: &str) -> Option<usize> {
    let mut quote = None;
    let mut open_end = None;
    for (index, character) in stanza.char_indices() {
        match character {
            '\'' | '"' if quote.is_none() => quote = Some(character),
            value if Some(value) == quote => quote = None,
            '>' if quote.is_none() => {
                open_end = Some(index);
                break;
            }
            _ => {}
        }
    }
    let open_end = open_end?;
    if stanza[..=open_end].trim_end().ends_with("/>") {
        return Some(open_end + 1);
    }
    stanza[open_end..]
        .find("</presence>")
        .map(|index| open_end + index + "</presence>".len())
}

fn version_from_presence(stanza: &str) -> Option<String> {
    let start = stanza.find("<valorant")?;
    let p_start = stanza[start..].find("<p>")?;
    let content_start = start + p_start + 3;
    let p_end = stanza[content_start..].find("</p>")?;
    let encoded = &stanza[content_start..content_start + p_end];
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(encoded.trim())
        .ok()?;
    let value: Value = serde_json::from_slice(&bytes).ok()?;
    value
        .get("partyPresenceData")
        .and_then(|data| data.get("partyClientVersion"))
        .or_else(|| value.get("partyClientVersion"))
        .and_then(Value::as_str)
        .filter(|version| *version != "unknown")
        .map(str::to_string)
}

fn find_bytes(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|window| window == needle)
}

fn marker_suffix(haystack: &[u8], needle: &[u8]) -> usize {
    for length in (1..needle.len()).rev() {
        if haystack.ends_with(&needle[..length]) {
            return length;
        }
    }
    0
}

fn partial_presence_prefix(tail: &str) -> usize {
    let marker = "<presence";
    for length in (1..marker.len()).rev() {
        if tail.ends_with(&marker[..length]) {
            return length;
        }
    }
    0
}

fn rewrite_presence(stanza: &str, status: &str) -> String {
    if !stanza.contains("<presence") {
        return stanza.to_string();
    }
    let mut value = GAMES.replace_all(stanza, "<games></games>").to_string();
    if value.contains("<show>") {
        value = SHOW
            .replace(&value, format!("<show>{status}</show>").as_str())
            .to_string();
    } else if value.contains("<show/>") || value.contains("<show />") {
        value = SHOW_EMPTY
            .replace(&value, format!("<show>{status}</show>").as_str())
            .to_string();
    } else {
        let open_end = value.find('>').unwrap_or(value.len());
        if !value[..open_end].contains("type=") {
            if let Some(close) = value.rfind("</presence>") {
                value.insert_str(close, &format!("<show>{status}</show>"));
            }
        }
    }
    value = STATUS.replace_all(&value, "").to_string();
    value = STATUS_EMPTY.replace_all(&value, "").to_string();
    value
}

fn status_message(status: &str) -> &'static str {
    match status {
        "online" => "You appear online again.",
        "away" => "Friends see you as away.",
        "mobile" => "Friends see you on mobile.",
        _ => "Friends see you offline.",
    }
}

fn find_riot_client() -> Option<String> {
    let path = std::env::var_os("PROGRAMDATA")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"))
        .join("Riot Games")
        .join("RiotClientInstalls.json");
    let text = std::fs::read_to_string(path).ok()?;
    let value: Value = serde_json::from_str(&text).ok()?;
    for key in ["rc_default", "rc_live", "rc_beta"] {
        if let Some(path) = value.get(key).and_then(Value::as_str) {
            if Path::new(path).is_file() {
                return Some(path.to_string());
            }
        }
    }
    None
}

fn kill_riot() {
    for name in RIOT_PROCESSES {
        let mut command = Command::new("taskkill");
        command.args(["/F", "/IM", name]);
        command
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null());
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            command.creation_flags(0x08000000);
        }
        let _ = command.status();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn presence_rewrite_hides_game_activity() {
        let stanza = "<presence from='a'><games><valorant/></games><show>chat</show><status>In game</status></presence>";
        let rewritten = rewrite_presence(stanza, "offline");
        assert!(rewritten.contains("<show>offline</show>"));
        assert!(!rewritten.contains("<status>"));
        assert!(!rewritten.contains("<valorant/>"));
    }

    #[test]
    fn split_presence_is_buffered() {
        let (out, rest, last) = process_presences("<presence from='a'><show>chat</show>", "away");
        assert!(out.is_empty());
        assert!(rest.starts_with("<presence"));
        assert!(last.is_none());
        let (out, rest, last) = process_presences(&(rest + "</presence>"), "away");
        assert!(out.contains("<show>away</show>"));
        assert!(rest.is_empty());
        assert!(last.is_some());
    }

    #[test]
    fn directed_presence_is_untouched() {
        let stanza = "<presence to='b'><show>chat</show></presence>";
        let (out, _, last) = process_presences(stanza, "offline");
        assert_eq!(out, stanza);
        assert!(last.is_none());
    }

    #[test]
    fn marker_suffix_only_holds_partial_tags() {
        assert_eq!(
            marker_suffix(b"payload<query xmlns=", ROSTER_MARKER.as_bytes()),
            b"<query xmlns=".len()
        );
        assert_eq!(marker_suffix(b"stream:features", ROSTER_MARKER.as_bytes()), 0);
    }

    #[test]
    fn fake_friend_commands_switch_status() {
        let dir = std::env::temp_dir().join(format!("valotrack-offline-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let engine = OfflineEngine::new(&dir, reqwest::Client::new());
        let stanza = format!(
            "<message to='{}/{FAKE_RES}' type='chat'><body>away</body></message>",
            fake_jid()
        );
        handle_fake_command(&engine, &stanza);
        assert_eq!(engine.status.lock().as_str(), "away");
        handle_fake_command(&engine, &stanza.replace("away", "online"));
        assert_eq!(engine.status.lock().as_str(), "online");
        let _ = std::fs::remove_dir_all(dir);
    }
}

#[cfg(test)]
mod capture_tests {
    use super::*;
    use base64::Engine;

    #[test]
    fn captures_valorant_private_presence() {
        let payload = json!({
            "matchPresenceData": {
                "sessionLoopState": "INGAME"
            },
            "partyPresenceData": {
                "partyId": "abc"
            }
        });
        let encoded = base64::engine::general_purpose::STANDARD
            .encode(serde_json::to_vec(&payload).unwrap());
        let stanza = format!(
            "<presence from='self'><games><valorant><p>{encoded}</p></valorant></games></presence>"
        );
        capture_presence(&stanza);
        let captured = captured_private().unwrap();
        assert_eq!(captured["matchPresenceData"]["sessionLoopState"], "INGAME");
        assert_eq!(captured["partyPresenceData"]["partyId"], "abc");
    }
}

#[derive(Clone)]
struct HelperContext {
    engine: Arc<OfflineEngine>,
    token: String,
    state_path: PathBuf,
}

#[derive(Deserialize)]
struct HelperAuth {
    t: Option<String>,
}

#[derive(Deserialize)]
struct StatusBody {
    status: Option<String>,
}

#[derive(Deserialize)]
struct EnabledBody {
    enabled: Option<bool>,
}

#[derive(Deserialize)]
struct LaunchBody {
    status: Option<String>,
}

fn helper_authorized(context: &HelperContext, auth: &HelperAuth) -> bool {
    auth.t.as_deref() == Some(context.token.as_str())
}

async fn helper_status(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    Json(context.engine.status()).into_response()
}

async fn helper_presence(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    Json(json!({ "private": captured_private() })).into_response()
}

async fn helper_set_status(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
    Json(body): Json<StatusBody>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    match context
        .engine
        .set_status(body.status.as_deref().unwrap_or("offline"))
    {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

async fn helper_set_enabled(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
    Json(body): Json<EnabledBody>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    match context.engine.set_enabled(body.enabled.unwrap_or(true)) {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

async fn helper_launch(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
    Json(body): Json<LaunchBody>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    match context.engine.launch(body.status.as_deref()).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::BAD_REQUEST, error).into_response(),
    }
}

async fn helper_shutdown(
    State(context): State<HelperContext>,
    Query(auth): Query<HelperAuth>,
) -> Response {
    if !helper_authorized(&context, &auth) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let _ = std::fs::remove_file(&context.state_path);
    tokio::spawn(async {
        tokio::time::sleep(Duration::from_millis(300)).await;
        std::process::exit(0);
    });
    Json(json!({ "ok": true })).into_response()
}

fn helper_token() -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

pub fn run_helper(data_dir: &str) -> Result<(), String> {
    let data_dir = PathBuf::from(data_dir);
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()
        .map_err(|error| error.to_string())?;
    runtime.block_on(async move {
        std::fs::create_dir_all(offline_dir(&data_dir)).map_err(|error| error.to_string())?;
        set_capture_path(capture_path(&data_dir));
        let http = reqwest::Client::builder()
            .danger_accept_invalid_certs(true)
            .user_agent("Valotrack/0.1")
            .timeout(Duration::from_secs(12))
            .build()
            .map_err(|error| error.to_string())?;
        let engine = OfflineEngine::new(&data_dir, http);
        engine.start().await?;
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
            .await
            .map_err(|error| error.to_string())?;
        let port = listener
            .local_addr()
            .map_err(|error| error.to_string())?
            .port();
        let token = helper_token();
        let context = HelperContext {
            engine,
            token: token.clone(),
            state_path: helper_state_path(&data_dir),
        };
        let router = Router::new()
            .route("/status", get(helper_status))
            .route("/presence", get(helper_presence))
            .route("/status", post(helper_set_status))
            .route("/enabled", post(helper_set_enabled))
            .route("/launch", post(helper_launch))
            .route("/shutdown", post(helper_shutdown))
            .with_state(context);
        let info = json!({
            "port": port,
            "token": token,
            "pid": std::process::id(),
            "version": env!("CARGO_PKG_VERSION")
        });
        std::fs::write(
            helper_state_path(&data_dir),
            serde_json::to_vec(&info).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
        axum::serve(listener, router)
            .await
            .map_err(|error| error.to_string())
    })
}
