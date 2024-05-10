use std::{
    collections::HashMap,
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

mod backend;
mod state;

type CmdState = Arc<Mutex<HashMap<u64, state::CommandInfo>>>;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));

    axum::serve(listener, app(cmd_state.clone())).await.unwrap();
}

fn app(cmd_state: CmdState) -> Router {
    Router::new()
        .route("/", get(pitch))
        .route("/api/submitcmd", post(submitcmd))
        .route("/api/getcmdstatus/:cmd_id", get(getcmdstatus))
        .with_state(cmd_state.clone())
}

// Serialize and Deserialize json payload
#[derive(serde::Deserialize, Debug)]
struct SubmitCmd {
    shell: String,
    cmd: String,
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
    let shell = req.shell;
    let cmd = req.cmd;
    match backend::init(shell, cmd, state).await {
        Ok(r) => r.to_string().into_response(),
        Err(err) => err.to_string().into_response(),
    }
}

async fn getcmdstatus(
    Path(GetCmdStatus { cmd_id }): Path<GetCmdStatus>,
    State(state): State<CmdState>,
) -> Json<CmdResponse> {
    let cmd_unwrap = state.lock().unwrap();
    match cmd_unwrap.get(&cmd_id) {
        Some(v) => {
            let mut c_res = CmdResponse::empty();
            c_res.state = v.state.clone();
            c_res.output = String::from_utf8(v.output.clone()).unwrap();
            Json(c_res)
        }
        None => Json(CmdResponse::empty()),
    }
}

async fn pitch() -> Response {
    "This is Runit application".into_response()
}
