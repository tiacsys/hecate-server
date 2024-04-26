mod connection;
mod frame;

use connection::Connection;
use frame::Frame;

use bytes::Bytes;
use hecate_protobuf as proto;
use polars::prelude::*;
use proto::Message;
use rocket::{
    futures::lock::{Mutex, MutexGuard},
    fs::NamedFile,
    get,
    launch,
    response::status::NotFound,
    routes,
    serde::json::Json,
    State,
    tokio::time::timeout,
};
use rocket_ws as ws;
use std::collections::HashMap;
use std::path::PathBuf;


struct Connections {
    connections: Arc<Mutex<HashMap<String, Connection>>>,
}

impl Connections {
    pub fn new() -> Self {
        Self { connections: Arc::new(Mutex::new(HashMap::new())) }
    }

    pub fn get<'a>(lock: &'a mut MutexGuard<'_, HashMap<String, Connection>>, id: &str) -> Option<&'a mut Connection> {
        (*lock).get_mut(id)
    }
}

#[launch]
fn rocket() -> _ {
    rocket::build()
        .manage(Connections::new())
        .mount("/", routes![
            index,
            static_files,
            connections,
            sensor_connected,
            sensor_data,
            ws_data,
        ])
}

#[get("/")]
async fn index() -> Result<NamedFile, NotFound<String>> {
    NamedFile::open("../ui/dist/index.html").await
        .map_err(|e| NotFound(e.to_string()))
}

#[get("/<path..>")]
async fn static_files(path: PathBuf) -> Result<NamedFile, NotFound<String>> {
    let path = PathBuf::from("../ui/dist").join(path);
    NamedFile::open(path).await
        .or_else(|e| Err(NotFound(e.to_string())))
}

#[get("/sensor/connections")]
async fn connections(state: &State<Connections>) -> Json<Vec<String>> {
    let lock = state.connections.lock().await;
    let keys = lock.keys().cloned().collect();
    Json(keys)
}

#[get("/sensor/<id>/connected")]
async fn sensor_connected(id: &str, state: &State<Connections>) -> Option<Json<bool>> {
    let mut lock = state.connections.lock().await;
    Connections::get(&mut lock, id)
        .and_then(|c| Some(Json(c.active)))
}

#[get("/sensor/<id>/data")]
async fn sensor_data(id: &str, state: &State<Connections>) -> Option<Json<DataFrame>> {
    let mut lock = state.connections.lock().await;
    Connections::get(&mut lock, id)
        .and_then(|c| Some(Json(c.recent_data().clone())))
}

#[get("/ws")]
async fn ws_data<'r>(ws: ws::WebSocket, state: &'r State<Connections>) -> ws::Channel<'r> {

    use rocket::futures::{SinkExt, StreamExt};

    ws.channel(move |mut stream| Box::pin(async move {

        // First thing a sensor must send is its ID as text
        let id = match stream.next().await {
            Some(Ok(ws::Message::Text(id))) => {
                id
            },
            _ => {
                stream.send(ws::Message::Close(None)).await?;
                return Ok(())
            },
        };

        // Register the connection as active
        {
            let mut lock = state.connections.lock().await;
            if let Some(connection) = lock.get_mut(&id) {
                connection.active = true;
            } else {
                let mut new_connection = Connection::new(&id);
                new_connection.active = true;
                lock.insert(id.clone(), new_connection);
            }
        }

        // Process data as it comes in. On timeout send a courtesy close, then
        // drop the connection.
        loop {
            match timeout(std::time::Duration::from_secs(10), stream.next()).await {
                Err(_) => {
                    // This means we timed out
                    stream.send(ws::Message::Close(None)).await?;
                    break;
                },
                Ok(None) => {
                    // This means the stream iterator has ended, which shouldn't
                    // actually happen
                    stream.send(ws::Message::Close(None)).await?;
                    break;
                },
                Ok(Some(message)) => {
                    match message {
                        // Stop processing on receiving a close frame
                        Ok(ws::Message::Close(_)) => {
                            break;
                        },
                        // Decode received data and add it to the dataframe for
                        // this connection
                        Ok(ws::Message::Binary(data)) => {
                            if let Some(frame) = proto::SensorData::decode(Bytes::from(data))
                                .ok().and_then(|d| d.frame().ok()) {
                                
                                let mut lock = state.connections.lock().await;
                                if let Some(connection) = Connections::get(&mut lock, &id) {
                                    _ = connection.append_data(frame)
                                        .and_then(|_| connection.discard_older_than(chrono::Duration::seconds(60)));
                                }
                            }
                        },
                        _=> {},
                    }
                },
            };
        }

        {
            let mut lock = state.connections.lock().await;
            if let Some(connection) = Connections::get(&mut lock, &id) {
                connection.active = false;
            }
        }

        Ok(())
    }))
}
