use std::{
    net::UdpSocket,
    sync::Arc,
};

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::{Html, IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use parking_lot::Mutex;
use qrcode::{render::svg, QrCode};
use serde::Deserialize;
use serde_json::{json, Value};
use tokio::net::TcpListener;

use crate::app::AppState;

#[derive(Clone)]
struct RemoteContext {
    app: Arc<AppState>,
    token: String,
}

#[derive(Default)]
pub struct RemoteState {
    info: Mutex<Option<Value>>,
}

impl RemoteState {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn info(&self) -> Value {
        self.info
            .lock()
            .clone()
            .unwrap_or_else(|| json!({ "running": false }))
    }
}

#[derive(Deserialize)]
struct AuthQuery {
    t: Option<String>,
}

#[derive(Deserialize)]
struct ActionBody {
    action: String,
    #[serde(rename = "queueId")]
    queue_id: Option<String>,
    agent: Option<String>,
    mode: Option<String>,
}

pub async fn start(state: &Arc<AppState>) -> Result<Value, String> {
    if let Some(info) = state.remote.info.lock().clone() {
        if info.get("running").and_then(Value::as_bool) == Some(true) {
            return Ok(info);
        }
    }
    let listener = TcpListener::bind("0.0.0.0:0")
        .await
        .map_err(|error| format!("Could not open the remote port: {error}"))?;
    let port = listener
        .local_addr()
        .map_err(|error| error.to_string())?
        .port();
    let token = format!("{:016x}", rand_token());
    let ip = local_ip().unwrap_or_else(|| "127.0.0.1".into());
    let url = format!("http://{ip}:{port}/?t={token}");
    let qr = QrCode::new(url.as_bytes())
        .map_err(|error| error.to_string())?
        .render::<svg::Color>()
        .min_dimensions(220, 220)
        .dark_color(svg::Color("#f1ede4"))
        .light_color(svg::Color("#0b0d12"))
        .build();
    let context = RemoteContext {
        app: state.clone(),
        token: token.clone(),
    };
    let router = Router::new()
        .route("/", get(mobile_page))
        .route("/api/board", get(board))
        .route("/api/agents", get(agents))
        .route("/api/action", post(action))
        .with_state(context);
    tauri::async_runtime::spawn(async move {
        let _ = axum::serve(listener, router).await;
    });
    let info = json!({
        "running": true,
        "url": url,
        "token": token,
        "port": port,
        "qrSvg": qr
    });
    *state.remote.info.lock() = Some(info.clone());
    Ok(info)
}

fn rand_token() -> u64 {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos()
        .hash(&mut hasher);
    std::process::id().hash(&mut hasher);
    hasher.finish()
}

fn local_ip() -> Option<String> {
    let socket = UdpSocket::bind("0.0.0.0:0").ok()?;
    socket.connect("8.8.8.8:80").ok()?;
    socket.local_addr().ok().map(|address| address.ip().to_string())
}

fn authorized(context: &RemoteContext, query: &AuthQuery) -> bool {
    query.t.as_deref() == Some(context.token.as_str())
}

async fn mobile_page() -> Html<&'static str> {
    Html(MOBILE)
}

async fn board(
    State(context): State<RemoteContext>,
    Query(query): Query<AuthQuery>,
) -> Response {
    if !authorized(&context, &query) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    match crate::live::board(&context.app).await {
        Ok(value) => Json(value).into_response(),
        Err(error) => (StatusCode::BAD_GATEWAY, error).into_response(),
    }
}

async fn agents(
    State(context): State<RemoteContext>,
    Query(query): Query<AuthQuery>,
) -> Response {
    if !authorized(&context, &query) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    Json(crate::catalog::agents_json()).into_response()
}

async fn action(
    State(context): State<RemoteContext>,
    Query(query): Query<AuthQuery>,
    Json(body): Json<ActionBody>,
) -> Response {
    if !authorized(&context, &query) {
        return (StatusCode::UNAUTHORIZED, "unauthorized").into_response();
    }
    let result = match body.action.as_str() {
        "select" => {
            crate::actions::queue_action(&context.app, "select", body.queue_id, false).await
        }
        "start" => crate::actions::queue_action(&context.app, "start", None, false).await,
        "stop" => crate::actions::queue_action(&context.app, "stop", None, false).await,
        "side" => crate::actions::check_side(&context.app).await,
        "dodge" => crate::actions::dodge(&context.app, false).await,
        "instalock" => {
            crate::actions::live_instalock(
                &context.app,
                body.agent.as_deref().unwrap_or("Jett"),
                body.mode.as_deref().unwrap_or("lock"),
                false,
            )
            .await
        }
        _ => json!({ "ok": false, "message": "Unknown action." }),
    };
    Json(result).into_response()
}

const MOBILE: &str = r##"<!doctype html>
<html lang="en"><head><meta charset="utf-8"/><meta name="viewport" content="width=device-width,initial-scale=1"/><title>Valotrack Remote</title>
<style>
*{box-sizing:border-box}body{margin:0;background:#08090c;color:#f1ede4;font-family:-apple-system,Segoe UI,sans-serif;padding:14px 12px 90px}
header{display:flex;align-items:center;justify-content:space-between;margin-bottom:12px}
.logo{font-weight:900;letter-spacing:.08em;font-size:14px}.logo span{color:#ff4655}
.brand{display:flex;align-items:center;gap:8px}
.mark{display:block;width:28px;height:28px;overflow:hidden;border-radius:5px;background:#fff}
.mark svg{display:block;width:100%;height:100%}
.pill{font:10px ui-monospace,monospace;color:#8b93a1;text-transform:uppercase;border:1px solid #ffffff16;border-radius:99px;padding:4px 8px}
.card{background:#0f1117;border:1px solid #ffffff12;border-radius:8px;padding:11px;margin-bottom:9px}
.kicker{font:9px ui-monospace,monospace;letter-spacing:.12em;text-transform:uppercase;color:#5b6270}
h1{font-size:26px;line-height:1;margin:4px 0 0;text-transform:uppercase}
h2{font-size:18px;margin:2px 0 8px;text-transform:uppercase}
.score{display:flex;gap:16px;margin-top:6px;font-size:22px;font-weight:900}.score span{color:#8b93a1;font-size:11px;display:block;text-align:center}
.team{margin-top:7px}.teamhead{display:flex;justify-content:space-between;font:9px ui-monospace,monospace;color:#8b93a1;text-transform:uppercase;margin-bottom:4px}
.row{display:grid;grid-template-columns:20px 1fr 62px 40px;gap:5px;align-items:center;padding:6px 0;border-bottom:1px solid #ffffff0d;font-size:11px}
.row.self{background:#ff46550d;margin:0 -5px;padding:6px 5px}
.agent{width:18px;height:18px;border-radius:3px;object-fit:cover;background:#1b2029}
.name{font-weight:700;white-space:nowrap;overflow:hidden;text-overflow:ellipsis}.rank{font-size:9px;text-align:right;white-space:nowrap}.kd{font:9px ui-monospace,monospace;text-align:right;color:#8b93a1}
.smurf{font:8px ui-monospace,monospace;color:#e8b45a;margin-left:4px}
.actions{position:fixed;bottom:0;left:0;right:0;padding:9px 12px;background:#0b0d12ee;border-top:1px solid #ffffff12;display:flex;gap:6px;backdrop-filter:blur(10px)}
button{border:1px solid #ffffff1a;background:#141821;color:#f1ede4;border-radius:6px;padding:9px 10px;font-size:11px;font-weight:700;flex:1}
button.red{background:#ff4655;border-color:#ff4655;color:#fff}button.ghost{flex:0 0 auto}button:disabled{opacity:.4}
select{flex:1;background:#141821;color:#f1ede4;border:1px solid #ffffff1a;border-radius:6px;padding:8px;font-size:11px}
.empty{color:#8b93a1;font-size:12px;line-height:1.5;text-align:center;padding:26px 12px}
.toast{position:fixed;left:12px;right:12px;bottom:62px;background:#141821;border:1px solid #ff465544;border-radius:6px;padding:9px;font-size:10px;color:#f1ede4;display:none}
</style></head><body>
<header><div class="brand"><span class="mark"><svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 800 800"><rect width="800" height="800" fill="#fff"/><path d="M100 110 L100 290 L300 650 L500 650 Z" fill="#000"/><path d="M700 110 L700 290 L560 490 L440 490 Z" fill="#000"/></svg></span><div class="logo">VALO<span>TRACK</span> <small style="color:#5b6270;font-weight:400">REMOTE</small></div></div><div id="state" class="pill">connecting</div></header>
<main id="app"><div class="empty">Reading the local client…</div></main>
<div class="toast" id="toast"></div>
<div class="actions">
<select id="agents"></select>
<button id="lock">Lock</button>
<button id="queue" class="red">Start queue</button>
</div>
<script>
const t=new URLSearchParams(location.search).get('t');
let inQueue=false;
async function load(){
 try{
  const b=await fetch('/api/board?t='+t).then(r=>r.json());
  render(b);
 }catch(e){document.getElementById('state').textContent='offline'}
}
function render(b){
 const state=document.getElementById('state');
 state.textContent=b.stateLabel||b.state||'offline';
 inQueue=!!(b.queue&&b.queue.inQueue);
 const teams=b.teams||{}; let html='';
 if(!b.players||!b.players.length){html='<div class="card"><div class="kicker">'+(b.map||'VALORANT')+'</div><h1>'+(b.stateLabel||'Offline')+'</h1><div class="empty">'+(b.notice?.message||'Open VALORANT and join a lobby to see live stats.')+'</div></div>'}
 else{
  html+='<div class="card"><div class="kicker">'+(b.mode||'')+' · '+(b.stateLabel||'')+'</div><h1>'+(b.map||'Lobby')+'</h1>';
  if(b.score)html+='<div class="score"><div>'+(b.score.ally||0)+'<span>YOU</span></div><div>'+(b.score.enemy||0)+'<span>ENEMY</span></div></div>';
  html+='</div>';
  Object.keys(teams).forEach(tm=>{if(!teams[tm].length)return;html+='<div class="card team"><div class="teamhead"><span>'+(tm===b.selfTeam?'Your team':'Enemy team')+'</span><span>'+(b.teamStats?.[tm]?.avgRank||'')+'</span></div>';
   teams[tm].forEach(p=>{html+='<div class="row '+(p.isSelf?'self':'')+'">'+(p.agentPortrait?'<img class="agent" src="'+p.agentPortrait+'">':'<span></span>')+'<div class="name">'+esc(p.name)+(p.smurf?'<span class="smurf">SMURF</span>':'')+'</div><div class="rank" style="color:'+(p.rankColor||'#8b93a1')+'">'+(p.rank||'Unrated')+'</div><div class="kd">'+(p.kd??'-')+'</div></div>'});
   html+='</div>'});
 }
 document.getElementById('app').innerHTML=html;
 document.getElementById('queue').textContent=inQueue?'Cancel queue':'Start queue';
}
function esc(s){return String(s||'').replace(/[&<>"]/g,c=>({'&':'&amp;','<':'&lt;','>':'&gt;','"':'&quot;'}[c]))}
async function act(body){const r=await fetch('/api/action?t='+t,{method:'POST',headers:{'content-type':'application/json'},body:JSON.stringify(body)}).then(r=>r.json());show(r.message||'Done');load()}
function show(m){const el=document.getElementById('toast');el.textContent=m;el.style.display='block';clearTimeout(window.tt);window.tt=setTimeout(()=>el.style.display='none',3500)}
document.getElementById('queue').onclick=()=>act({action:inQueue?'stop':'start'});
document.getElementById('lock').onclick=()=>act({action:'instalock',agent:document.getElementById('agents').value,mode:'lock'});
fetch('/api/agents?t='+t).then(r=>r.json()).then(j=>{const s=document.getElementById('agents');s.innerHTML=(j.agents||[]).map(a=>'<option>'+a.name+'</option>').join('')}).catch(()=>{});
load();setInterval(load,3500);
</script></body></html>"##;
