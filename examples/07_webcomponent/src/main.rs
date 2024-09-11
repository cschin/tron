#![allow(dead_code)]
#![allow(unused_imports)]

mod sl_button;

use askama::Template;
use futures_util::Future;

use axum::{extract::Json, http::HeaderMap, response::Html};
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_app::tron_components::{
    self, button::TnButtonBuilder, tn_future, TnActionExecutionMethod, TnAsset,
    TnFutureHTMLResponse, TnFutureString, TnHtmlResponse,
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentState, TnComponentValue, TnContext, TnContextBase,
    TnEvent, TnTextArea,
};
//use std::sync::Mutex;
use sl_button::*;
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
    let app_share_data = tron_app::AppData::builder(build_context, layout)
        .set_head(include_str!("../templates/head.html"))
        .set_html_attributes(r#"lang="en" class="sl-theme-dark""#)
        .build();
    tron_app::run(app_share_data, app_config).await
}

// These functions are used to build the application context,
// layout, and event actions respectively
fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    let btn = SLButton::builder()
        .init(BUTTON.into(), "click me".into())
        .set_attribute("class", "btn btn-sm btn-outline btn-primary flex-1")
        .set_attribute("hx-target", "#count")
        .set_attribute("hx-swap", "innerHTML")
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

fn layout(context: TnContext) -> TnFutureString {
    tn_future! {
        let context_guard = context.read().await;
        let button = context_guard.render_to_string(BUTTON).await;
        let html = AppPageTemplate { button };
        html.render().unwrap()
    }
}

fn button_clicked(context: TnContext, event: TnEvent, _payload: Value) -> TnFutureHTMLResponse {
    tn_future! {
        tracing::info!(target: "tron_app", "{:?}", event);
        if event.e_trigger != BUTTON {
            None
        } else {
            let new_count = increase_count(context).await;
            Some((HeaderMap::new(), Html::from(format!("count: {new_count}"))))
        }
    }
}

async fn increase_count(context: TnContext) -> u32 {
    let asset_ref = context.get_asset_ref().await;
    let mut asset_guard = asset_ref.write().await;
    let count = asset_guard.get_mut("count").unwrap();
    if let TnAsset::U32(count) = count {
        *count += 1;
        *count
    } else {
        0
    }
}
