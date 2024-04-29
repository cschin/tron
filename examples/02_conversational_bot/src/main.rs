mod dg_service;
mod openai_service;
use openai_service::simulate_dialog;

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
    time::SystemTime,
};
use tokio::sync::oneshot;
use tokio::sync::{
    mpsc::{Receiver, Sender},
    Mutex, RwLock,
};
#[allow(unused_imports)]
use tracing::{debug, info};
use tron_app::{send_sse_msg_to_client, SseTriggerMsg, TriggerData};
use tron_components::audio_recorder::SseAudioRecorderTriggerMsg;
use tron_components::{text::append_and_send_stream_textarea_with_context, *};

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

fn build_session_context() -> TnContext {
    let mut context = TnContextBase::<'static>::default();
    let mut component_id = 0_u32;

    {
        // add a recorder button
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
    }
    {
        // add a recorder
        component_id += 1;
        let recorder = TnAudioRecorder::<'static>::new(
            component_id,
            "recorder".to_string(),
            "Paused".to_string(),
        );
        context.add_component(recorder);
    }
    {
        // add a player
        component_id += 1;
        let mut player =
            TnAudioPlayer::<'static>::new(component_id, "player".to_string(), "Paused".to_string());
        player.set_attribute("class".to_string(), "flex-1 p-1 h-10".to_string());
        context.add_component(player);
    }

    {
        // add a reset button
        component_id += 1;
        let mut btn = TnButton::new(
            component_id,
            "reset_button".into(),
            "Reset The Conversation".into(),
        );
        btn.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        context.add_component(btn);
    }
    {
        // add a chatbox
        component_id += 1;
        let mut transcript_output =
            TnChatBox::<'static>::new(component_id, "transcript".to_string(), vec![]);
        transcript_output.set_attribute(
            "class".to_string(),
            "flex flex-col overflow-auto flex-1 p-2".to_string(),
        );

        context.add_component(transcript_output);
    }
    {
        // add a status box
        component_id += 1;
        let mut status_output =
            TnStreamTextArea::<'static>::new(component_id, "status".to_string(), vec![]);
        status_output.set_attribute(
            "class".to_string(),
            "flex-1 p-2 textarea textarea-bordered h-40 max-h-40 min-h-40".to_string(),
        );
        status_output.set_attribute("hx-trigger".into(), "server_side_trigger".into());

        context.add_component(status_output);
    }

    {
        component_id += 1;
        let prompt = include_str!("../templates/prompt1.txt");
        let mut prompt_box =
            TnTextArea::<'static>::new(component_id, "prompt".into(), prompt.into());
        prompt_box.remove_attribute("disabled".into());
        prompt_box.set_attribute("hx-trigger".into(), "change, server_side_trigger".into()); // change will update the value one the textarea is out of focus

        prompt_box.set_attribute(
            "class".into(),
            "flex-1 p-2 textarea textarea-bordered mx-auto h-96 max-h-96 min-h-96".into(),
        );
        prompt_box.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        );

        context.add_component(prompt_box);
    }

    // create a TnContext so we can pass to some services which need to interact with the components
    let context = TnContext {
        base: Arc::new(RwLock::new(context)),
    };

    // add services
    {
        let (transcript_request_tx, transcript_request_rx) =
            tokio::sync::mpsc::channel::<TnServiceRequestMsg>(1);
        let (transcript_response_tx, transcript_response_rx) =
            tokio::sync::mpsc::channel::<TnServiceResponseMsg>(1);
        context.blocking_write().services.insert(
            "transcript_service".into(),
            (transcript_request_tx.clone(), Mutex::new(None)),
        );
        // service sending audio stream to deepgram
        tokio::task::spawn(transcript_service(
            transcript_request_rx,
            transcript_response_tx,
        ));
        // service processing the output from deepgram
        tokio::task::spawn(transcript_post_processing_service(
            context.clone(),
            transcript_response_rx,
        ));

        // service handling the LLM and TTS at once
        let (llm_request_tx, llm_request_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(1);
        context.blocking_write().services.insert(
            "llm_service".into(),
            (llm_request_tx.clone(), Mutex::new(None)),
        );
        tokio::task::spawn(simulate_dialog(context.clone(), llm_request_rx));
    }

    {
        // set up audio stream out
        let context_guard = context.blocking_write();
        let mut stream_data_guard = context_guard.stream_data.blocking_write();
        stream_data_guard.insert("player".into(), ("audio/mp3".into(), VecDeque::default()));
    }

    context
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    btn: String,
    recorder: String,
    player: String,
    transcript: String,
    status: String,
    prompt: String,
    reset_button: String,
}

fn layout(context: TnContext) -> String {
    let guard = context.blocking_read();
    let btn = guard.render_to_string("rec_button");
    let recorder = guard.render_to_string("recorder");
    let player = guard.render_to_string("player");
    let transcript = guard.first_render_to_string("transcript");
    let status = guard.first_render_to_string("status");
    let prompt = guard.first_render_to_string("prompt");
    let reset_button = guard.first_render_to_string("reset_button");

    let html = AppPageTemplate {
        btn,
        recorder,
        player,
        transcript,
        status,
        prompt,
        reset_button,
    };
    html.render().unwrap()
}

fn build_session_actions(_context: TnContext) -> TnEventActions {
    let mut actions = TnEventActions::default();
    {
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
    }
    {
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
    }
    {
        // handling player ended event
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
    }
    {
        // for processing reset button click
        let evt = TnEvent {
            e_target: "reset_button".into(),
            e_type: "click".into(),
            e_state: "ready".into(),
        };

        actions.insert(
            evt,
            (ActionExecutionMethod::Await, Arc::new(reset_conversation)),
        );
    }

    actions
}

fn _do_nothing(
    context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        //let comp = context.get_component(&event.e_target);
        tracing::info!(target: "tron_app", "{:?}", payload);
        context
            .set_state_for_component(&event.e_target.clone(), TnComponentState::Pending)
            .await;
        let sse_tx = context.get_sse_tx_with_context().await;
        let msg = SseTriggerMsg {
            server_side_trigger: TriggerData {
                target: event.e_target.clone(),
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&sse_tx, msg).await;
        let comp = context.get_component(&event.e_target).await;
        tracing::info!(target: "tron_app", "value: {:?}", comp.read().await.value());
    };
    Box::pin(f)
}

fn reset_conversation(
    context: TnContext,
    _event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let llm_tx = context
            .read()
            .await
            .services
            .get("llm_service")
            .unwrap()
            .0
            .clone();
        let (tx, mut rx) = oneshot::channel::<String>();

        let llm_req_msg = TnServiceRequestMsg {
            request: "clear-history".into(),
            payload: TnAsset::String("".into()),
            response: tx,
        };

        let _ = llm_tx.send(llm_req_msg).await;
        let _ = rx.try_recv();

        let sse_tx = context.get_sse_tx_with_context().await;
        {
            // remove the transcript in the chatbox component, and sent the hx-reswap to innerHTML
            // once the server side trigger for an update, the content will be empty
            // the hx-reswap will be removed when there is new text in append_chatbox_value()
            context
                .set_value_for_component("transcript", TnComponentValue::VecString2(vec![]))
                .await;
            let guard = context.get_component("transcript").await;
            guard
                .write()
                .await
                .set_header("hx-reswap".into(), "innerHTML".into());

            let msg = SseTriggerMsg {
                server_side_trigger: TriggerData {
                    target: "transcript".into(),
                    new_state: "ready".into(),
                },
            };

            send_sse_msg_to_client(&sse_tx, msg).await;
        }
        {
            context
                .set_state_for_component("reset_button", TnComponentState::Ready)
                .await;

            let msg = SseTriggerMsg {
                // update the button state
                server_side_trigger: TriggerData {
                    target: "reset_button".into(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    };
    Box::pin(f)
}

fn toggle_recording(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let previous_rec_button_value;
        let sse_tx = context.get_sse_tx_with_context().await;
        {
            let rec_button = context.get_component("rec_button").await;
            let mut rec_button = rec_button.write().await;
            previous_rec_button_value = (*rec_button.value()).clone();

            if let TnComponentValue::String(value) = rec_button.value() {
                match value.as_str() {
                    "Stop Conversation" => {
                        rec_button.set_value(TnComponentValue::String("Start Conversation".into()));
                        rec_button.set_state(TnComponentState::Ready);
                    }
                    "Start Conversation" => {
                        rec_button.set_value(TnComponentValue::String("Stop Conversation".into()));
                        rec_button.set_state(TnComponentState::Ready);
                    }
                    _ => {}
                }
            }
        }
        {
            // Fore stop and start the recording stream
            if let TnComponentValue::String(value) = previous_rec_button_value {
                match value.as_str() {
                    "Stop Conversation" => {
                        {
                            context
                                .set_value_for_component(
                                    "recorder",
                                    TnComponentValue::String("Paused".into()),
                                )
                                .await;
                            context
                                .set_state_for_component("recorder", TnComponentState::Updating)
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
                            let trx_req_msg = TnServiceRequestMsg {
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
                        context
                            .set_value_for_component(
                                "recorder",
                                TnComponentValue::String("Recording".into()),
                            )
                            .await;

                        context
                            .set_state_for_component("recorder", TnComponentState::Updating)
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
    context: TnContext,
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
            let chunk_len = chunk.len();

            {
                let recorder = context.get_component("recorder").await;
                audio_recorder::append_audio_data(recorder.clone(), chunk.clone()).await;
            }

            {
                let context_guard = context.read().await;
                let (trx_srv, _) = context_guard.services.get("transcript_service").unwrap();
                let (tx, rx) = oneshot::channel::<String>();
                let trx_req_msg = TnServiceRequestMsg {
                    request: "sending audio".into(),
                    payload: TnAsset::VecU8(chunk.to_vec()),
                    response: tx,
                };
                let _ = trx_srv.send(trx_req_msg).await;
                if let Ok(out) = rx.await {
                    tracing::debug!(target: "tron_app", "sending audio, returned string: {}", out );
                };
            }

            {
                let now = SystemTime::now()
                    .duration_since(SystemTime::UNIX_EPOCH)
                    .unwrap()
                    .as_micros();
                let now = now % 1_000_000;
                if now < 100_000 {
                    // sample 1/10 of the data
                    append_and_send_stream_textarea_with_context(
                        context.clone(),
                        "status",
                        &format!("recording audio, chunk length: {chunk_len}\n"),
                    )
                    .await;
                }
            }
        };
    };

    Box::pin(f)
}

async fn transcript_service(
    mut rx: Receiver<TnServiceRequestMsg>,
    tx: Sender<TnServiceResponseMsg>,
) {
    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<StreamResponse>(1);

    // start a loop for maintaining connection with DG, this calls the DG WS,
    // it passes transcript_tx to deepgram_transcript_service(), so it can send the transcript back
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

    // loop to get to the transcript output and pass to the transcript_post_processing_service()
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
                        .send(TnServiceResponseMsg {
                            response: "transcript_fragment".to_string(),
                            payload: TnAsset::String(trx_fragment),
                        })
                        .await;
                }
                if speech_final {
                    let transcript = transcript_fragments.join(" ").trim().to_string();
                    if !transcript.is_empty() {
                        let _ = tx
                            .send(TnServiceResponseMsg {
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
                        .send(TnServiceResponseMsg {
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
    context: TnContext,
    mut response_rx: Receiver<TnServiceResponseMsg>,
) {
    let assets = context.read().await.asset.clone();
    let components = context.read().await.components.clone();
    let transcript_area_id = context.clone().read().await.get_component_id("transcript");
    while let Some(response) = response_rx.recv().await {
        match response.response.as_str() {
            "transcript_final" => {
                if let TnAsset::String(transcript) = response.payload {
                    {
                        append_and_send_stream_textarea_with_context(
                            context.clone(),
                            "status",
                            &format!("ASR output: {transcript}\n"),
                        )
                        .await;

                        let llm_tx = context
                            .clone()
                            .read()
                            .await
                            .services
                            .get("llm_service")
                            .unwrap()
                            .0
                            .clone();

                        let (tx, rx) = oneshot::channel::<String>();

                        let llm_req_msg = TnServiceRequestMsg {
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
                                ("user".into(), transcript.clone()),
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
                            let sse_tx = context.get_sse_tx_with_context().await;
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

                        append_and_send_stream_textarea_with_context(
                            context.clone(),
                            "status",
                            &format!("trx fragment: {transcript}\n"),
                        )
                        .await;
                    }
                    // maybe show the interim transcription results somewhere
                }
            }
            _ => {}
        }
    }
}
