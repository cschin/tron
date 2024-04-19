use askama::Template;
use futures_util::Future;

use axum::body::Bytes;
use serde::Deserialize;
//use serde::{Deserialize, Serialize};
use base64::prelude::*;
use bytes::{BufMut, BytesMut};
use serde_json::Value;
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
//use std::sync::Mutex;
use futures_util::StreamExt;
use lazy_static::lazy_static;
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
};
use tokio::sync::oneshot;
use tokio_stream::wrappers::ReceiverStream;

lazy_static! {
    static ref DG: deepgram::Deepgram =
        deepgram::Deepgram::new(std::env::var("DG_API_KEY").unwrap());
}

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
        let (request_tx, request_rx) = tokio::sync::mpsc::channel::<ServiceRequestMessage>(1);
        let (response_tx, mut response_rx) =
            tokio::sync::mpsc::channel::<ServiceResponseMessage>(1);
        context.services.insert(
            "transcript_service".into(),
            (request_tx.clone(), Mutex::new(None)),
        );
        //services_guide.insert("transcript_service".into(), tx.clone());
        tokio::task::spawn(transcript_service(request_rx, response_tx));
        let assets = context.assets.clone();
        tokio::task::spawn(async move {
            while let Some(response) = response_rx.recv().await {
                if let TnAsset::String(transcript) = response.payload {
                    let mut assets_guard = assets.write().await;
                    let e = assets_guard.entry("transcript".into()).or_default();
                    println!("recorded transcript: {:?}", transcript);
                    (*e).push(TnAsset::String(transcript));
                }
            }
        });
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
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        let previous_rec_button_value;
        let sse_tx = {
            let context_guard = context.read().await;
            context_guard.sse_channels.tx.clone()
        };

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
                            send_sse_msg_to_client(&sse_tx, msg).await;
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
                        send_sse_msg_to_client(&sse_tx, msg).await;
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
                let (trx_srv, _) = context_guard.services.get("transcript_service").unwrap();
                let (tx, rx) = oneshot::channel::<String>();
                let trx_req_msg = ServiceRequestMessage {
                    request: "sending audio:".into(),
                    payload: TnAsset::VecU8(chunk.to_vec()),
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

async fn transcript_service(
    mut rx: Receiver<ServiceRequestMessage>,
    tx: Sender<ServiceResponseMessage>,
) {

    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<String>(1);
    tokio::spawn(async move {
        loop {
            println!("restart dg_trx");
            let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<Result<Bytes, axum::Error>>(1);
            let handle = tokio::spawn(dg_trx(audio_rx, transcript_tx.clone()));
            tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            while let Some(req) = rx.recv().await {
                if let TnAsset::VecU8(payload) = req.payload {
                    println!("req received: {}, payload: {}", req.request, payload.len());
                    let _ = audio_tx.send(Ok(Bytes::from_iter(payload))).await;
                    let _ = req.response.send("audio sent to trx service".to_string());
                }
                if handle.is_finished() {
                    break;
                }
            }
        }
    });
    while let Some(trx_rtn) = transcript_rx.recv().await {
        println!("trx_rtn: {}", trx_rtn);
        let _ = tx
            .send(ServiceResponseMessage {
                response: "response".to_string(),
                payload: TnAsset::String(trx_rtn),
            })
            .await;
    }
}

use deepgram::transcription::live::StreamResponse::{TerminalResponse, TranscriptResponse};
async fn dg_trx(
    audio_rx: Receiver<Result<Bytes, axum::Error>>,
    transcript_tx: Sender<String>,
) -> Result<(), deepgram::DeepgramError> {
    let data_stream = ReceiverStream::new(audio_rx);
    //let DG: deepgram::Deepgram =
    //    deepgram::Deepgram::new(std::env::var("DG_API_KEY").unwrap());
    let dg_trx = DG.transcription();

    let mut results = dg_trx
        .stream_request()
        .stream(data_stream)
        .start()
        .await
        .unwrap();
    loop {
        if let Some(result) = results.next().await {
            match result {
                Ok(r) => match r {
                    TranscriptResponse {
                        duration,
                        is_final,
                        speech_final,
                        channel,
                    } => {
                        println!("duration: {}", duration);
                        println!("is_final; {}", is_final);
                        println!("speech_final: {}", speech_final);
                        let transcript =
                            serde_json::to_string(&(duration, is_final, speech_final, channel))
                                .unwrap();
                        transcript_tx
                            .send(transcript)
                            .await
                            .expect("transcript send fail");
                    }
                    TerminalResponse {
                        request_id,
                        created,
                        duration,
                        channels,
                    } => {
                        println!("terminal request_id {:?}", request_id);
                        println!("terminal created {:?}", created);
                        println!("terminal duration {:?}", duration);
                        println!("terminal channels {:?}", channels);
                        break;
                    }
                },
                Err(e) => {
                    println!("Err {:?}", e);
                    return Err(e);
                }
            }
        } else {
            println!("session listener loop end, last results:: {:?}", results);
            break;
        }
    }

    println!("session listener final end");

    Ok(())
}
