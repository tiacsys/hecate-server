mod fetch;

use fetch::Fetch;
use gloo::net::http;
use polars::prelude::*;
use yew::prelude::*;
use yew_hooks::prelude::*;
use uuid::Uuid;
use charming::{component::{Axis, Title}, element::AxisType, series::Line, Chart, WasmRenderer};

#[function_component(ConnectionIndicator)]
fn connection_indicator() -> Html {

    let status = use_state(|| false );
    {
        let status = status.clone();
        use_interval(move || {
            let status = status.clone();
            yew::platform::spawn_local(async move {
                let new_status = bool::fetch("/connected").await.unwrap_or(false);
                status.set(new_status);
            });

        }, 1000);
    }

    html! {
        <p>{ format!("Connected: {}", *status) }</p>
    }
}

#[derive(Debug, PartialEq, Properties)]
struct DataFrameTableProps {
    frame: DataFrame,
}

#[function_component(DataFrameTable)]
fn dataframe_table(DataFrameTableProps { frame }: &DataFrameTableProps) -> Html {

    html! {
        <p style="white-space: pre; font-family: monospace"> { format!("{frame}") }</p>
    }
}

#[derive(Debug, Clone, PartialEq)]
struct PlotData {
    pub name: String,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
}

impl PlotData {
    pub fn over_time(frame: &DataFrame, time_str: &str, value_str: &str, name: &str) -> Option<Self> {
        let xs = frame.column(time_str).ok()
            .and_then(|s| s.duration().ok().cloned())
            .map(|x| x.nanoseconds())
            .map(|s| s.into_iter().collect::<Vec<_>>())
            .map(|s| s.into_iter().map(|x| x.map(|x| (x as f64) * 1.0e-9)).collect::<Vec<_>>())?;

        let ys = frame.column(value_str).ok()
            .and_then(|s| s.cast(&DataType::Float64).ok())
            .and_then(|s| s.f64().ok().cloned())
            .map(|s| s.into_iter().collect::<Vec<_>>())?;

        let (xs, ys): (Vec<_>, Vec<_>) = xs.into_iter().zip(ys)
            .map(|(x, y)| match (x, y) {
                (Some(x), Some(y)) => Some((x, y)),
                _ => None,
            })
            .filter_map(|xy| xy)
            .unzip();

        Some(Self {
            xs,
            ys,
            name: name.into(),
        })
    }
}

#[derive(Debug, Properties, PartialEq)]
struct PlotProps {
    #[prop_or_default]
    data: Option<PlotData>,
}

#[function_component(Plot)]
fn plot(PlotProps { data }: &PlotProps) -> Html {

    let zipped_data = data.clone()
        .map(|d| d.xs.iter().zip(d.ys.iter()).map(|(x, y)| vec![*x, *y]).collect::<Vec<_>>())
        .unwrap_or(vec![Vec::new()]);

    let x_min = data.clone().and_then(|d|
            d.xs.iter().copied().min_by(|a, b| a.total_cmp(b))
        ).unwrap_or(0.0);

    let x_max = data.clone().and_then(|d|
            d.xs.iter().copied().max_by(|a, b| a.total_cmp(b))
        ).unwrap_or(1.0);

    let id = Uuid::new_v4();

    let plot_name = data.clone().map(|d| d.name).unwrap_or(String::new());

    yew::platform::spawn_local(async move {
        let chart = Chart::new()
            .title(Title::new().text(plot_name))
            .x_axis(Axis::new()
                .type_(AxisType::Value)
                .min(x_min)
                .max(x_max)
            )
            .y_axis(Axis::new()
                .type_(AxisType::Value)
            )
            .series(Line::new().data(zipped_data));
        WasmRenderer::new(400, 300)
            .render(&id.to_string(), &chart)
            .unwrap();
    });

    html! {
        <div id={id.to_string()}></div>
    }
}

fn downsampled_data(frame: &DataFrame, time_str: &str, sample_len: polars::time::Duration) -> Option<DataFrame> {

    frame.clone().lazy()
        .with_column((col(time_str) + lit(chrono::NaiveDate::from_isoywd_opt(0, 1, chrono::Weekday::Mon).unwrap())).alias("time_abs"))
        .sort(["time_abs"], Default::default())
        .group_by_dynamic(
            col("time_abs"),
            [],
            DynamicGroupOptions {
                every: sample_len,
                period: sample_len,
                offset: Duration::parse("0"),
                ..Default::default()
            }
        )
        .agg([col("*").mean()])
        .select([col("*").exclude(["time_abs"])])
        .collect().ok()
}

#[derive(Debug, Properties, PartialEq)]
struct DataViewProps {
    device_id: UseStateHandle<String>
}

#[function_component(DataView)]
fn data_view(DataViewProps { device_id }: &DataViewProps) -> Html {

    if (*device_id.clone()) == "" {
        return html! {
            <></>
        };
    }

    let data = use_state(|| DataFrame::empty());
    {
        let data = data.clone();
        let device_id = device_id.clone();
        use_interval(move || {
            let data = data.clone();
            let device_id = device_id.clone();
            yew::platform::spawn_local(async move {
                if let Ok(new_data) = DataFrame::fetch(&format!("/sensor/{}/data", *device_id)).await {
                    data.set(new_data);
                }
            });
        }, 1000);
    }
    
    // Downsample by 10x
    let downsampled = downsampled_data(&data, "time", Duration::parse("200ms")).unwrap_or(DataFrame::empty());

    let acc_x = PlotData::over_time(&downsampled, "time", "acc_x", "Acc X");
    let acc_y = PlotData::over_time(&downsampled, "time", "acc_y", "Acc Y");
    let acc_z = PlotData::over_time(&downsampled, "time", "acc_z", "Acc Z");
    let mag_x = PlotData::over_time(&downsampled, "time", "mag_x", "Mag X");
    let mag_y = PlotData::over_time(&downsampled, "time", "mag_y", "Mag Y");
    let mag_z = PlotData::over_time(&downsampled, "time", "mag_z", "Mag Z");
    let gyro_x = PlotData::over_time(&downsampled, "time", "gyro_x", "Gyro X");
    let gyro_y = PlotData::over_time(&downsampled, "time", "gyro_y", "Gyro Y");
    let gyro_z = PlotData::over_time(&downsampled, "time", "gyro_z", "Gyro Z");

    html! {
        <>
            <div>
                <h2 style="display: inline-block">{ format!("Device: {}", **device_id) }</h2>
                <button style="display: inline-block; margin-left: 10px;" onclick={
                    let device_id = device_id.clone();
                    Callback::from(move |_| {
                        let device_id = device_id.clone();
                        yew::platform::spawn_local(async move {
                            _ = http::Request::post(&format!("/sensor/{}/data/reset", *device_id)).send().await;
                        });
                    })
                }>
                    { "Reset" }
                </button>
            </div>
            <table>
                <tr>
                    <td><Plot data={acc_x}/></td>
                    <td><Plot data={acc_y}/></td>
                    <td><Plot data={acc_z}/></td>
                </tr>
                <tr>
                    <td><Plot data={mag_x}/></td>
                    <td><Plot data={mag_y}/></td>
                    <td><Plot data={mag_z}/></td>

                </tr>
                <tr>
                    <td><Plot data={gyro_x}/></td>
                    <td><Plot data={gyro_y}/></td>
                    <td><Plot data={gyro_z}/></td>

                </tr>
            </table>
            { "Raw data:" }
            <DataFrameTable frame={downsampled.clone()}/>
            // <DataFrameTable frame={(*data).clone()} />
        </>
    }
}

#[derive(Debug, Properties, PartialEq)]
struct ConnectedDevicesListProps {
    selected_id: UseStateHandle<String>,
}

#[function_component(ConnectedDevicesList)]
fn connected_devices_list(ConnectedDevicesListProps { selected_id }: &ConnectedDevicesListProps) -> Html {

    let ids = use_state(|| Vec::<String>::new());
    {
        let ids = ids.clone();
        use_interval(move || {
            let ids = ids.clone();
            yew::platform::spawn_local(async move {
                if let Ok(received_ids) = Vec::<String>::fetch("/sensor/connections").await {
                    ids.set(received_ids);
                }
            });
        }, 1000);
    }

    html! {
        <div class="device-list">
            <h2>{ "Connected Devices" }</h2>
            <table>
            { 
                for (*ids).iter().map(|id| {
                    let selected_id = selected_id.clone();
                    let id_clone = id.clone();
                    html! {
                        <tr><td onclick={Callback::from(move |_| selected_id.set(id_clone.clone()))}> {id} </td></tr>
                    }
                })
            }
            </table>
        </div>
    }
}

#[function_component(App)]
fn app() -> Html {

    let selected_id = use_state(|| String::new());

    html! {
        <>
            <ConnectedDevicesList selected_id={selected_id.clone()} />
            <div class="main">
                <DataView device_id={selected_id.clone()}/>
            </div>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
