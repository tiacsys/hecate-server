use yew::prelude::*;
use gloo::timers::callback::Interval;
use gloo::net::http;
use ::common::DataPoint;
use anyhow::Result;

async fn fetch_connection_status() -> Result<bool> {
    let response = http::Request::get("/connected").send().await?;
    let parsed = response.json().await?;
    Ok(parsed)
}

async fn fetch_recent_data() -> Result<String> {
    let response = http::Request::get("/data").send().await?;
    let parsed = response.json().await?;
    Ok(parsed)
}

struct App {
    connected: bool,
    recent_data: String,
    _updater: Interval,
}

enum AppMessage {
    UpdatePending,
    UpdateFailed,
    UpdateConnectionStatus(bool),
    UpdateRecentData(String),
}

impl Component for App {
    type Properties = ();
    type Message = AppMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let updater = Interval::new(1000, move || link.send_message(AppMessage::UpdatePending));

        App {
            connected: false,
            recent_data: "".into(),
            _updater: updater,
        }
    }

    fn update(&mut self, ctx: &Context<Self>, msg: Self::Message) -> bool {
        match msg {
            AppMessage::UpdatePending => {
                ctx.link().send_future(async {
                    match fetch_connection_status().await {
                        Ok(connected) => AppMessage::UpdateConnectionStatus(connected),
                        Err(_) => AppMessage::UpdateFailed,
                    }
                });
                ctx.link().send_future(async {
                    match fetch_recent_data().await {
                        Ok(data) => AppMessage::UpdateRecentData(data),
                        Err(_) => AppMessage::UpdateFailed,
                    }
                });
                false
            },
            AppMessage::UpdateConnectionStatus(connected) => {
                self.connected = connected;
                true
            },
            AppMessage::UpdateRecentData(data) => {
                self.recent_data = data;
                true
            }
            AppMessage::UpdateFailed => false,
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {

        html! {
            <>
                <p>{ format!("Connected: {}", self.connected) }</p>
                <p>{ format!("Data: {}", self.recent_data) }</p>
            </>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
