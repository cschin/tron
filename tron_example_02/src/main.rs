mod dg_service;
mod open_ai_service;
use open_ai_service::simulate_dialog;

use askama::Template;
use dg_service::{deepgram_transcript_service, DeepgramError, StreamResponse};
use futures_util::Future;

use axum::body::Bytes;
use data_encoding::BASE64;
use serde::Deserialize;

use bytes::{BufMut, BytesMut};
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
use tracing::debug;
use tron_app::{
    utils::send_sse_msg_to_client, SseAudioRecorderTriggerMsg, SseTriggerMsg, TriggerData,
};
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

    tron_app::run(app_share_data).await
}

fn build_session_context() -> Arc<RwLock<Context<'static>>> {
    let mut context = Context::<'static>::default();
    let mut component_id = 0_u32;
    let mut btn = TnButton::new(component_id, "rec_button".into(), "Start Conversation".into());
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
    let mut transcript_output = TnTextArea::<'static>::new(
        component_id,
        "transcript".to_string(),
        "<< ".to_string(),
    );
    transcript_output.set_attribute(
        "class".to_string(),
        "textarea textarea-bordered flex-1 min-h-80v".to_string(),
    );
    transcript_output.set_attribute(
        "hx-swap".into(),
        "outerHTML scroll:bottom focus-scroll:true".into(),
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
    let transcript = context_guard.render_to_string("transcript");
    let html = AppPageTemplate {
        btn,
        recorder,
        player,
        transcript,
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
        (ActionExecutionMethod::Await, Arc::new(stop_audio_playing)),
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
        let sse_tx = get_sse_tx(context.clone()).await;
        {
            let context_guard = context.read().await;
            let rec_button_id = context_guard.get_component_id(&event.e_target);
            let mut components_guard = context_guard.components.write().await;
            let rec_button = components_guard.get_mut(&rec_button_id).unwrap();
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
                            context_set_value_for(
                                &context,
                                "recorder",
                                ComponentValue::String("Paused".into()),
                            )
                            .await;
                            context_set_state_for(&context, "recorder", ComponentState::Updating)
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
                            if let Ok(_out) = rx.await {
                                //println!("returned string: {}", out);
                            };
                        }

                        // // write the audio to file.
                        // {
                        //     let context_guard = context.read().await;
                        //     let components_guard = context_guard.components.read().await;
                        //     let recorder_id = context_guard.get_component_id("recorder");
                        //     let recorder = components_guard.get(&recorder_id).unwrap().as_ref();
                        //     audio_recorder::write_audio_data_to_file(recorder);
                        // }

                        // {
                        //     let context_guard = context.write().await;
                        //     let mut components_guard = context_guard.components.write().await;
                        //     let player_id = context_guard.get_component_id("player");
                        //     let player = components_guard.get_mut(&player_id).unwrap();
                        //     player.set_attribute("autoplay".into(), "true".into());
                        //     player.set_attribute("src".into(), "/tron_streaming/player".into());
                        //     player.set_state(ComponentState::Updating);
                        // }

                        // let msg = SseTriggerMsg {
                        //     server_side_trigger: TriggerData {
                        //         target: "player".into(),
                        //         new_state: "updating".into(),
                        //     },
                        // };
                        // send_sse_msg_to_client(&sse_tx, msg).await;
                    }
                    "Start Conversation" => {
                        context_set_value_for(
                            &context,
                            "recorder",
                            ComponentValue::String("Recording".into()),
                        )
                        .await;
                        context_set_state_for(&context, "recorder", ComponentState::Updating).await;

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
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let chunk: Option<AudioChunk> = serde_json::from_value(payload).unwrap_or(None);
        if let Some(chunk) = chunk {
            // println!(
            //     "stream, audio data received, len={}",
            //     chunk.audio_data.len()
            // );

            let id = {
                let context_guard = context.read().await;
                *context_guard.tron_id_to_id.get(&event.e_target).unwrap()
            };

            let b64data = chunk.audio_data.trim_end_matches('"');
            let mut split = b64data.split(',');
            let _head = split.next().unwrap();
            let b64str = split.next().unwrap();
            let chunk = Bytes::from(BASE64.decode(b64str.as_bytes()).unwrap());

            {
                let context_guard = context.read().await;
                let mut component_guard = context_guard.components.write().await;
                let recorder = component_guard.get_mut(&id).unwrap();
                audio_recorder::append_audio_data(recorder, chunk.clone());
            }
            // {
            //     let context_guard = context.write().await;
            //     let mut stream_data_guard = context_guard.stream_data.write().await;
            //     let player_data = stream_data_guard.get_mut("player").unwrap();
            //     let mut data = BytesMut::new();
            //     data.put(&chunk[..]);
            //     player_data.1.push_back(data);
            // }
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
                if let Ok(_out) = rx.await {
                    // println!("returned string: {}", out);
                };
            }
        };
    };

    Box::pin(f)
}

async fn get_sse_tx(context: Arc<RwLock<Context<'static>>>) -> Sender<String> {
    // unlock two layers of Rwlock !!
    let sse_tx = context
        .read()
        .await
        .sse_channels
        .read()
        .await
        .as_ref()
        .unwrap()
        .tx
        .clone();
    sse_tx
}

fn stop_audio_playing(
    context: Arc<RwLock<Context<'static>>>,
    _event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        {
            let context_guard = context.write().await;
            let mut components_guard = context_guard.components.write().await;
            let player_id = context_guard.get_component_id("player");
            let player = components_guard.get_mut(&player_id).unwrap();
            player.set_header("HX-Reswap".into(), "none".into()); 
            player.set_state(ComponentState::Ready);
        }
        {
            let sse_tx = get_sse_tx(context.clone()).await;
            let msg = SseTriggerMsg {
                server_side_trigger: TriggerData {
                    target: "player".into(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    };
    Box::pin(f)
}

async fn transcript_service(
    mut rx: Receiver<ServiceRequestMessage>,
    tx: Sender<ServiceResponseMessage>,
) {
    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<StreamResponse>(1);
    tokio::spawn(async move {
        //println!("restart dg_trx");
        let (mut audio_tx, audio_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(1);
        let mut handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        while let Some(req) = rx.recv().await {
            println!("req: {}", req.request);
            if handle.is_finished() {
                audio_tx.closed().await;
                let (audio_tx0, audio_rx) =
                    tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(1);
                audio_tx = audio_tx0;
                handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }
            if let TnAsset::VecU8(payload) = req.payload {
                //println!("req received: {}, payload: {}", req.request, payload.len());
                let _ = audio_tx.send(Ok(Bytes::from_iter(payload))).await;
                let _ = req.response.send("audio sent to trx service".to_string());
            }
        }
        audio_tx.closed().await;
    });

    let mut transcript_fragments = Vec::<String>::new();
    while let Some(trx_rtn) = transcript_rx.recv().await {
        //println!("trx_rtn: {:?}", trx_rtn);
        //let transcript = serde_json::to_string(&trx_rtn).unwrap();
        match trx_rtn {
            StreamResponse::TerminalResponse {
                request_id,
                created,
                duration,
                channels,
            } => {}
            StreamResponse::TranscriptResponse {
                duration,
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
            StreamResponse::UtteranceEnd { last_word_end } => {
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
    let sse_tx = context.read().await.sse_channels.clone();
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
                            payload: TnAsset::String(transcript),
                            response: tx,
                        };
                        let _ = llm_tx.send(llm_req_msg).await;
                        if let Ok(_out) = rx.await {
                            //println!("returned string: {}", out);
                        };
                    }
                    {
                        let mut components_guard = components.write().await;
                        let transcript_area =
                            components_guard.get_mut(&transcript_area_id).unwrap();
                        text::append_textarea_value(transcript_area, " <END>\n", None);
                    }
                    {
                        let msg = SseTriggerMsg {
                            server_side_trigger: TriggerData {
                                target: "transcript".into(),
                                new_state: "ready".into(),
                            },
                        };
                        let sse_tx_guard = sse_tx.read().await;
                        let sse_tx = sse_tx_guard.as_ref().unwrap().tx.clone();
                        send_sse_msg_to_client(&sse_tx, msg).await;
                    }
                    // {
                    //     // toggle the rec_button to stop recording
                    //     let msg = SseTriggerMsg {
                    //         server_side_trigger: TriggerData {
                    //             target: "rec_button".into(),
                    //             new_state: "ready".into(),
                    //         },
                    //     };
                    //     let sse_tx_guard = sse_tx.read().await;
                    //     let sse_tx = sse_tx_guard.as_ref().unwrap().tx.clone();
                    //     send_sse_msg_to_client(&sse_tx, msg).await;
                    // }
                }
            }
            "transcript_fragment" => {
                if let TnAsset::String(transcript) = response.payload {
                    if !transcript.is_empty() {
                        let mut assets_guard = assets.write().await;
                        let e = assets_guard.entry("transcript".into()).or_default();
                        (*e).push(TnAsset::String(transcript.clone()));
                        {
                            let mut components_guard = components.write().await;
                            let transcript_area =
                                components_guard.get_mut(&transcript_area_id).unwrap();
                            text::append_textarea_value(transcript_area, &transcript, Some(" "));
                        }
                        {
                            let msg = SseTriggerMsg {
                                server_side_trigger: TriggerData {
                                    target: "transcript".into(),
                                    new_state: "ready".into(),
                                },
                            };
                            let sse_tx_guard = sse_tx.read().await;
                            let sse_tx = sse_tx_guard.as_ref().unwrap().tx.clone();
                            send_sse_msg_to_client(&sse_tx, msg).await;
                        }
                    }
                }
            }
            _ => {}
        }
    }
}
