mod dg_service;
mod open_ai_service;
use open_ai_service::simulate_dialog;

use askama::Template;
use dg_service::{deepgram_transcript_service, DeepgramError, StreamResponse};
use futures_util::Future;

use axum::body::Bytes;
use data_encoding::BASE64;
use serde::Deserialize;

use serde_json::Value;
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::oneshot;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex, RwLock,
};
#[allow(unused_imports)]
use tracing::{debug, info};
use tron_app::{send_sse_msg_to_client, SseAudioRecorderTriggerMsg, SseTriggerMsg, TriggerData};
use tron_components::*;

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

    //tron_app::run(app_share_data, Some("server=debug,tower_http=debug,tron_app=info")).await
    tron_app::run(
        app_share_data,
        Some("server=info,tower_http=info,tron_app=info"),
    )
    .await
}

fn build_session_context() -> Arc<RwLock<Context<'static>>> {
    let mut context = Context::<'static>::default();
    let mut component_id = 0_u32;
    let mut btn = TnButton::new(
        component_id,
        "rec_button".into(),
        "Start Conversation".into(),
    );
    btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary flex-1".to_string(),
    );
    context.add_component(btn);

    component_id += 1;
    let recorder =
        TnAudioRecorder::<'static>::new(component_id, "recorder".to_string(), "Paused".to_string());
    context.add_component(recorder);

    component_id += 1;
    let mut player =
        TnAudioPlayer::<'static>::new(component_id, "player".to_string(), "Paused".to_string());
    player.set_attribute("class".to_string(), "flex-1".to_string());
    context.add_component(player);

    component_id += 1;
    let mut transcript_output =
        TnChatBox::<'static>::new(component_id, "transcript".to_string(), vec![]);
    transcript_output.set_attribute(
        "class".to_string(),
        "flex flex-col overflow-auto border-2 border-gray-1000 rounded-lg flex-1 h-64 max-h-64 min-h-64".to_string(),
    );

    context.add_component(transcript_output);

    let context = Arc::new(RwLock::new(context));

    // add services
    {
        let (transcript_request_tx, transcript_request_rx) =
            tokio::sync::mpsc::channel::<ServiceRequestMessage>(1);
        let (transcript_response_tx, transcript_response_rx) =
            tokio::sync::mpsc::channel::<ServiceResponseMessage>(1);
        context.blocking_write().services.insert(
            "transcript_service".into(),
            (transcript_request_tx.clone(), Mutex::new(None)),
        );
        tokio::task::spawn(transcript_service(
            transcript_request_rx,
            transcript_response_tx,
        ));
        tokio::task::spawn(transcript_post_processing_service(
            context.clone(),
            transcript_response_rx,
        ));

        let (llm_request_tx, llm_request_rx) =
            tokio::sync::mpsc::channel::<ServiceRequestMessage>(1);
        context.blocking_write().services.insert(
            "llm_service".into(),
            (llm_request_tx.clone(), Mutex::new(None)),
        );
        tokio::task::spawn(simulate_dialog(context.clone(), llm_request_rx));
    }

    {
        let context_guard = context.blocking_write();
        let mut stream_data_guard = context_guard.stream_data.blocking_write();

        // stream_data_guard.insert("player".into(), ("audio/webm".into(), VecDeque::default()));
        stream_data_guard.insert("player".into(), ("audio/mp3".into(), VecDeque::default()));
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
    transcript: String,
}

fn layout(context: Arc<RwLock<Context<'static>>>) -> String {
    let context_guard = context.blocking_read();
    let btn = context_guard.render_to_string("rec_button");
    let recorder = context_guard.render_to_string("recorder");
    let player = context_guard.render_to_string("player");
    let transcript = context_guard.first_render_to_string("transcript");
    let html = AppPageTemplate {
        btn,
        recorder,
        player,
        transcript,
    };
    html.render().unwrap()
}

fn build_session_actions(_context: Arc<RwLock<Context<'static>>>) -> TnEventActions {
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

    let evt = TnEvent {
        e_target: "rec_button".into(),
        e_type: "server_side_trigger".into(),
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

    let evt = TnEvent {
        e_target: "player".into(),
        e_type: "ended".into(),
        e_state: "updating".into(),
    };
    actions.insert(
        evt,
        (
            ActionExecutionMethod::Await,
            Arc::new(audio_player::stop_audio_playing_action),
        ),
    );

    actions
}

fn toggle_recording(
    context: Arc<RwLock<Context<'static>>>,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let previous_rec_button_value;
        let sse_tx = get_sse_tx_with_context(context.clone()).await;
        {
            let rec_button = get_component_with_contex(context.clone(), "rec_button").await;
            let mut rec_button = rec_button.write().await;
            previous_rec_button_value = (*rec_button.value()).clone();

            if let ComponentValue::String(value) = rec_button.value() {
                match value.as_str() {
                    "Stop Conversation" => {
                        rec_button.set_value(ComponentValue::String("Start Conversation".into()));
                        rec_button.set_state(ComponentState::Ready);
                    }
                    "Start Conversation" => {
                        rec_button.set_value(ComponentValue::String("Stop Conversation".into()));
                        rec_button.set_state(ComponentState::Ready);
                    }
                    _ => {}
                }
            }
        }
        {
            // Fore stop and start the recording stream
            if let ComponentValue::String(value) = previous_rec_button_value {
                match value.as_str() {
                    "Stop Conversation" => {
                        {
                            set_value_with_context(
                                &context,
                                "recorder",
                                ComponentValue::String("Paused".into()),
                            )
                            .await;
                            set_state_with_context(&context, "recorder", ComponentState::Updating)
                                .await;
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
                            send_sse_msg_to_client(&sse_tx, msg).await;
                        }

                        {
                            // ask the transcription service to stop
                            let context_guard = context.read().await;
                            let (trx_srv, _) =
                                context_guard.services.get("transcript_service").unwrap();
                            let (tx, rx) = oneshot::channel::<String>();
                            let trx_req_msg = ServiceRequestMessage {
                                request: "stop".into(),
                                payload: TnAsset::VecU8(vec![]),
                                response: tx,
                            };
                            let _ = trx_srv.send(trx_req_msg).await;
                            if let Ok(out) = rx.await {
                                tracing::debug!(target:"tron_app", "returned string: {}", out);
                            };
                        }
                    }
                    "Start Conversation" => {
                        set_value_with_context(
                            &context,
                            "recorder",
                            ComponentValue::String("Recording".into()),
                        )
                        .await;
                        set_state_with_context(&context, "recorder", ComponentState::Updating)
                            .await;

                        {
                            let context_guard = context.write().await;
                            let mut stream_data_guard = context_guard.stream_data.write().await;
                            stream_data_guard.get_mut("player").unwrap().1.clear();
                            // clear the stream buffer
                        }

                        let msg = SseAudioRecorderTriggerMsg {
                            server_side_trigger: TriggerData {
                                target: "recorder".into(),
                                new_state: "ready".into(),
                            },
                            audio_recorder_control: "start".into(),
                        };
                        send_sse_msg_to_client(&sse_tx, msg).await;
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
        send_sse_msg_to_client(&sse_tx, msg).await;
    };

    Box::pin(f)
}

#[derive(Clone, Debug, Deserialize)]
struct AudioChunk {
    audio_data: String,
}

fn audio_input_stream_processing(
    context: Arc<RwLock<Context<'static>>>,
    _event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let chunk: Option<AudioChunk> = serde_json::from_value(payload).unwrap_or(None);
        if let Some(chunk) = chunk {
            let b64data = chunk.audio_data.trim_end_matches('"');
            let mut split = b64data.split(',');
            let _head = split.next().unwrap();
            let b64str = split.next().unwrap();
            let chunk = Bytes::from(BASE64.decode(b64str.as_bytes()).unwrap());

            {
                let recorder = get_component_with_contex(context.clone(), "recorder").await;
                audio_recorder::append_audio_data(recorder.clone(), chunk.clone()).await;
            }

            {
                let context_guard = context.read().await;
                let (trx_srv, _) = context_guard.services.get("transcript_service").unwrap();
                let (tx, rx) = oneshot::channel::<String>();
                let trx_req_msg = ServiceRequestMessage {
                    request: "sending audio".into(),
                    payload: TnAsset::VecU8(chunk.to_vec()),
                    response: tx,
                };
                let _ = trx_srv.send(trx_req_msg).await;
                if let Ok(out) = rx.await {
                    tracing::debug!(target: "tron_app", "sending audio, returned string: {}", out );
                };
                // XXXX
            }
        };
    };

    Box::pin(f)
}

async fn transcript_service(
    mut rx: Receiver<ServiceRequestMessage>,
    tx: Sender<ServiceResponseMessage>,
) {
    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<StreamResponse>(1);
    tokio::spawn(async move {
        tracing::debug!(target: "tran_app", "restart dg_trx");
        let (mut audio_tx, audio_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(1);
        let mut handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        while let Some(req) = rx.recv().await {
            tracing::debug!(target: "tran_app", "req: {}", req.request);
            if handle.is_finished() {
                audio_tx.closed().await;
                let (audio_tx0, audio_rx) =
                    tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(1);
                audio_tx = audio_tx0;
                handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            if let TnAsset::VecU8(payload) = req.payload {
                tracing::debug!(target: "tran_app", "req received: {}, payload: {}", req.request, payload.len());
                let _ = audio_tx.send(Ok(Bytes::from_iter(payload))).await;
                let _ = req.response.send("audio sent to trx service".to_string());
            }
        }
        audio_tx.closed().await;
    });

    let mut transcript_fragments = Vec::<String>::new();
    while let Some(trx_rtn) = transcript_rx.recv().await {
        match trx_rtn {
            StreamResponse::TerminalResponse {
                request_id: _,
                created: _,
                duration: _,
                channels: _,
            } => {}
            StreamResponse::TranscriptResponse {
                duration: _,
                is_final,
                speech_final,
                channel,
            } => {
                if is_final && !channel.alternatives.is_empty() {
                    let trx_fragment = channel
                        .alternatives
                        .first()
                        .unwrap()
                        .transcript
                        .trim()
                        .to_string();
                    if !trx_fragment.is_empty() {
                        transcript_fragments.push(trx_fragment.clone());
                    }
                    let _ = tx
                        .send(ServiceResponseMessage {
                            response: "transcript_fragment".to_string(),
                            payload: TnAsset::String(trx_fragment),
                        })
                        .await;
                }
                if speech_final {
                    let transcript = transcript_fragments.join(" ").trim().to_string();
                    if !transcript.is_empty() {
                        let _ = tx
                            .send(ServiceResponseMessage {
                                response: "transcript_final".to_string(),
                                payload: TnAsset::String(transcript),
                            })
                            .await;
                        transcript_fragments.clear();
                    }
                }
            }
            StreamResponse::UtteranceEnd { last_word_end: _ } => {
                let transcript = transcript_fragments.join(" ").trim().to_string();
                if !transcript.is_empty() {
                    let _ = tx
                        .send(ServiceResponseMessage {
                            response: "transcript_final".to_string(),
                            payload: TnAsset::String(transcript),
                        })
                        .await;
                    transcript_fragments.clear();
                }
            }
        };
    }
}

async fn transcript_post_processing_service(
    context: Arc<RwLock<Context<'static>>>,
    mut response_rx: Receiver<ServiceResponseMessage>,
) {
    let assets = context.read().await.assets.clone();
    let components = context.read().await.components.clone();
    let transcript_area_id = context.read().await.get_component_id("transcript");
    while let Some(response) = response_rx.recv().await {
        match response.response.as_str() {
            "transcript_final" => {
                if let TnAsset::String(transcript) = response.payload {
                    {
                        let llm_tx = context
                            .read()
                            .await
                            .services
                            .get("llm_service")
                            .unwrap()
                            .0
                            .clone();

                        let (tx, rx) = oneshot::channel::<String>();
                        let llm_req_msg = ServiceRequestMessage {
                            request: "chat-complete".into(),
                            payload: TnAsset::String(transcript.clone()),
                            response: tx,
                        };
                        let _ = llm_tx.send(llm_req_msg).await;
                        if let Ok(out) = rx.await {
                            tracing::debug!(target: "tron_app", "returned string: {}", out);
                        };
                        {
                            let components_guard = components.write().await;
                            let transcript_area =
                                components_guard.get(&transcript_area_id).unwrap();
                            chatbox::append_chatbox_value(
                                transcript_area.clone(),
                                ("user".into(), transcript),
                            )
                            .await;
                        }
                        {
                            let msg = SseTriggerMsg {
                                server_side_trigger: TriggerData {
                                    target: "transcript".into(),
                                    new_state: "ready".into(),
                                },
                            };
                            let sse_tx = get_sse_tx_with_context(context.clone()).await;
                            send_sse_msg_to_client(&sse_tx, msg).await;
                        }
                    }
                }
            }
            "transcript_fragment" => {
                if let TnAsset::String(transcript) = response.payload {
                    if !transcript.is_empty() {
                        let mut assets_guard = assets.write().await;
                        let e = assets_guard.entry("transcript".into()).or_default();
                        (*e).push(TnAsset::String(transcript.clone()));
                    }
                    // maybe show the interim transcription results somewhere
                }
            }
            _ => {}
        }
    }
}
