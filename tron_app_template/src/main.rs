use askama::Template;
use futures_util::Future;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_components::{
    text::TnTextInput, ComponentBaseTrait, ComponentState, ComponentValue, Components, TnButton,
    TnEvent, TnEventActions, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc};

#[tokio::main]
async fn main() {
    // set app state
    let app_share_data = tron_app::AppData {
        session_components: RwLock::new(HashMap::default()),
        session_sse_channels: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_session_components: Arc::new(Box::new(build_session_components)),
        build_session_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data).await
}

fn build_session_components() -> Components<'static> {
    let mut components = Components::default();
    components
}

fn layout(components: &Components) -> String {
    "This is an template, please fill in the components and how to layout them.".into()
}

fn build_session_actions() -> TnEventActions {
    let mut actions = TnEventActions::default();
    actions
}

fn test_event_action(
    components: Arc<RwLock<Components<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    todo!()
}
