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
    // set app state
    let app_share_data = tron_app::AppData {
        session_context: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_session_context: Arc::new(Box::new(build_session_context)),
        build_session_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, None).await
}

fn build_session_context() -> TnContext {
    let context = Arc::new(RwLock::new(TnContextBase::default()));
    TnContext { base: context }
}

fn layout(context: TnContext) -> String {
    "This is an template, please fill in the components and how to layout them.".into()
}

fn build_session_actions(context: TnContext) -> TnEventActions {
    let actions = TnEventActions::default();
    actions
}

fn test_event_action(
    context: TnContext,
    tx: Sender<Json<Value>>,
    event: TnEvent,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    todo!()
}
