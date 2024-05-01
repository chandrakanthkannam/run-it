use std::{
    collections::HashMap,
    io::{self, Read},
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

mod backend;
mod state;

type CmdState = Arc<Mutex<HashMap<u64, state::CommandInfo>>>;

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));

    axum::serve(listener, app(cmd_state.clone())).await.unwrap();

    // This is for terminal code
    terminal_f();
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
    backend::init(shell, cmd, state)
        .await
        .to_string()
        .into_response()
}

async fn getcmdstatus(
    Path(GetCmdStatus { cmd_id }): Path<GetCmdStatus>,
    State(state): State<CmdState>,
) -> Response {
    let cmd_unwrap = state.lock().unwrap();
    let cmd_status = cmd_unwrap.get(&cmd_id).unwrap();
    String::from_utf8(cmd_status.output.clone())
        .unwrap()
        .into_response()
}

async fn pitch() -> Response {
    "This is Runit application".into_response()
}

async fn terminal_f() {
    let cmd_state: CmdState = Arc::new(Mutex::new(HashMap::new()));
    loop {
        let mut shell = String::new();
        let mut script = String::new();
        let cmd_state = cmd_state.clone();

        println!("Choose a Shell: bash or powershell");
        match io::stdin().read_line(&mut shell) {
            Ok(_) => {}
            Err(_) => {
                println!("Something went wrong reading input, lets try again..");
                continue;
            }
        };
        println!("Enter the command to run");
        match io::stdin().read_line(&mut script) {
            Ok(_) => {}
            Err(_) => {
                println!("Something went wrong reading input, lets try again..");
                continue;
            }
        };

        tokio::spawn(async move {
            backend::init(
                shell.trim().to_string(),
                script.trim().to_string(),
                cmd_state,
            )
            .await;
        });
    }
}
