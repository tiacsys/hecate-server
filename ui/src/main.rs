mod fetch;

use fetch::Fetch;
use gloo::net::http;
use polars::prelude::*;
use yew::prelude::*;
use yew_hooks::prelude::*;
use uuid::Uuid;
use charming::{component::{Axis, Title}, element::AxisType, series::Line, Chart, WasmRenderer};
use wasm_bindgen::JsCast;
use web_sys::HtmlInputElement;

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
    let data_duration = use_state(|| String::from("1m"));
    let sampling_interval = use_state(|| String::from("500ms"));

    {
        let data = data.clone();
        let sampling_interval = sampling_interval.clone();
        let data_duration = data_duration.clone();
        let device_id = device_id.clone();
        use_interval(move || {
            let data = data.clone();
            let sampling_interval = sampling_interval.clone();
            let data_duration = data_duration.clone();
            let device_id = device_id.clone();
            yew::platform::spawn_local(async move {
                if let Ok(new_data) = DataFrame::fetch(&format!("/sensor/{}/data?interval={}&duration={}", *device_id, *sampling_interval, *data_duration)).await {
                    data.set(new_data);
                }
            });
        }, 1000);
    }
    
    let reset_button_onclick = {
        let device_id = device_id.clone();
        Callback::from(move |_| {
            let device_id = device_id.clone();
            yew::platform::spawn_local(async move {
                _ = http::Request::post(&format!("/sensor/{}/data/reset", *device_id)).send().await;
            });
        })
    };

    let data_duration_onchange = {
        let data_duration = data_duration.clone();
        Callback::from(move |e: Event| {
            e.target()
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                .map(|i| data_duration.set(i.value()));
        })
    };

    let sampling_interval_onchange = {
        let sampling_interval = sampling_interval.clone();
        Callback::from(move |e: Event| {
            e.target()
                .and_then(|t| t.dyn_into::<HtmlInputElement>().ok())
                .map(|i| sampling_interval.set(i.value()));
        })
    };

    let acc_x = PlotData::over_time(&data, "time", "acc_x", "Acc X");
    let acc_y = PlotData::over_time(&data, "time", "acc_y", "Acc Y");
    let acc_z = PlotData::over_time(&data, "time", "acc_z", "Acc Z");
    let mag_x = PlotData::over_time(&data, "time", "mag_x", "Mag X");
    let mag_y = PlotData::over_time(&data, "time", "mag_y", "Mag Y");
    let mag_z = PlotData::over_time(&data, "time", "mag_z", "Mag Z");
    let gyro_x = PlotData::over_time(&data, "time", "gyro_x", "Gyro X");
    let gyro_y = PlotData::over_time(&data, "time", "gyro_y", "Gyro Y");
    let gyro_z = PlotData::over_time(&data, "time", "gyro_z", "Gyro Z");

    html! {
        <>
            <h2>{ format!("Device: {}", **device_id) }</h2>
            <div class="data-view-settings">
                <span>{ "Duration:" }</span>
                <input style="width: 7ch;" onchange={data_duration_onchange} placeholder={format!("{}", *data_duration)}/>
                <span>{ "Sampling interval:" }</span>
                <input style="width: 7ch;" onchange={sampling_interval_onchange} placeholder={format!("{}", *sampling_interval)}/>
                <button onclick={reset_button_onclick}>{ "Reset Data" }</button>
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
            <DataFrameTable frame={(*data).clone()} />
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
