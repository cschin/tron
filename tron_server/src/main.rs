mod tn_app;
use futures_util::Future;
use tn_app::*;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tron_components::{
    text::TnText, ComponentBaseTrait, ComponentValue, Components, TnButton, TnEvent, TnEventActions,
    ComponentState
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc};


#[tokio::main]
async fn main() {
    // set app state
    let app_share_data = AppData {
        session_components: RwLock::new(HashMap::default()),
        session_sse_channels: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_session_components: Arc::new(Box::new(build_session_components)),
        build_session_actions: Arc::new(Box::new(build_session_actions)),
    };
    tn_app::run(app_share_data).await
}

fn test_evt_task(
    components: Arc<RwLock<Components<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = || async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(500));
        let mut i = 0;

        println!("Event: {:?}", event);
        loop {
            {
                let mut components_guard = components.write().await;
                let c = components_guard.get_mut_component_by_tron_id(&event.evt_target);
                let v = match c.value() {
                    ComponentValue::String(s) => s.parse::<u32>().unwrap(),
                    _ => 0,
                };
                c.set_value(ComponentValue::String(format!("{:02}", v + 1)));
                c.set_state(ComponentState::Updating);

                let origin_string = if let ComponentValue::String(origin_string) = components_guard
                    .get_mut_component_by_tron_id("text-11")
                    .value() {
                        origin_string.clone()
                    } else {
                        "".to_string()
                    };

                components_guard
                    .get_mut_component_by_tron_id("text-11")
                    .set_value(ComponentValue::String(format!("{}<br>{}--{:02}", origin_string, event.evt_target, v + 1)));
            }

            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"updating" }} }}"##,
                event.evt_target
            );
            let v: Value = serde_json::from_str(data.as_str()).unwrap();
            if tx.send(axum::Json(v)).await.is_err() {
                println!("tx dropped");
            }

            let data = r##"{"server_side_trigger": { "target":"text-11", "new_state":"ready" } }"##;
            let v: Value = serde_json::from_str(data).unwrap();
            if tx.send(axum::Json(v)).await.is_err() {
                println!("tx dropped");
            }

            i += 1;
            interval.tick().await;
            println!("loop triggered: {} {}", event.evt_target, i);

            if i > 10 {
                break;
            };
        }
        {
            let mut components_guard = components.write().await;
            let c = components_guard.get_mut_component_by_tron_id(&event.evt_target);
            c.set_state(ComponentState::Ready);
            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"{}" }} }}"##,
                event.evt_target, "ready"
            );
            let v: Value = serde_json::from_str(data.as_str()).unwrap();
            if tx.send(axum::Json(v)).await.is_err() {
                println!("tx dropped");
            }
        }
    };
    Box::pin(f())
}

fn build_session_components() -> Components<'static> {
    let mut components = Components::default();

    for i in 0..10 {
        let mut btn = TnButton::new(i, format!("btn-{:02}", i), format!("{:02}", i));
        btn.set_attribute("hx-post".to_string(), format!("/tron/{}", i));
        btn.set_attribute("hx-swap".to_string(), "outerHTML".to_string());
        btn.set_attribute("hx-target".to_string(), format!("#btn-{:02}", i));
        components.add_component(btn);
    }

    let mut text = TnText::new(11, format!("text-{:02}", 11), "Text".to_string());
    text.set_attribute("hx-post".to_string(), format!("/tron/{}", 11));
    text.set_attribute("hx-swap".to_string(), "outerHTML".to_string());
    components.add_component(text);
    components

    //Arc::new(RwLock::new(components))
}

fn build_session_actions() -> TnEventActions {
    let mut actions = TnEventActions::default();
    for i in 0..10 {
        let evt = TnEvent {
            evt_target: format!("btn-{:02}", i),
            evt_type: "click".to_string(),
            state: "ready".to_string(),
        };
        actions.insert(evt, Arc::new(test_evt_task));
    }
    actions
}
