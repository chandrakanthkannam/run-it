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
