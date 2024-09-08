#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;

use axum::{extract::Json, http::HeaderMap, response::Html};
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_app::tron_components::{
    self, button::TnButtonBuilder, TnActionExecutionMethod, TnAsset, TnHtmlResponse,
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc, task::Context};

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
        build_context: Arc::new(Box::new(build_context)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, app_config).await
}

// These functions are used to build the application context,
// layout, and event actions respectively
fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    let btn = TnButton::builder()
        .init(context.next_index(), BUTTON.into(), "click me".into())
        .set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        )
        .set_attribute("hx-target".to_string(), "#count".to_string())
        .set_attribute("hx-swap".to_string(), "innerHTML".to_string())
        .set_action(TnActionExecutionMethod::Await, button_clicked)
        .build();
    context
        .assets
        .blocking_write()
        .insert("count".into(), TnAsset::U32(0));

    context.add_component(btn);

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    button: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let button = context_guard.render_to_string(BUTTON);
    let html = AppPageTemplate { button };
    html.render().unwrap()
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
            let asset_ref = context.get_asset_ref().await;
            let mut asset_guard = asset_ref.write().await;
            let count = asset_guard.get_mut("count").unwrap();
            let new_count = if let TnAsset::U32(count) = count {
                *count += 1;
                *count
            } else {
                0
            };
            Some((HeaderMap::new(), Html::from(format!("count: {new_count}"))))
        }
    };
    Box::pin(action)
}
