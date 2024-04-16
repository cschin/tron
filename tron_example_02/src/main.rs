use askama::Template;
use futures_util::Future;

use axum::{body::Bytes, extract::Json};
use serde::Deserialize;
//use serde::{Deserialize, Serialize};
use base64::prelude::*;
use serde_json::Value;
use tokio::sync::{mpsc::Sender, RwLock};
#[allow(unused_imports)]
use tracing::debug;
use tron_app::{
    utils::send_sse_msg_to_client, SseAudioRecorderTriggerMsg, SseTriggerMsg, TriggerData,
};
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
    // for processing rec button click
    let evt = TnEvent {
        e_target: "rec_button".into(),
        e_type: "click".into(),
        e_state: "ready".into(),
    };
    actions.insert(
        evt,
        (ActionExecutionMethod::Await, Arc::new(toggle_recording)),
    );

    // for processing the incoming audio stream data
    let evt = TnEvent {
        e_target: "recorder".into(),
        e_type: "streaming".into(),
        e_state: "updating".into(),
    };
    actions.insert(
        evt,
        (
            ActionExecutionMethod::Await,
            Arc::new(audio_input_stream_processing),
        ),
    );

    actions
}

fn toggle_recording(
    components: Arc<RwLock<Components<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
    _payload: Value,
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
            if let ComponentValue::String(value) = previous_rec_button_value {
                match value.as_str() {
                    "Stop Recording" => {
                        {
                            let mut components_guard = components.write().await;
                            let recorder =
                                components_guard.get_mut_component_by_tron_id("recorder");
                            recorder.set_value(ComponentValue::String("Paused".into()));
                            recorder.set_state(ComponentState::Updating);
                            let msg = SseAudioRecorderTriggerMsg {
                                server_side_trigger: TriggerData {
                                    target: "recorder".into(),
                                    new_state: "updating".into(),
                                },
                                audio_recorder_control: "stop".into(),
                            };
                            send_sse_msg_to_client(&tx, msg).await;
                        }
                        {
                            let components_guard = components.read().await;
                            let recorder = components_guard.get_component_by_tron_id("recorder");
                            audio_recorder::write_audio_data_to_file(recorder);
                        }
                    }
                    "Start Recording" => {
                        let mut components_guard = components.write().await;
                        let recorder = components_guard.get_mut_component_by_tron_id("recorder");
                        recorder.set_value(ComponentValue::String("Recording".into()));
                        recorder.set_state(ComponentState::Updating);
                        let msg = SseAudioRecorderTriggerMsg {
                            server_side_trigger: TriggerData {
                                target: "recorder".into(),
                                new_state: "updating".into(),
                            },
                            audio_recorder_control: "start".into(),
                        };
                        send_sse_msg_to_client(&tx, msg).await;
                    }
                    _ => {}
                }
            }
        }
        let data = SseTriggerMsg {
            server_side_trigger: TriggerData {
                target: event.e_target,
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&tx, data).await;
    };

    Box::pin(f())
}

#[derive(Clone, Debug, Deserialize)]
struct AudioChunk {
    audio_data: String,
}

fn audio_input_stream_processing(
    components: Arc<RwLock<Components<'static>>>,
    _tx: Sender<Json<Value>>,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = || async move {
        let chunk: Option<AudioChunk> = serde_json::from_value(payload).unwrap_or(None);
        if let Some(chunk) = chunk {
            println!(
                "stream, audio data received, len={}",
                chunk.audio_data.len()
            );

            let id = {
                let components_guard = components.read().await;
                *components_guard.tron_id_to_id.get(&event.e_target).unwrap()
            };

            let b64data = chunk.audio_data.trim_end_matches('"');
            let mut split = b64data.split(',');
            let _head = split.next().unwrap();
            let b64str = split.next().unwrap();
            let chunk = Bytes::from(BASE64_STANDARD.decode(b64str).unwrap());

            {
                let mut components_guard = components.write().await;
                let recorder = components_guard.components.get_mut(&id).unwrap();
                audio_recorder::append_audio_data(recorder, chunk);
            }
        };
    };

    Box::pin(f())
}
