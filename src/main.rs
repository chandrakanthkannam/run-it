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
    let addr = SocketAddrV4::new(Ipv4Addr::new(127, 0, 0, 1), port);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));
    info!("Listening on {}", addr);
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

async fn pitch() -> Response {
    "This is Runit application".into_response()
}

#[cfg(test)]
mod tests {
    use crate::app;
    use crate::CmdState;
    use axum::body;
    use axum::body::Body;
    use axum::http::{Request, StatusCode};
    use serde_json::json;
    use std::{
        collections::HashMap,
        sync::{Arc, Mutex},
    };
    use tower::{Service, ServiceExt};

    #[tokio::test]
    async fn echo() {
        let r_body = json!(
            {
                "cmd": "echo",
                "args": "Hello from Run-It"
            }
        );

        let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));
        let mut app = app(cmd_state).into_service();

        // Submit cmd
        let sub_cmd = Request::builder()
            .uri("/api/submitcmd")
            .header("Content-Type", "application/json")
            .method("POST")
            .body(Body::from(serde_json::to_string(&r_body).unwrap()))
            .unwrap();
        let c_req = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(sub_cmd)
            .await
            .unwrap();
        assert_eq!(c_req.status(), StatusCode::OK);

        let c_id = body::to_bytes(c_req.into_body(), usize::MAX).await.unwrap();

        // Get cmd status
        let cmd_id = Request::builder()
            .uri(format!(
                "/api/getcmdstatus/{}",
                String::from_utf8(c_id.to_vec()).unwrap()
            ))
            .method("GET")
            .body(Body::empty())
            .unwrap();
        let c_res = ServiceExt::<Request<Body>>::ready(&mut app)
            .await
            .unwrap()
            .call(cmd_id)
            .await
            .unwrap();
        assert_eq!(c_res.status(), StatusCode::OK);

        let c_output = body::to_bytes(c_res.into_body(), usize::MAX).await.unwrap();
        println!("{}", String::from_utf8(c_output.to_vec()).unwrap());
    }
}
