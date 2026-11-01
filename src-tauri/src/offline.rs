use std::{
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, LazyLock},
    time::Duration,
};

use axum::{
    body::Body,
    extract::{OriginalUri, State},
    http::{HeaderMap, Method, StatusCode},
    response::Response,
    routing::any,
    Router,
};
use parking_lot::Mutex;
use regex::Regex;
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

#[derive(Clone)]
struct Target {
    host: Option<String>,
    port: u16,
}

struct Connection {
    upstream: mpsc::UnboundedSender<Vec<u8>>,
    last_presence: Mutex<Option<String>>,
}

pub struct OfflineEngine {
    status: Mutex<String>,
    running: Mutex<bool>,
    connected: Mutex<bool>,
    config_port: Mutex<Option<u16>>,
    chat_port: Mutex<Option<u16>>,
    target: Mutex<Target>,
    connections: Mutex<Vec<Arc<Connection>>>,
    status_path: PathBuf,
    cert_path: PathBuf,
    http: reqwest::Client,
}

impl OfflineEngine {
    pub fn new(data_dir: &Path, http: reqwest::Client) -> Arc<Self> {
        let dir = data_dir.join("offline");
        let _ = std::fs::create_dir_all(&dir);
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
        tauri::async_runtime::spawn(async move {
            loop {
                let Ok((stream, _)) = chat_listener.accept().await else {
                    continue;
                };
                let engine = engine.clone();
                let acceptor = acceptor.clone();
                tauri::async_runtime::spawn(async move {
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
        tauri::async_runtime::spawn(async move {
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
        }
        Ok(json!({
            "ok": true,
            "status": status,
            "enabled": status != "online",
            "message": status_message(&status)
        }))
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
            rewrite_config(&engine, &mut value);
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

fn rewrite_config(engine: &OfflineEngine, config: &mut Value) {
    if let Some(host) = config.get("chat.host").and_then(Value::as_str) {
        engine.target.lock().host = Some(host.to_string());
    }
    if let Some(port) = config.get("chat.port").and_then(Value::as_u64) {
        engine.target.lock().port = port as u16;
    }
    let chat_port = *engine.chat_port.lock();
    if let Some(port) = chat_port {
        config["chat.port"] = json!(port);
    }
    config["chat.host"] = json!(CHAT_DOMAIN);
    if let Some(affinities) = config.get("chat.affinities").and_then(Value::as_object) {
        let keys: Vec<String> = affinities.keys().cloned().collect();
        config["chat.affinities"] = json!(keys
            .into_iter()
            .map(|key| (key, json!(CHAT_DOMAIN)))
            .collect::<serde_json::Map<_, _>>());
    }
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
        return Ok(());
    };
    let connector = tokio_native_tls::TlsConnector::from(
        native_tls::TlsConnector::builder()
            .build()
            .map_err(|error| error.to_string())?,
    );
    let tcp = tokio::net::TcpStream::connect((host.as_str(), port))
        .await
        .map_err(|error| error.to_string())?;
    let upstream = connector
        .connect(&host, tcp)
        .await
        .map_err(|error| error.to_string())?;

    let (mut client_read, mut client_write) = tokio::io::split(stream);
    let (mut upstream_read, mut upstream_write) = tokio::io::split(upstream);
    let (client_tx, mut client_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let (upstream_tx, mut upstream_rx) = mpsc::unbounded_channel::<Vec<u8>>();
    let connection = Arc::new(Connection {
        upstream: upstream_tx.clone(),
        last_presence: Mutex::new(None),
    });
    engine.connections.lock().push(connection.clone());
    *engine.connected.lock() = true;

    let writer_client = tauri::async_runtime::spawn(async move {
        while let Some(data) = client_rx.recv().await {
            if client_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });
    let writer_upstream = tauri::async_runtime::spawn(async move {
        while let Some(data) = upstream_rx.recv().await {
            if upstream_write.write_all(&data).await.is_err() {
                break;
            }
        }
    });

    let engine_in = engine.clone();
    let connection_pump = connection.clone();
    let client_pump = tauri::async_runtime::spawn(async move {
        let mut buffer = String::new();
        let mut chunk = vec![0u8; 65536];
        while let Ok(read) = client_read.read(&mut chunk).await {
            if read == 0 {
                break;
            }
            let text = String::from_utf8_lossy(&chunk[..read]);
            buffer.push_str(&text);
            let status = engine_in.status.lock().clone();
            let (out, remaining, last) = process_presences(&buffer, &status);
            buffer = remaining;
            if let Some(raw) = last {
                *connection_pump.last_presence.lock() = Some(raw);
            }
            if !out.is_empty() && upstream_tx.send(out.into_bytes()).is_err() {
                break;
            }
        }
    });

    let client_out = client_tx.clone();
    let upstream_pump = tauri::async_runtime::spawn(async move {
        let mut chunk = vec![0u8; 65536];
        while let Ok(read) = upstream_read.read(&mut chunk).await {
            if read == 0 {
                break;
            }
            if client_out.send(chunk[..read].to_vec()).is_err() {
                break;
            }
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
}
