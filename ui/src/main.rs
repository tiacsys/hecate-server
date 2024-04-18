use yew::prelude::*;
use gloo::timers::callback::Interval;
use gloo::net::http;
use anyhow::Result;
use huginn_protobuf as proto;
use charming::{
    component::{Axis, Title}, element::AxisType, series::Line, Chart, WasmRenderer
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

async fn request_reset_data() -> Result<()> {
    http::Request::post("/reset/data").send().await?;
    Ok(())
}

#[derive(Properties, PartialEq)]
struct PlotAttributes {
    plot_id: AttrValue,
    #[prop_or_default]
    title: String,
    xs: Vec<f64>,
    ys: Vec<f64>,
}

#[function_component(Plot)]
fn plot(PlotAttributes { plot_id, title, xs, ys }: &PlotAttributes) -> Html {

    let x_min = xs.iter().copied().max_by(|a, b| a.total_cmp(b)).unwrap_or(0.0);
    let x_max = xs.iter().copied().min_by(|a, b| a.total_cmp(b)).unwrap_or(1.0);
    let data = xs.into_iter().zip(ys.into_iter()).map(|(x, y)| vec![*x, *y]).collect();
    let id_clone = plot_id.clone();
    let title_clone = title.clone();
    yew::platform::spawn_local(async move {

        let chart = Chart::new()
            .title(Title::new().text(title_clone))
            .x_axis(Axis::new()
                .type_(AxisType::Value)
                .min(x_min)
                .max(x_max)
            )
            .y_axis(Axis::new()
                .type_(AxisType::Value)
            )
            .series(Line::new()
                .data(data)
            );
        WasmRenderer::new(400, 300)
            .render(&id_clone, &chart).expect(":(");
    });

    html! {
        <div id={plot_id}></div>
    }
}

#[derive(Properties, PartialEq)]
struct DataPlotsProps {
    data: proto::SensorData,
}

#[function_component(DataPlots)]
fn data_plots(DataPlotsProps { data }: &DataPlotsProps) -> Html {

    let times = data.samples.iter().map(|s| s.time as f64).collect::<Vec<_>>();
    let acc_x = data.samples.iter().map(|s| s.acceleration.x as f64).collect::<Vec<_>>();
    let acc_y = data.samples.iter().map(|s| s.acceleration.y as f64).collect::<Vec<_>>();
    let acc_z = data.samples.iter().map(|s| s.acceleration.z as f64).collect::<Vec<_>>();
    let mag_x = data.samples.iter().map(|s| s.magnetometer.x as f64).collect::<Vec<_>>();
    let mag_y = data.samples.iter().map(|s| s.magnetometer.y as f64).collect::<Vec<_>>();
    let mag_z = data.samples.iter().map(|s| s.magnetometer.z as f64).collect::<Vec<_>>();
    let gyro_x = data.samples.iter().map(|s| s.gyroscope.x as f64).collect::<Vec<_>>();
    let gyro_y = data.samples.iter().map(|s| s.gyroscope.y as f64).collect::<Vec<_>>();
    let gyro_z = data.samples.iter().map(|s| s.gyroscope.z as f64).collect::<Vec<_>>();

    html! {
        <>
            <h2>{ "Accelerometer" }</h2>
            <div>
                <div class="inline-block-child">
                    <Plot plot_id="acc_x" title="X-Axis" xs={times.clone()} ys={acc_x}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="acc_y" title="Y-Axis" xs={times.clone()} ys={acc_y}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="acc_zz" title="Z-Axis" xs={times.clone()} ys={acc_z}/>
                </div>
            </div>
            <h2>{ "Magnetometer" }</h2>
            <div>
                <div class="inline-block-child">
                    <Plot plot_id="mag_x" title="X-Axis" xs={times.clone()} ys={mag_x}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="mag_y" title="Y-Axis" xs={times.clone()} ys={mag_y}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="mag_z" title="Z-Axis" xs={times.clone()} ys={mag_z}/>
                </div>
            </div>
            <h2>{ "Gyroscope" }</h2>
            <div>
                <div class="inline-block-child">
                    <Plot plot_id="gyro_x" title="X-Axis" xs={times.clone()} ys={gyro_x}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="gyro_y" title="Y-Axis" xs={times.clone()} ys={gyro_y}/>
                </div>
                <div class="inline-block-child">
                    <Plot plot_id="gyro_z" title="Z-Axis" xs={times.clone()} ys={gyro_z}/>
                </div>
            </div>
        </>
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
        let updater = Interval::new(5000, move || link.send_message(AppMessage::UpdatePending));

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

        html! {
            <>
                <p>{ format!("Connected: {}", self.connected) }</p>
                <button onclick={Callback::from(|_| {
                    yew::platform::spawn_local(async move {
                        _ = request_reset_data().await;
                    });
                })}>
                    { "Reset Data" }
                </button>
                if let Some(data) = self.recent_data.clone() {
                    <DataPlots data={data} />
                }
            </>
        }
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
