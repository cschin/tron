mod dg_service;
mod llm_service;
use http::HeaderMap;
use llm_service::simulate_dialog;

use askama::Template;
use dg_service::{deepgram_transcript_service, tts_service, DeepgramError, StreamResponse};
use futures_util::Future;

use axum::{body::Bytes, response::Html};
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
use tron_app::tron_components;
use tron_app::{send_sse_msg_to_client, TnServerSideTriggerData, TnSseTriggerMsg};
use tron_components::audio_recorder::SseAudioRecorderTriggerMsg;
use tron_components::{text::append_and_update_stream_textarea_with_context, *};

#[global_allocator]
static GLOBAL: tikv_jemallocator::Jemalloc = tikv_jemallocator::Jemalloc;

static RECORDING_BUTTON: &str = "rec_button";
static RECORDER: &str = "recorder";
static PLAYER: &str = "player";
static RESET_BUTTON: &str = "reset_button";
static TRANSCRIPT_OUTPUT: &str = "transcript_output";
static LLM_STREAM_OUTPUT: &str = "llm_stream_output";
static STATUS: &str = "status";
static PROMPT: &str = "prompt";
static TTS_MODEL_SELECT: &str = "tts_model_select";
static PRESET_PROMPT_SELECT: &str = "preset_prompt_select";
static TRANSCRIPT_SERVICE: &str = "transcript_service";
static LLM_SERVICE: &str = "llm_service";
static TTS_SERVICE: &str = "tts_service";
static TRON_APP: &str = "tron_app";

#[tokio::main]
async fn main() {
    // Configure the application
    let app_configure = tron_app::AppConfigure {
        log_level: Some("server=info,tower_http=info,tron_app=info"),
        cognito_login: true,
        session_expiry: Some(time::Duration::seconds_f32(600.0)),
        ..Default::default()
    };
    // set app state
    let app_share_data = tron_app::AppData::builder(build_session_context, layout).build(); 
    // Run the application
    //tron_app::run(app_share_data, Some("server=debug,tower_http=debug,tron_app=info")).await
    tron_app::run(app_share_data, app_configure).await
}

/// Builds and initializes the session context for the application.
///
/// This function sets up the components, assets, and services required for the application to run.
/// It creates and configures various UI components such as buttons, audio recorder, audio player,
/// chatbox, text areas, and select dropdowns. It also loads and sets up prompt templates and
/// initializes services for speech transcription, language model interaction, and text-to-speech
/// generation.
///
/// Returns:
///     A `TnContext` instance containing the initialized application context.
fn build_session_context() -> TnContext {
    let mut context = TnContextBase::<'static>::default();

    {
        // add a recorder button
        let btn = TnButton::builder()
            .init(
                RECORDING_BUTTON.into(),
                "Start Conversation".into(),
            )
            .set_attribute(
                "class".to_string(),
                "btn btn-sm btn-outline btn-primary flex-1".to_string(),
            )
            .set_action(TnActionExecutionMethod::Await, toggle_recording)
            .build();
        context.add_component(btn);
    }
    {
        // add a recorder
        let recorder = TnAudioRecorder::builder()
            .init(
                RECORDER.to_string(),
                "Paused".to_string(),
            )
            .set_action(
                TnActionExecutionMethod::Await,
                audio_input_stream_processing,
            )
            .build();
        context.add_component(recorder);
    }
    {
        // add a player
        let player = TnAudioPlayer::builder()
            .init(
                PLAYER.to_string(),
                "Paused".to_string(),
            )
            .set_attribute("class".to_string(), "flex-1 p-1 h-10".to_string())
            .set_action(
                TnActionExecutionMethod::Await,
                audio_player::stop_audio_playing_action,
            )
            .build();
        context.add_component(player);
    }

    {
        // add a reset button
        let btn = TnButton::builder()
            .init(
                RESET_BUTTON.into(),
                "Reset The Conversation".into(),
            )
            .set_attribute(
                "class".to_string(),
                "btn btn-sm btn-outline btn-primary flex-1".to_string(),
            )
            .set_action(TnActionExecutionMethod::Await, reset_conversation)
            .build();
        context.add_component(btn);
    }
    {
        // add a chatbox
        let transcript_output = TnChatBox::builder()
            .init(TRANSCRIPT_OUTPUT.to_string(), vec![])
            .set_attribute(
                "class".to_string(),
                "flex flex-col overflow-auto flex-1 p-2".to_string(),
            )
            .build();

        context.add_component(transcript_output);
    }
    {
        // add a textarea showing partial stream content
        let llm_stream_output = TnStreamTextArea::builder()
            .init(
                LLM_STREAM_OUTPUT.to_string(),
                Vec::new(),
            )
            .set_attribute(
                "class".to_string(),
                "overflow-auto flex-1 p-2 h-19 max-h-19 min-h-19".to_string(),
            )
            .build();
        context.add_component(llm_stream_output);
    }

    {
        // add a status box
        let status_output = TnStreamTextArea::builder()
            .init(STATUS.to_string(), Vec::new())
            .set_attribute(
                "class".to_string(),
                "flex-1 p-2 textarea textarea-bordered h-40 max-h-40 min-h-40".to_string(),
            )
            .set_attribute("hx-trigger".into(), "server_side_trigger".into())
            .build();

        context.add_component(status_output);
    }

    {
        let prompt = include_str!("../templates/drunk-bioinformatist.txt");
        let mut prompt_box =
            TnTextArea::builder().init(PROMPT.into(), prompt.into())
                .set_attribute("hx-trigger".into(), "change, server_side_trigger".into()) // change will update the value one the textarea is out of focus
                .set_attribute(
                    "class".into(),
                    "flex-1 p-2 textarea textarea-bordered mx-auto h-96 max-h-96 min-h-96".into(),
                )
                .set_attribute(
                    "hx-vals".into(),
                    r##"js:{event_data:get_input_event(event)}"##.into(),
                )
                .build();
        prompt_box.remove_attribute("disabled".into());

        context.add_component(prompt_box);
    }

    {
        let default_model = "aura-arcas-en".into();
        let model_options = vec![
            ("aura-asteria-en".into(), "Asteria (F)".into()),
            ("aura-luna-en".into(), "Luna (F)".into()),
            ("aura-stella-en".into(), "Stella (F)".into()),
            ("aura-hera-en".into(), "Hera (F)".into()),
            ("aura-orion-en".into(), "Orion (M)".into()),
            ("aura-arcas-en".into(), "Arcas (M)".into()),
            ("aura-perseus-en".into(), "Perseus (M)".into()),
            ("aura-angus-en".into(), "Angus (M)".into()),
            ("aura-orpheus-en".into(), "Orpheus (M)".into()),
            ("aura-helios-en".into(), "Helios (M)".into()),
            ("aura-zeus-en".into(), "Zeus (M)".into()),
        ];
        let tts_model_select = TnSelect::builder().init(
            TTS_MODEL_SELECT.into(),
            default_model,
            model_options,
        )
        .set_attribute(
            "class".into(),
            "select select-bordered w-full max-w-xs".into(),
        )
        .build();
        context.add_component(tts_model_select);
    }

    {
        let default_prompt = "drunk-bioinformatist".into();
        let prompt_options = vec![
            ("drunk-bioinformatist".into(), "Drunk Bioinformatist".into()),
            ("socrat".into(), "Socrat".into()),
            ("poet".into(), "Poet".into()),
        ];
        let preset_prompt_select = TnSelect::builder().init(
            PRESET_PROMPT_SELECT.into(),
            default_prompt,
            prompt_options,
        )
        .set_attribute(
            "class".into(),
            "select select-bordered w-full max-w-xs".into(),
        )
        .set_action(TnActionExecutionMethod::Await, preset_prompt_select_change)
        .build();

        context.add_component(preset_prompt_select);

        let mut prompts = HashMap::<String, String>::default();
        let prompt = include_str!("../templates/drunk-bioinformatist.txt");
        prompts.insert("drunk-bioinformatist".into(), prompt.into());
        let prompt = include_str!("../templates/Socrat.txt");
        prompts.insert("socrat".into(), prompt.into());
        let prompt = include_str!("../templates/poet.txt");
        prompts.insert("poet".into(), prompt.into());
        let prompts = TnAsset::HashMapString(prompts);
        context
            .assets
            .blocking_write()
            .insert("prompts".into(), prompts);
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
            TRANSCRIPT_SERVICE.into(),
            (transcript_request_tx.clone(), Mutex::new(None)),
        );
        // service sending audio stream to deepgram
        let transcript_service = tokio::task::spawn(transcript_service(
            context.clone(),
            transcript_request_rx,
            transcript_response_tx,
        ));

        context
            .blocking_write()
            .service_handles
            .push(transcript_service);

        // service processing the output from deepgram
        let transcript_post_processing_service = tokio::task::spawn(
            transcript_post_processing_service(context.clone(), transcript_response_rx),
        );

        context
            .blocking_write()
            .service_handles
            .push(transcript_post_processing_service);

        // service handling the LLM and TTS at once
        let (llm_request_tx, llm_request_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(1);
        context.blocking_write().services.insert(
            LLM_SERVICE.into(),
            (llm_request_tx.clone(), Mutex::new(None)),
        );
        let simulate_dialog = tokio::task::spawn(simulate_dialog(context.clone(), llm_request_rx));

        context
            .blocking_write()
            .service_handles
            .push(simulate_dialog);

        let (tts_tx, tts_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(32);
        context
            .blocking_write()
            .services
            .insert(TTS_SERVICE.into(), (tts_tx.clone(), Mutex::new(None)));
        let tts_service = tokio::spawn(tts_service(tts_rx, context.clone()));
        context.blocking_write().service_handles.push(tts_service);
    }

    {
        // set up audio stream out
        let context_guard = context.blocking_write();
        let mut stream_data_guard = context_guard.stream_data.blocking_write();
        stream_data_guard.insert(PLAYER.into(), ("audio/mp3".into(), VecDeque::default()));
    }

    context
}

/// Struct representing the HTML template for the application page.
/// It contains fields for various components like buttons, recorder, player,
/// chatbox, text areas, and select dropdowns.
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
    tts_model_select: String,
    llm_stream_output: String,
    preset_prompt_select: String,
}

/// Renders the HTML layout for the application page.
///
/// This function sets the initial values and states for the RECORDER and RECORDING_BUTTON components.
/// It then retrieves the rendered HTML strings for various components and constructs an
/// `AppPageTemplate` struct with these strings. Finally, it renders the template to produce the
/// HTML layout for the application page.
///
/// Returns:
///     A `String` containing the rendered HTML layout for the application page.
fn layout(context: TnContext) -> String {
    context.set_value_for_component_blocking(RECORDER, TnComponentValue::String("Paused".into()));
    context.set_state_for_component_blocking(RECORDER, TnComponentState::Ready);

    context.set_value_for_component_blocking(
        RECORDING_BUTTON,
        TnComponentValue::String("Start Conversation".into()),
    );
    context.set_state_for_component_blocking(RECORDING_BUTTON, TnComponentState::Ready);

    let guard = context.blocking_read();
    let btn = guard.render_to_string(RECORDING_BUTTON);
    let recorder = guard.render_to_string(RECORDER);
    let player = guard.render_to_string(PLAYER);
    let transcript = guard.first_render_to_string(TRANSCRIPT_OUTPUT);
    let status = guard.first_render_to_string(STATUS);
    let prompt = guard.first_render_to_string(PROMPT);
    let reset_button = guard.first_render_to_string(RESET_BUTTON);
    let tts_model_select = guard.first_render_to_string(TTS_MODEL_SELECT);
    let llm_stream_output = guard.first_render_to_string(LLM_STREAM_OUTPUT);
    let preset_prompt_select = guard.first_render_to_string(PRESET_PROMPT_SELECT);

    let html = AppPageTemplate {
        btn,
        recorder,
        player,
        transcript,
        status,
        prompt,
        reset_button,
        tts_model_select,
        llm_stream_output,
        preset_prompt_select,
    };
    html.render().unwrap()
}

fn _do_nothing(
    context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        //let comp = context.get_component(&event.e_target);
        tracing::info!(target: TRON_APP, "{:?}", payload);
        context
            .set_state_for_component(&event.e_trigger.clone(), TnComponentState::Pending)
            .await;
        let sse_tx = context.get_sse_tx().await;
        let msg = TnSseTriggerMsg {
            server_side_trigger_data: TnServerSideTriggerData {
                target: event.e_trigger.clone(),
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&sse_tx, msg).await;
        let comp = context.get_component(&event.e_trigger).await;
        tracing::info!(target: TRON_APP, "value: {:?}", comp.read().await.value());
    };
    Box::pin(f)
}

/// Handles the change event for the preset prompt select dropdown.
///
/// This function retrieves the selected value from the PRESET_PROMPT_SELECT component and uses
/// it to fetch the corresponding prompt from the application's asset store. It then sets the
/// fetched prompt as the value of the PROMPT text area component and marks both the PROMPT
/// and PRESET_PROMPT_SELECT components as ready.
///
/// Returns:
///     A `TnHtmlResponse` containing the rendered HTML for the component that triggered the event.
///     If the event type is not "change" or the component state is not "ready", it returns `None`.
fn preset_prompt_select_change(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        if event.e_type != "change" || event.e_state != "ready" {
            return None;
        }
        let value = context.get_value_from_component(PRESET_PROMPT_SELECT).await;
        tracing::info!(target: TRON_APP, "preset_prompt_select_change, value:{:?}", value);
        let s = if let TnComponentValue::String(s) = value {
            s
        } else {
            "".into()
        };
        let prompt = if let Some(m) = context.read().await.assets.read().await.get("prompts") {
            if let TnAsset::HashMapString(ref m) = m {
                m.get(&s).unwrap().clone()
            } else {
                "".to_string()
            }
        } else {
            "".into()
        };
        // tracing::info!(target: TRON_APP, "preset_prompt_select_change, prompt:{}", prompt);
        context
            .set_value_for_component(PROMPT, TnComponentValue::String(prompt))
            .await;
        context.set_ready_for(PROMPT).await;
        context.set_ready_for(PRESET_PROMPT_SELECT).await;

        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f)
}

/// Handles the reset conversation action.
///
/// This function is triggered when the RESET_BUTTON component is clicked and in the "ready" state.
/// It sends a "clear-history" request to the LLM_SERVICE to clear the conversation history.
/// Then, it cleans the chatbox component with the ID TRANSCRIPT and sets the RESET_BUTTON component to the "ready" state.
///
/// Returns:
///     A `TnHtmlResponse` containing the rendered HTML for the component that triggered the event.
///     If the event type is not "click" or the component state is not "ready", it returns `None`.
fn reset_conversation(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        {
            if event.e_type != "click" || event.e_state != "ready" {
                return None;
            }
            // send message to the llm service to clean up the history

            let llm_tx = context.get_service_tx(LLM_SERVICE).await;
            let (tx, mut rx) = oneshot::channel::<String>();

            let llm_req_msg = TnServiceRequestMsg {
                request: "clear-history".into(),
                payload: TnAsset::String("".into()),
                response: tx,
            };

            let _ = llm_tx.send(llm_req_msg).await;
            let _ = rx.try_recv();
        }

        chatbox::clean_chatbox_with_context(&context, TRANSCRIPT_OUTPUT).await;
        context.set_ready_for(RESET_BUTTON).await;

        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f)
}

/// Toggles the recording state of the audio recorder component.
///
/// This function is triggered when the RECORDING_BUTTON component is clicked or a server-side trigger
/// event occurs. It handles the start and stop of the audio recording process.
///
/// When the recording is started, it sets the RECORDER component's value to "Pending" and its
/// state to "Updating". It then clears the audio stream buffer and sends a server-sent event (SSE)
/// to start the audio recording on the client-side.
///
/// When the recording is stopped, it sets the RECORDER component's value to "Paused" and sends
/// an SSE to stop the audio recording on the client-side. It also sends a request to the
/// TRANSCRIPT_SERVICE to stop the transcription process.
///
/// After handling the start or stop of the recording, it sets the RECORDING_BUTTON component's state
/// to "ready" and renders the updated component HTML.
///
/// Returns:
///     A `TnHtmlResponse` containing the rendered HTML for the component that triggered the event.
///     If the event type is not "click" or "server_side_trigger", it returns `None`.
fn toggle_recording(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        if event.e_type != "click" && event.e_type != "server_side_trigger" {
            return None;
        }
        let previous_rec_button_value;
        let sse_tx = context.get_sse_tx().await;
        {
            let rec_button = context.get_component(RECORDING_BUTTON).await;
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
                                    RECORDER,
                                    TnComponentValue::String("Paused".into()),
                                )
                                .await;
                            context
                                .set_state_for_component(RECORDER, TnComponentState::Updating)
                                .await;
                            let mut delay =
                                tokio::time::interval(tokio::time::Duration::from_millis(300));
                            delay.tick().await; //The first tick completes immediately.
                            delay.tick().await; //wait a bit for all data stream transferred
                            let msg = SseAudioRecorderTriggerMsg {
                                server_side_trigger_data: TnServerSideTriggerData {
                                    target: RECORDER.into(),
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
                                context_guard.services.get(TRANSCRIPT_SERVICE).unwrap();
                            let (tx, rx) = oneshot::channel::<String>();
                            let trx_req_msg = TnServiceRequestMsg {
                                request: "stop".into(),
                                payload: TnAsset::VecU8(vec![]),
                                response: tx,
                            };
                            let _ = trx_srv.send(trx_req_msg).await;
                            if let Ok(out) = rx.await {
                                tracing::debug!(target:TRON_APP, "returned string: {}", out);
                            };
                        }
                    }

                    "Start Conversation" => {
                        context
                            .set_value_for_component(
                                RECORDER,
                                TnComponentValue::String("Pending".into()),
                            )
                            .await;

                        context
                            .set_state_for_component(RECORDER, TnComponentState::Updating)
                            .await;

                        context.set_ready_for(PLAYER).await; // rest the player state to ready

                        {
                            let context_guard = context.write().await;
                            let mut stream_data_guard = context_guard.stream_data.write().await;
                            // clear the stream buffer
                            stream_data_guard.get_mut(PLAYER).unwrap().1.clear();
                        }

                        let msg = SseAudioRecorderTriggerMsg {
                            server_side_trigger_data: TnServerSideTriggerData {
                                target: RECORDER.into(),
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

        let msg = TnSseTriggerMsg {
            server_side_trigger_data: TnServerSideTriggerData {
                target: event.e_trigger,
                new_state: "ready".into(),
            },
        };
        send_sse_msg_to_client(&sse_tx, msg).await;
        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };

    Box::pin(f)
}

#[derive(Clone, Debug, Deserialize)]
struct AudioChunk {
    audio_data: String,
}

/// Handles the processing of audio input stream data.
///
/// This function is triggered when an audio input stream event occurs with the event type
/// "streaming" and the state "updating". It deserializes the payload data into an `AudioChunk`
/// struct and processes the audio data.
///
/// It appends the audio data to the RECORDER component, updates the value and state of the
/// RECORDER component as needed, and sends the audio data to the TRANSCRIPT_SERVICE for
/// transcription.
///
/// Additionally, it logs a message to the STATUS component with the chunk length of the audio
/// data, sampled at a rate of 1/10.
///
/// Returns:
///     A `TnHtmlResponse` containing the rendered HTML for the component that triggered the event.
///     If the event type is not "streaming" or the component state is not "updating", it returns `None`.
fn audio_input_stream_processing(
    context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        if event.e_type != "streaming" || event.e_state != "updating" {
            return None;
        }

        let event_data = payload
            .as_object()
            .unwrap()
            .get("event_data")
            .unwrap_or(&Value::Null);

        let chunk: Option<AudioChunk> = serde_json::from_value(event_data.clone()).unwrap_or(None);

        if let Some(chunk) = chunk {
            let b64data = chunk.audio_data.trim_end_matches('"');
            let mut split = b64data.split(',');
            let _head = split.next().unwrap();
            let b64str = split.next().unwrap();
            let chunk = Bytes::from(BASE64.decode(b64str.as_bytes()).unwrap());
            let chunk_len = chunk.len();

            {
                let recorder = context.get_component(RECORDER).await;
                audio_recorder::append_audio_data(recorder.clone(), chunk.clone()).await;

                let recorder_value = context.get_value_from_component(RECORDER).await;
                if let TnComponentValue::String(s) = recorder_value {
                    if s.as_str() == "Pending" {
                        context
                            .set_value_for_component(
                                RECORDER,
                                TnComponentValue::String("Recording".into()),
                            )
                            .await;

                        context
                            .set_state_for_component(RECORDER, TnComponentState::Updating)
                            .await;

                        let msg = TnSseTriggerMsg {
                            server_side_trigger_data: TnServerSideTriggerData {
                                target: RECORDER.into(),
                                new_state: "ready".into(),
                            },
                        };
                        let sse_tx = context.get_sse_tx().await;
                        send_sse_msg_to_client(&sse_tx, msg).await;
                    }
                }
            }

            {
                let context_guard = context.read().await;
                let (trx_srv, _) = context_guard.services.get(TRANSCRIPT_SERVICE).unwrap();
                let (tx, rx) = oneshot::channel::<String>();
                let trx_req_msg = TnServiceRequestMsg {
                    request: "sending audio".into(),
                    payload: TnAsset::VecU8(chunk.to_vec()),
                    response: tx,
                };
                let _ = trx_srv.send(trx_req_msg).await;
                if let Ok(out) = rx.await {
                    tracing::debug!(target: TRON_APP, "sending audio, returned string: {}", out );
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
                    append_and_update_stream_textarea_with_context(
                        &context,
                        STATUS,
                        &format!("recording audio, chunk length: {chunk_len}\n"),
                    )
                    .await;
                }
            }
        };
        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };

    Box::pin(f)
}

/// Handles the transcription service for the application.
///
/// This function receives audio data from the client and sends it to the DeepGram transcription
/// service. It then processes the transcription responses received from DeepGram and sends the
/// final transcripts to the `transcript_post_processing_service`.
///
/// It spawns a separate task to maintain the connection with the DeepGram WebSocket and handle
/// incoming audio data. If the connection with DeepGram is lost, it re-establishes the connection.
///
/// The function continuously listens for transcription responses from DeepGram and processes them
/// accordingly. It accumulates transcript fragments until a final transcript is received or an
/// utterance ends, at which point it sends the final transcript to the
/// `transcript_post_processing_service`.
///
/// Args:
///     rx: A `Receiver` for receiving `TnServiceRequestMsg` messages containing audio data.
///     tx: A `Sender` for sending `TnServiceResponseMsg` messages containing transcripts.
async fn transcript_service(
    context: TnContext,
    mut rx: Receiver<TnServiceRequestMsg>,
    tx: Sender<TnServiceResponseMsg>,
) {
    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<StreamResponse>(16);

    // start a loop for maintaining connection with DG, this calls the DG WS,
    // it passes transcript_tx to deepgram_transcript_service(), so it can send the transcript back
    tokio::spawn(async move {
        tracing::debug!(target: TRON_APP, "restart dg_trx");

        let (mut audio_tx, audio_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(16);

        let mut handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        while let Some(req) = rx.recv().await {
            tracing::debug!(target: TRON_APP, "req: {}", req.request);

            if handle.is_finished() {
                audio_tx.closed().await;
                let (audio_tx0, audio_rx) =
                    tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(16);
                audio_tx = audio_tx0;
                handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));
                tokio::time::sleep(std::time::Duration::from_secs(1)).await;
            }

            if let TnAsset::VecU8(payload) = req.payload {
                tracing::debug!(target: TRON_APP, "req received: {}, payload: {}", req.request, payload.len());
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
                        let player_state = {
                            let player = context.get_component(PLAYER).await;
                            let player_guard = player.read().await;
                            let player_state = player_guard.state();
                            tracing::info!(target:"tron_app", "player state: {:?}", player_state);
                            player_state.clone()
                        };

                        if player_state == TnComponentState::Updating {
                            {
                                let context_guard = context.write().await;
                                let mut stream_data_guard = context_guard.stream_data.write().await;
                                stream_data_guard.get_mut(PLAYER).unwrap().1.clear();
                                tracing::info!( target:TRON_APP, "clean audio stream data");
                            }
                            {
                                let llm_tx = context.get_service_tx(LLM_SERVICE).await;

                                let (tx, _rx) = oneshot::channel::<String>();

                                let llm_req_msg = TnServiceRequestMsg {
                                    request: "chat-complete-interrupted".into(),
                                    payload: TnAsset::String("stop".into()),
                                    response: tx,
                                };

                                let _ = llm_tx.send(llm_req_msg).await;
                            }
                            {
                                let chatbot = context.get_component(TRANSCRIPT_OUTPUT).await;
                                chatbox::append_chatbox_value(
                                    chatbot.clone(),
                                    ("user".into(), transcript.clone()),
                                )
                                .await;
                            }
                            context.set_ready_for(TRANSCRIPT_OUTPUT).await;
                        } else {
                            let _ = tx
                                .send(TnServiceResponseMsg {
                                    response: "transcript_final".to_string(),
                                    payload: TnAsset::String(transcript),
                                })
                                .await;
                        }
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

/// Handles the post-processing of transcription responses.
///
/// This function listens for transcription responses from the transcription service and performs
/// further processing based on the response type. It handles both final transcripts and transcript
/// fragments.
///
/// When a final transcript is received, it sends the transcript to the language model service for
/// generating a response, appends the user's transcript to the chatbox, and triggers a server-sent
/// event to update the chatbox component.
///
/// When a transcript fragment is received, it stores the fragment in the application's asset store
/// and logs the fragment to the status component.
///
/// Args:
///     context: The application context.
///     response_rx: A receiver for receiving transcription responses.
async fn transcript_post_processing_service(
    context: TnContext,
    mut response_rx: Receiver<TnServiceResponseMsg>,
) {
    let assets = context.read().await.assets.clone();
    let components = context.read().await.components.clone();

    while let Some(response) = response_rx.recv().await {
        match response.response.as_str() {
            "transcript_final" => {
                if let TnAsset::String(transcript) = response.payload {
                    {
                        append_and_update_stream_textarea_with_context(
                            &context,
                            STATUS,
                            &format!("ASR output: {transcript}\n"),
                        )
                        .await;

                        let llm_tx = context.get_service_tx(LLM_SERVICE).await;

                        let (tx, rx) = oneshot::channel::<String>();

                        let llm_req_msg = TnServiceRequestMsg {
                            request: "chat-complete".into(),
                            payload: TnAsset::String(transcript.clone()),
                            response: tx,
                        };
                        let _ = llm_tx.send(llm_req_msg).await;

                        if let Ok(out) = rx.await {
                            tracing::debug!(target: TRON_APP, "returned string: {}", out);
                        };

                        {
                            let components_guard = components.write().await;
                            let transcript_area =
                                components_guard.get(TRANSCRIPT_OUTPUT).unwrap();
                            chatbox::append_chatbox_value(
                                transcript_area.clone(),
                                ("user".into(), transcript.clone()),
                            )
                            .await;
                        }
                        context.set_ready_for(TRANSCRIPT_OUTPUT).await;
                    }
                }
            }
            "transcript_fragment" => {
                if let TnAsset::String(transcript) = response.payload {
                    if !transcript.is_empty() {
                        let mut assets_guard = assets.write().await;
                        let e = assets_guard
                            .entry(TRANSCRIPT_OUTPUT.into())
                            .or_insert(TnAsset::VecString(Vec::default()));
                        if let TnAsset::VecString(ref mut v) = e {
                            v.push(transcript.clone());
                        }

                        append_and_update_stream_textarea_with_context(
                            &context,
                            STATUS,
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
