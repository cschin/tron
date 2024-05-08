#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnEventActions, TnTextArea,
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
    let app_share_data = tron_app::AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_context)),
        build_actions: Arc::new(Box::new(build_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, app_configure).await
}

fn build_context() -> TnContext {
    let context = Arc::new(RwLock::new(TnContextBase::default()));
    let context_guard = context.blocking_write();
    let logout_html = include_str!("../templates/logout.html");
    context_guard.asset.blocking_write().insert("logout_page".into(), tron_components::TnAsset::String(logout_html.into()));

    TnContext { base: context.clone() }
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
}

fn layout(_context: TnContext) -> String {
    let html = AppPageTemplate {};
    html.render().unwrap()
}

fn build_actions(_context: TnContext) -> TnEventActions {
    TnEventActions::default()
}

// fn test_event_action(
//     context: TnContext,
//     tx: Sender<Json<Value>>,
//     event: TnEvent,
// ) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
//     todo!()
// }
