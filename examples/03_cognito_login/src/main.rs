#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_app::tron_components;
use tron_components::{
    text::TnTextInput, TnButton, TnComponentState, TnComponentValue, TnContext, TnContextBase,
    TnEvent, TnFutureString, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc};

#[tokio::main]
async fn main() {
    let app_configure = tron_app::AppConfigure {
        cognito_login: true,
        ..Default::default()
    };
    // set app state
    let app_share_data = tron_app::AppData::builder(build_context, layout).build();
    tron_app::run(app_share_data, app_configure).await
}

fn build_context() -> TnContext {
    let context = Arc::new(RwLock::new(TnContextBase::default()));
    let context_guard = context.blocking_write();
    let logout_html = include_str!("../templates/logout.html");
    context_guard.assets.blocking_write().insert(
        "logout_page".into(),
        tron_components::TnAsset::String(logout_html.into()),
    );

    TnContext {
        base: context.clone(),
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {}

fn layout(_context: TnContext) -> TnFutureString {
    Box::pin(async {
        let html = AppPageTemplate {};
        html.render().unwrap()
    })
}

// fn test_event_action(
//     context: TnContext,
//     tx: Sender<Json<Value>>,
//     event: TnEvent,
// ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
//     todo!()
// }
