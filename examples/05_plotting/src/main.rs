#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;

use axum::{extract::Json, http::HeaderMap, response::Html};
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_app::tron_components::{self, TnActionExecutionMethod, TnAsset, TnHtmlResponse, TnSimpleScatterPlot};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnEventActions, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc, task::Context, vec};

static SIMPLESCATTERPLOT: &str = "simple_scatter_plot";

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

    let component_index = 0;
    let xy = vec![ (0.0, 0.0,), (100.0, 100.0), (-100.0, 50.0)];
    let c = vec![ (1.0, 0.5, 1.0, 0.7), (0.0, 0.0, 0.1, 1.0), (0.0, 0.0, 1.0, 0.5)];
    let s = vec![ 5.0, 15.0, 10.0];
    let mut plot = TnSimpleScatterPlot::new(component_index, SIMPLESCATTERPLOT.into(), xy, c, s);

    plot.set_attribute("hx-swap".to_string(), "none".to_string());
  
    context.add_component(plot);

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    plot: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let plot = context_guard.render_to_string(SIMPLESCATTERPLOT);
    let html = AppPageTemplate { plot };
    html.render().unwrap()
}

fn build_actions(context: TnContext) -> TnEventActions {
    let mut actions = TnEventActions::default();
    let index = context.blocking_read().get_component_index(SIMPLESCATTERPLOT);
    actions.insert(
        index,
        (TnActionExecutionMethod::Await, Arc::new(button_clicked)),
    );
    actions
}

fn button_clicked(
    _context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app event", "{:?}", event);
        tracing::info!(target: "tron_app payload", "{:?}", payload);
        if event.e_trigger != SIMPLESCATTERPLOT {
            None
        } else {
            Some((HeaderMap::new(), Html::from("".to_string())))
        }
    };
    Box::pin(action)
}
