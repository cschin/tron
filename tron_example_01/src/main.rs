use askama::Template;
use futures_util::Future;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_components::{
    text::TnTextInput, ActionExecutionMethod, ComponentBaseTrait, ComponentState, ComponentValue,
    Components, TnButton, TnEvent, TnEventActions, TnTextArea,
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

fn test_evt_task(
    components: Arc<RwLock<Components<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = || async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
        let mut i = 0;

        debug!("Event: {:?}", event.clone());
        loop {
            {
                let mut components_guard = components.write().await;
                let c = components_guard.get_mut_component_by_tron_id(&event.e_target);
                let v = match c.value() {
                    ComponentValue::String(s) => s.parse::<u32>().unwrap(),
                    _ => 0,
                };
                c.set_value(ComponentValue::String(format!("{:02}", v + 1)));
                c.set_state(ComponentState::Updating);

                let origin_string = if let ComponentValue::String(origin_string) = components_guard
                    .get_mut_component_by_tron_id("textarea")
                    .value()
                {
                    origin_string.clone()
                } else {
                    "".to_string()
                };

                components_guard
                    .get_mut_component_by_tron_id("textarea")
                    .set_value(ComponentValue::String(format!(
                        "{}\n {} -- {:02};",
                        origin_string,
                        event.e_target,
                        v + 1
                    )));
            }

            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"updating" }} }}"##,
                event.e_target
            );
            let v: Value = serde_json::from_str(data.as_str()).unwrap();
            if tx.send(axum::Json(v)).await.is_err() {
                debug!("tx dropped");
            }

            let data =
                r##"{"server_side_trigger": { "target":"textarea", "new_state":"ready" } }"##;
            let v: Value = serde_json::from_str(data).unwrap();
            if tx.send(axum::Json(v)).await.is_err() {
                debug!("tx dropped");
            }

            i += 1;
            interval.tick().await;
            debug!("loop triggered: {} {}", event.e_target, i);

            if i > 9 {
                break;
            };
        }
        {
            let mut components_guard = components.write().await;
            let c = components_guard.get_mut_component_by_tron_id(&event.e_target);
            c.set_state(ComponentState::Ready);
            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"{}" }} }}"##,
                event.e_target, "ready"
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
    let mut component_id = 0_u32;
    loop {
        let mut btn = TnButton::new(
            component_id,
            format!("btn-{:02}", component_id),
            format!("{:02}", component_id),
        );
        btn.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        components.add_component(btn);
        component_id += 1;
        if component_id >= 10 {
            break;
        }
    }

    let mut textarea = TnTextArea::new(component_id, "textarea".into(), "".into());
    textarea.set_attribute(
        "class".into(),
        "textarea textarea-bordered flex-1 min-h-80v".into(),
    );
    textarea.set_attribute(
        "hx-swap".into(),
        "outerHTML scroll:bottom focus-scroll:true".into(),
    );
    components.add_component(textarea);

    component_id += 1;

    let mut textinput = TnTextInput::new(component_id, "textinput".into(), "10".into());
    textinput.set_attribute("class".into(), "input w-full max-w-xs".into());
    textinput.set_attribute(
        "hx-swap".into(),
        "outerHTML scroll:bottom focus-scroll:true".into(),
    );
    components.add_component(textinput);

    components

    //Arc::new(RwLock::new(components))
}

fn build_session_actions() -> TnEventActions {
    let mut actions = TnEventActions::default();
    for i in 0..10 {
        let evt = TnEvent {
            e_target: format!("btn-{:02}", i),
            e_type: "click".to_string(),
            e_state: "ready".to_string(),
        };
        actions.insert(evt, (ActionExecutionMethod::Spawn, Arc::new(test_evt_task)));
    }
    actions
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    buttons: Vec<String>,
    textarea: String,
    textinput: String,
}

fn layout(components: &Components) -> String {
    let buttons = (0..10)
        .map(|i| {
            components
                .get_component_by_tron_id(format!("btn-{:02}", i).as_str())
                .render_to_string()
        })
        .collect::<Vec<String>>();
    let textarea = components
        .get_component_by_tron_id("textarea")
        .render_to_string();
    let textinput = components
        .get_component_by_tron_id("textinput")
        .render_to_string();
    let html = AppPageTemplate {
        buttons,
        textarea,
        textinput,
    };
    html.render().unwrap()
}
