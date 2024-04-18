use std::sync::Arc;
use std::path::PathBuf;
use bytes::Bytes;
use rocket::futures::lock::{self, Mutex};
use rocket::{
    fs::NamedFile,
    get,
    launch,
    response::status::{
        NotFound,
    },
    routes,
    serde::json::Json,
    State
};
use rocket_ws as ws;
use rocket_ws::WebSocket;
use hecate_protobuf as proto;
use proto::{Acceleration, Message};

struct AppState {
    connected: Arc<Mutex<bool>>,
    recent_data: Arc<Mutex<Option<proto::Acceleration>>>,
}

impl AppState {
    pub fn new() -> Self {
        Self {
            connected: Arc::new(Mutex::new(false)),
            recent_data: Arc::new(Mutex::new(None)),
        }
    }

    pub async fn set_connected(&self, connected: bool) {
        let mut locked = self.connected.lock().await;
        *locked = connected;
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(AppState::new())
        .mount("/", routes![index, static_files, data, connected, ws_data])
}

async fn get_index() -> Result<NamedFile, NotFound<String>> {
    NamedFile::open("../ui/dist/index.html").await
        .map_err(|e| NotFound(e.to_string()))
}

#[get("/")]
async fn index() -> Result<NamedFile, NotFound<String>> {
    get_index().await
}

#[get("/<path..>")]
async fn static_files(path: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = PathBuf::from("../ui/dist").join(path);
    NamedFile::open(path).await
        .or_else(|e| Err(NotFound(e.to_string())))
}

#[get("/data")]
async fn data(state: &State<AppState>) -> Json<Acceleration> {
    let locked = state.recent_data.lock().await;
    if let Some(value) = locked.clone() {
        Json(value)
    } else {
        Json(Acceleration { x: 0.0, y: 0.0, z: 0.0 })
    }
}

#[get("/connected")]
async fn connected(state: &State<AppState>) -> Json<bool> {
    let locked = state.connected.lock().await;
    Json(*locked)
}

#[get("/ws")]
async fn ws_data<'r>(ws: WebSocket, state: &'r State<AppState>) -> ws::Channel<'r> {

    use rocket::futures::{SinkExt, StreamExt};

    ws.channel(move |mut stream| Box::pin(async move {
        state.set_connected(true).await;
        while let Some (message) = stream.next().await {
            match message {
                Ok(ws::Message::Text(_)) => {
                    stream.send(message?).await?;
                },
                Ok(ws::Message::Close(_)) => {
                    state.set_connected(false).await;
                },
                Ok(ws::Message::Binary(data)) => {
                    if let Ok(decoded) = Acceleration::decode(Bytes::from(data)) {
                        let mut locked = state.recent_data.lock().await;
                        *locked = Some(decoded);
                    }
                }
                Err(_) => {
                    state.set_connected(false).await;
                }
                _=> {},
            }
        }

        Ok(())
    }))
}
