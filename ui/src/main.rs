mod fetch;

use fetch::Fetch;
use polars::prelude::*;
use yew::prelude::*;
use yew_hooks::prelude::*;


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

#[derive(Debug, Properties, PartialEq)]
struct DataViewProps {
    device_id: UseStateHandle<String>
}

#[function_component(DataView)]
fn data_view(DataViewProps { device_id }: &DataViewProps) -> Html {

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
    
    html! {
        <>
            <h2>{ format!("Device: {}", **device_id) }</h2>
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
        <>
        <h2>{ "Connected Devices" }</h2>
        <table style="tr:hover {background-color: gray;}">
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
        </>
    }
}

#[function_component(App)]
fn app() -> Html {

    let selected_id = use_state(|| String::new());

    html! {
        <>
            <ConnectedDevicesList selected_id={selected_id.clone()} />
            <DataView device_id={selected_id.clone()}/>
        </>
    }
}

fn main() {
    yew::Renderer::<App>::new().render();
}
