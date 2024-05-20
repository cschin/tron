#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;
use bytes::{BufMut, Bytes, BytesMut};

use axum::{extract::Json, http::HeaderMap, response::Html};
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_app::{
    send_sse_msg_to_client,
    tron_components::{
        self, button, d3_plot::SseD3PlotTriggerMsg, TnActionExecutionMethod, TnAsset, TnD3Plot,
        TnHtmlResponse,
    },
    TnServerSideTriggerData,
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnEventActions, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::{HashMap, VecDeque}, pin::Pin, sync::Arc, task::Context, vec};

static D3PLOT: &str = "d3_plot";
static BUTTON: &str = "button";

// This is the main entry point of the application
// It sets up the application configuration and state
// and then starts the application by calling tron_app::run
#[tokio::main]
async fn main() {
    let app_config = tron_app::AppConfigure {
        http_only: true,
        ..Default::default()
    };
    // set app state
    let app_share_data = tron_app::AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_context)),
        build_actions: Arc::new(Box::new(build_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, app_config).await
}

// These functions are used to build the application context,
// layout, and event actions respectively
fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    let mut component_index = 0;
   
    let d3_plot_script = include_str!("../templates/d3_plot_script.html").to_string();
    let d3_plot = TnD3Plot::new(component_index, D3PLOT.into(), d3_plot_script);
    context.add_component(d3_plot);

    component_index += 1;
    let mut btn = TnButton::new(component_index, BUTTON.into(), "click me".into());
    btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary flex-1".to_string(),
    );

    btn.set_attribute("hx-target".to_string(), format!("#{D3PLOT}"));
    btn.set_attribute("hx-swap".to_string(), "none".to_string());
    context.add_component(btn);

    
    {
        let mut stream_data_guard = context.stream_data.blocking_write();
        stream_data_guard.insert("plot_data".into(), ("application/text".into(), VecDeque::default()));
        let mut data = VecDeque::default();
        let raw_data = include_str!("../templates/2_TwoNum.csv").as_bytes(); 
        let raw_data = BytesMut::from(raw_data);

        data.push_back(raw_data); 
        stream_data_guard.insert("plot_data".into(), ("application/text".into(), data));

    }

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    d3_plot: String,
    button: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let d3_plot = context_guard.render_to_string(D3PLOT);
    let button = context_guard.render_to_string(BUTTON);
    let html = AppPageTemplate {
        d3_plot,
        button,
    };
    html.render().unwrap()
}

fn build_actions(context: TnContext) -> TnEventActions {
    let mut actions = TnEventActions::default();

    let index = context.blocking_read().get_component_index(D3PLOT);
    actions.insert(
        index,
        (
            TnActionExecutionMethod::Await,
            Arc::new(d3_plot_clicked),
        ),
    );

    let index = context.blocking_read().get_component_index(BUTTON);
    actions.insert(
        index,
        (TnActionExecutionMethod::Await, Arc::new(button_clicked)),
    );

    actions
}


fn d3_plot_clicked(
    _context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app event", "{:?}", event);
        tracing::info!(target: "tron_app payload", "{:?}", payload);
        None
    };
    Box::pin(action)
}

fn button_clicked(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app", "{:?}", event);
        if event.e_trigger != BUTTON {
            None
        } else {
            {
                let raw_data = include_str!("../templates/2_TwoNum.csv").to_string();
                let raw_data = raw_data.split('\n').take(20).collect::<String>(); 
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                let data = stream_data_guard.get_mut("plot_data").unwrap();
                data.1.clear();
                data.1.push_back(bytes::BytesMut::from(raw_data.as_bytes()));
                tracing::info!(target: "tron_app stream_data", "{:?}", data.1[0].len());
            }
            let sse_tx = context.get_sse_tx().await;
            let msg = SseD3PlotTriggerMsg {
                server_side_trigger_data: TnServerSideTriggerData {
                    target: D3PLOT.into(),
                    new_state: "ready".into(),
                },
                d3_plot: "re-plot".into(),
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
            None
        }
    };
    Box::pin(action)
}
