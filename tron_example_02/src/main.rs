use askama::Template;
use futures_util::Future;

use axum::{body::Bytes, extract::Json};
use serde::Deserialize;
//use serde::{Deserialize, Serialize};
use base64::prelude::*;
use bytes::{BufMut, BytesMut};
use serde_json::Value;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    RwLock,
};
#[allow(unused_imports)]
use tracing::debug;
use tron_app::{
    utils::send_sse_msg_to_client, SseAudioRecorderTriggerMsg, SseTriggerMsg, TriggerData,
};
use tron_components::*;
//use std::sync::Mutex;
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::oneshot;

#[tokio::main]
async fn main() {
    // set app state
    let app_share_data = tron_app::AppData {
        session_context: RwLock::new(HashMap::default()),
        session_sse_channels: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_session_context: Arc::new(Box::new(build_session_context)),
        build_session_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };

    tron_app::run(app_share_data).await
}

fn build_session_context() -> Context<'static> {
    let mut context = Context::<'static>::default();
    let mut component_id = 0_u32;
    let mut btn = TnButton::new(component_id, "rec_button".into(), "Start Recording".into());
    btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary flex-1".to_string(),
    );
    context.add_component(btn);

    component_id += 1;
    let mut recorder =
        TnAudioRecorder::new(component_id, "recorder".to_string(), "Paused".to_string());
    recorder.set_attribute("class".to_string(), "flex-1".to_string());
    context.add_component(recorder);

    component_id += 1;
    let mut player = TnAudioPlayer::new(component_id, "player".to_string(), "Paused".to_string());
    player.set_attribute("class".to_string(), "flex-1".to_string());
    context.add_component(player);

    // add service
    {
        let (tx, rx) = tokio::sync::mpsc::channel::<ServiceRequestMessage>(1);
        context
            .services
            .insert("transcript_service".into(), tx.clone());
        //services_guide.insert("transcript_service".into(), tx.clone());
        tokio::task::spawn(transcript_service(rx));
    }

    {
        context
            .stream_data
            .insert("player".into(), ("audio/webm".into(), VecDeque::default()));
    }

    //components.component_layout = Some(layout(&components));
    context
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    btn: String,
    recorder: String,
    player: String,
}

fn layout(context: &Context<'static>) -> String {
    let btn = context.render_to_string("rec_button");
    let recorder = context.render_to_string("recorder");
    let player = context.render_to_string("player");
    let html = AppPageTemplate {
        btn,
        recorder,
        player,
    };
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
    context: Arc<RwLock<Context<'static>>>,
    tx: Sender<Json<Value>>,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let previous_rec_button_value;
        {
            let mut context_guard = context.write().await;
            let rec_button = context_guard.get_mut_component_by_tron_id(&event.e_target);
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
                            let mut context_guard = context.write().await;
                            let recorder = context_guard.get_mut_component_by_tron_id("recorder");
                            recorder.set_value(ComponentValue::String("Paused".into()));
                            recorder.set_state(ComponentState::Updating);
                            let mut delay =
                                tokio::time::interval(tokio::time::Duration::from_millis(300));
                            delay.tick().await; //The first tick completes immediately.
                            delay.tick().await; //wait a bit for all data stream transferred
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
                            let context_guard = context.read().await;
                            let recorder = context_guard.get_component_by_tron_id("recorder");
                            audio_recorder::write_audio_data_to_file(recorder);
                        }
                        let msg = SseTriggerMsg {
                            server_side_trigger: TriggerData {
                                target: "player".into(),
                                new_state: "ready".into(),
                            },
                        };
                        send_sse_msg_to_client(&tx, msg).await;
                    }
                    "Start Recording" => {
                        {
                            let mut context_guard = context.write().await;
                            let recorder = context_guard.get_mut_component_by_tron_id("recorder");
                            recorder.set_value(ComponentValue::String("Recording".into()));
                            recorder.set_state(ComponentState::Updating);
                        }
                        {
                            let mut context_guard = context.write().await;
                            context_guard
                                .stream_data
                                .get_mut("player")
                                .unwrap()
                                .1
                                .clear(); // clear the stream buffer
                        }
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
        let msg = SseTriggerMsg {
            server_side_trigger: TriggerData {
                target: event.e_target,
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&tx, msg).await;
    };

    Box::pin(f)
}

#[derive(Clone, Debug, Deserialize)]
struct AudioChunk {
    audio_data: String,
}

fn audio_input_stream_processing(
    context: Arc<RwLock<Context<'static>>>,
    _tx: Sender<Json<Value>>,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let chunk: Option<AudioChunk> = serde_json::from_value(payload).unwrap_or(None);
        if let Some(chunk) = chunk {
            println!(
                "stream, audio data received, len={}",
                chunk.audio_data.len()
            );

            let id = {
                let context_guard = context.read().await;
                *context_guard.tron_id_to_id.get(&event.e_target).unwrap()
            };

            let b64data = chunk.audio_data.trim_end_matches('"');
            let mut split = b64data.split(',');
            let _head = split.next().unwrap();
            let b64str = split.next().unwrap();
            let chunk = Bytes::from(BASE64_STANDARD.decode(b64str).unwrap());

            {
                let mut context_guard = context.write().await;
                let recorder = context_guard.components.get_mut(&id).unwrap();
                audio_recorder::append_audio_data(recorder, chunk.clone());
            }
            {
                let mut context_guard = context.write().await;
                let player_data = context_guard.stream_data.get_mut("player").unwrap();
                let mut data = BytesMut::new();
                data.put(&chunk[..]);
                player_data.1.push_back(data);
            }
            {
                let context_guard = context.read().await;
                let trx_srv = context_guard.services.get("transcript_service").unwrap();
                let (tx, rx) = oneshot::channel::<String>();
                let trx_req_msg = ServiceRequestMessage {
                    request: "Here I am".into(),
                    payload: Vec::default(),
                    response: tx,
                };
                let _ = trx_srv.send(trx_req_msg).await;
                if let Ok(out) = rx.await {
                    println!("returned string: {}", out);
                };
            }
        };
    };

    Box::pin(f)
}

async fn transcript_service(mut rx: Receiver<ServiceRequestMessage>) {
    while let Some(req) = rx.recv().await {
        println!("req received: {}", req.request);
        let _ = req.response.send("the response".to_string());
    }
}
