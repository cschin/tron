use askama::Template;
use futures_util::Future;

use axum::extract::Json;
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, RwLock};

use serde_json::Value;

use tracing::debug;
use tron_components::*;
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
    let mut component_id = 0_u32;
    let mut btn = TnButton::new(component_id, "rec_button".into(), "Start Recording".into());
    btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary flex-1".to_string(),
    );
    components.add_component(btn);

    component_id += 1;
    let mut recorder =
        TnAudioRecorder::new(component_id, "recorder".to_string(), "Paused".to_string());
    recorder.set_attribute("class".to_string(), "flex-1".to_string());
    components.add_component(recorder);

    //components.component_layout = Some(layout(&components));
    components
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    btn: String,
    recorder: String,
}

fn layout(components: &Components) -> String {
    let btn = components.render_to_string("rec_button");
    let recorder = components.render_to_string("recorder");
    let html = AppPageTemplate { btn, recorder };
    html.render().unwrap()
}

fn build_session_actions() -> TnEventActions {
    let mut actions = TnEventActions::default();
    let evt = TnEvent {
        e_target: "rec_button".into(),
        e_type: "click".into(),
        e_state: "ready".into(),
    };
    actions.insert(
        evt,
        (ActionExecutionMethod::Await, Arc::new(toggle_recording)),
    );
    actions
}

fn toggle_recording(
    components: Arc<RwLock<Components<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = || async move {
        let previous_rec_button_value;
        {
            let mut components_guard = components.write().await;
            let rec_button = components_guard.get_mut_component_by_tron_id(&event.e_target);
            previous_rec_button_value = (*rec_button.value()).clone();
            if let ComponentValue::String(value) = rec_button.value() {
                match value.as_str() {
                    "Stop Recording" => {
                        rec_button.set_value(ComponentValue::String("Start Recording".into()));
                        rec_button.set_state(ComponentState::Ready);
                    }
                    "Start Recording" => {
                        rec_button.set_value(ComponentValue::String("Stop Recording".into()));
                        rec_button.set_state(ComponentState::Ready);
                    }
                    _ => {}
                }
            }
        }
        {
            let mut components_guard = components.write().await;
            let recorder = components_guard.get_mut_component_by_tron_id("recorder");
            if let ComponentValue::String(value) = previous_rec_button_value {
                match value.as_str() {
                    "Stop Recording" => {
                        recorder.set_value(ComponentValue::String("Paused".into()));
                        recorder.set_state(ComponentState::Updating);
                        let data = format!(
                            r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"ready" }} }}"##,
                            "recorder" 
                        );
                        let v: Value = serde_json::from_str(&data).unwrap();
                        if tx.send(axum::Json(v)).await.is_err() {
                            debug!("tx dropped");
                        }
                    }
                    "Start Recording" => {
                        recorder.set_value(ComponentValue::String("Recording".into()));
                        recorder.set_state(ComponentState::Updating);
                        let data = format!(
                            r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"updating" }} }}"##,
                            "recorder" 
                        );
                        let v: Value = serde_json::from_str(&data).unwrap();
                        if tx.send(axum::Json(v)).await.is_err() {
                            debug!("tx dropped");
                        }
                    }
                    _ => {}
                }
            }
        }
        let data = format!(
            r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"ready" }} }}"##,
            event.e_target
        );
        let v: Value = serde_json::from_str(&data).unwrap();
        if tx.send(axum::Json(v)).await.is_err() {
            debug!("tx dropped");
        }
    };

    Box::pin(f())
}
