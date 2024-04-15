use yew::prelude::*;
use gloo::timers::callback::Interval;
use gloo::net::http;
use anyhow::Result;
use huginn_protobuf as proto;
use charming::{
    component::Axis, element::AxisType, series::Line, Chart, WasmRenderer
};


async fn fetch_connection_status() -> Result<bool> {
    let response = http::Request::get("/connected").send().await?;
    let parsed = response.json().await?;
    Ok(parsed)
}

async fn fetch_recent_data() -> Result<proto::SensorData> {
    let response = http::Request::get("/data").send().await?;
    let parsed = response.json().await?;
    Ok(parsed)
}

fn plot(xs: &[f64], ys: &[f64]) -> Html {

    let data = xs.into_iter().zip(ys.into_iter()).map(|(x, y)| vec![*x, *y]).collect();
    yew::platform::spawn_local(async move {

        let chart = Chart::new()
            .x_axis(Axis::new()
                .type_(AxisType::Value)
            )
            .y_axis(Axis::new()
                .type_(AxisType::Value)
            )
            .series(Line::new()
                .data(data)
            );
        WasmRenderer::new(1000, 800)
            .render("chart", &chart).expect(":(");
    });

    html! {
        <div id="chart"></div>
    }
}
struct App {
    connected: bool,
    recent_data: Option<proto::SensorData>,
    _updater: Interval,
}

enum AppMessage {
    UpdatePending,
    UpdateFailed,
    UpdateConnectionStatus(bool),
    UpdateRecentData(proto::SensorData),
}

impl Component for App {
    type Properties = ();
    type Message = AppMessage;

    fn create(ctx: &Context<Self>) -> Self {
        let link = ctx.link().clone();
        let updater = Interval::new(1000, move || link.send_message(AppMessage::UpdatePending));

        App {
            connected: false,
            recent_data: None,
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
                self.recent_data = Some(data);
                true
            }
            AppMessage::UpdateFailed => false,
        }
    }

    fn view(&self, _ctx: &Context<Self>) -> Html {

        let (times, acc_x) = if let Some(data) = &self.recent_data {
            let times = data.samples.iter().map(|s| s.time as f64).collect();
            let acc_x = data.samples.iter().map(|s| s.acceleration.x as f64).collect();
            (times, acc_x)
        } else {
            (Vec::new(), Vec::new())
        };

        html! {
            <>
                <p>{ format!("Connected: {}", self.connected) }</p>
                { plot(&times, &acc_x) }
            </>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
