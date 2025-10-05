use std::{
    collections::HashMap,
    env,
    net::{Ipv4Addr, SocketAddrV4},
    sync::{Arc, Mutex},
};

use axum::{
    async_trait,
    extract::{FromRequest, Path, Request, State},
    http::{header::CONTENT_TYPE, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, RequestExt, Router,
};
use reqwest::Client;
use serde::Serialize;
use tracing::{self, info, warn};
use tracing_subscriber::prelude::*;
use tracing_subscriber::{fmt, EnvFilter};

mod backend;
mod state;

type CmdState = Arc<Mutex<HashMap<u64, state::CommandInfo>>>;

#[tokio::main]
async fn main() {
    // init fmt tracing layer
    let fmt_layer = fmt::layer().with_target(false);
    let filter_layer = EnvFilter::try_from_default_env()
        .or_else(|_| EnvFilter::try_new("info"))
        .unwrap();
    tracing_subscriber::registry()
        .with(filter_layer)
        .with(fmt_layer)
        .init();

    // port represent app name but in reverse: ITRUN on keypad
    let port: u16 = match env::var("RUN_IT_PORT") {
        Ok(c) => c.parse().unwrap_or(48786),
        Err(_) => 48786,
    };
    let addr = SocketAddrV4::new(Ipv4Addr::new(0, 0, 0, 0), port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    info!("Listening on {}", port);
    axum::serve(listener, app()).await.unwrap();
}

fn app() -> Router {
    let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));
    Router::new()
        .route("/", get(pitch))
        .route("/api/submitcmd", post(submitcmd))
        .route("/api/nl2cmd", post(nl2cmd))
        .route("/api/getcmdstatus/:cmd_id", get(getcmdstatus))
        .route("/api/submitfile", post(submitfile))
        .with_state(cmd_state.clone())
}

// Serialize and Deserialize json payload
#[derive(serde::Deserialize, Debug)]
struct SubmitCmd {
    cmd: String,
    args: Option<String>,
    is_shell: Option<bool>,
}

#[derive(serde::Deserialize)]
struct GetCmdStatus {
    cmd_id: u64,
}

#[derive(serde::Deserialize, Debug)]
struct Nl2Cmd {
    nl2cmd: String,
}

#[derive(serde::Serialize, Debug)]
struct Nl2CmdRequest {
    data: Nl2CmdData,
}

#[derive(serde::Serialize, Debug)]
struct Nl2CmdData {
    nl2cmd: String,
}

#[derive(serde::Deserialize, Debug)]
struct Nl2CmdResponse {
    result: Nl2CmdResult,
}

#[derive(serde::Deserialize, Debug)]
struct Nl2CmdResult {
    cmd: String,
    runnable: bool,
}

#[derive(Serialize)]
struct CmdResponse {
    state: String,
    output: String,
}
impl CmdResponse {
    fn empty() -> Self {
        let state = String::new();
        let output = String::new();
        Self { state, output }
    }
}
// Struct to meet `post` requirements and
// expecting json payload
struct JsonPayload<T>(T);

// axum supports this out-of-the box with a module called, `extract` and
// with struct `Json`, but this is learning project as well hence
// took the below approach.
#[async_trait]
impl<S, T> FromRequest<S> for JsonPayload<T>
where
    S: Send + Sync,
    Json<T>: FromRequest<()>,
    T: 'static,
{
    type Rejection = Response;
    async fn from_request(req: Request, _state: &S) -> Result<Self, Self::Rejection> {
        let content_type_header = req.headers().get(CONTENT_TYPE);
        let content_type = content_type_header.and_then(|value| value.to_str().ok());

        if let Some(content_type) = content_type {
            if content_type.starts_with("application/json") {
                let Json(payload) = req.extract().await.map_err(IntoResponse::into_response)?;
                return Ok(Self(payload));
            }
        }

        Err(StatusCode::UNSUPPORTED_MEDIA_TYPE.into_response())
    }
}

async fn submitcmd(
    State(state): State<CmdState>,
    JsonPayload(req): JsonPayload<SubmitCmd>, // order of extractors matter: https://docs.rs/axum/latest/axum/extract/index.html#the-order-of-extractors
) -> Response {
    let cmd = req.cmd;
    let args = req.args;
    let is_shell = req.is_shell.unwrap_or(false);
    info!(method = "POST", cmd, args, is_shell);
    match backend::init(cmd, args, is_shell, state).await {
        Ok(r) => r.to_string().into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn nl2cmd(State(state): State<CmdState>, JsonPayload(req): JsonPayload<Nl2Cmd>) -> Response {
    let nl2cmd_txt = req.nl2cmd;
    let ai_url =
        env::var("AI_URL").unwrap_or_else(|_| "http://localhost:3400/nl2CmdFlow".to_string());
    info!(method = "POST", nl2cmd_txt, ai_url);

    // AI request data
    let nl2cmd_req = Nl2CmdRequest {
        data: Nl2CmdData { nl2cmd: nl2cmd_txt },
    };
    // Make AI request
    let client = Client::new();
    match client
        .post(&ai_url)
        .header("Content-Type", "application/json")
        .json(&nl2cmd_req)
        .send()
        .await
    {
        Ok(res) => {
            match res.json::<Nl2CmdResponse>().await {
                Ok(ai_res) => {
                    info!("AI Response: {:?}", ai_res);
                    if ai_res.result.runnable {
                        let res_cmd = ai_res.result.cmd.clone();
                        let cmd_parts: Vec<&str> = res_cmd.split_whitespace().collect();
                        if cmd_parts.is_empty() {
                            return (StatusCode::BAD_REQUEST, "Empty command").into_response();
                        }

                        let cmd = cmd_parts[0].to_string();
                        let args = if cmd_parts.len() > 1 {
                            Some(cmd_parts[1..].join(" "))
                        } else {
                            None
                        };
                        info!("Executing - cmd: {}, args: {:?}", cmd, args);
                        match backend::init(cmd, args, false, state).await {
                            Ok(r) => r.to_string().into_response(),
                            Err(err) => err.to_string().into_response(),
                        }
                    } else {
                        // Command is not runnable, return the message
                        (StatusCode::BAD_REQUEST, ai_res.result.cmd).into_response()
                    }
                }
                Err(e) => {
                    warn!("Failed to parse AI response: {}", e);
                    (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        format!("Failed to parse AI response: {}", e),
                    )
                        .into_response()
                }
            }
        }
        Err(e) => {
            warn!("Failed to call AI endpoint: {}", e);
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Failed to call AI endpoint: {}", e),
            )
                .into_response()
        }
    }
}

async fn getcmdstatus(
    Path(GetCmdStatus { cmd_id }): Path<GetCmdStatus>,
    State(state): State<CmdState>,
) -> Json<CmdResponse> {
    let cmd_unwrap = state.lock().unwrap();
    info!(method = "GET", cmd_id);
    match cmd_unwrap.get(&cmd_id) {
        Some(v) => {
            let mut c_res = CmdResponse::empty();
            c_res.state = v.state.clone();
            c_res.output = String::from_utf8(v.output.clone()).unwrap();
            Json(c_res)
        }
        None => {
            warn!("Not found: {}", cmd_id);
            Json(CmdResponse::empty())
        }
    }
}

async fn submitfile(State(state): State<CmdState>) -> Response {
    todo!()
}

async fn pitch() -> Response {
    "This is Runit application".into_response()
}
