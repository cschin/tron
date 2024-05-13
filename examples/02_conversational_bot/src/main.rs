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
use tron_components::{text::append_and_send_stream_textarea_with_context, *};

#[tokio::main]
async fn main() {
    // Configure the application
    let app_configure = tron_app::AppConfigure {
        log_level: Some("server=info,tower_http=info,tron_app=info"),
        cognito_login: true,
        ..Default::default()
    };
    // set app state
    let app_share_data = tron_app::AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_session_context)),
        build_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };

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
    let mut component_index = 0_u32;

    {
        // add a recorder button
        let mut btn = TnButton::new(
            component_index,
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
        component_index += 1;
        let recorder = TnAudioRecorder::<'static>::new(
            component_index,
            "recorder".to_string(),
            "Paused".to_string(),
        );
        context.add_component(recorder);
    }
    {
        // add a player
        component_index += 1;
        let mut player = TnAudioPlayer::<'static>::new(
            component_index,
            "player".to_string(),
            "Paused".to_string(),
        );
        player.set_attribute("class".to_string(), "flex-1 p-1 h-10".to_string());
        context.add_component(player);
    }

    {
        // add a reset button
        component_index += 1;
        let mut btn = TnButton::new(
            component_index,
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
        component_index += 1;
        let mut transcript_output =
            TnChatBox::<'static>::new(component_index, "transcript".to_string(), vec![]);
        transcript_output.set_attribute(
            "class".to_string(),
            "flex flex-col overflow-auto flex-1 p-2".to_string(),
        );

        context.add_component(transcript_output);
    }
    {
        // add a textarea showing partial stream content
        component_index += 1;
        let mut llm_stream_output =
            TnTextArea::<'static>::new(component_index, "llm_stream_output".to_string(), "".into());
        llm_stream_output.set_attribute(
            "class".to_string(),
            "overflow-auto flex-1 p-2 h-19 max-h-19 min-h-19".to_string(),
        );
        context.add_component(llm_stream_output);
    }

    {
        // add a status box
        component_index += 1;
        let mut status_output =
            TnStreamTextArea::<'static>::new(component_index, "status".to_string(), vec![]);
        status_output.set_attribute(
            "class".to_string(),
            "flex-1 p-2 textarea textarea-bordered h-40 max-h-40 min-h-40".to_string(),
        );
        status_output.set_attribute("hx-trigger".into(), "server_side_trigger".into());

        context.add_component(status_output);
    }

    {
        component_index += 1;
        let prompt = include_str!("../templates/drunk-bioinformatist.txt");
        let mut prompt_box =
            TnTextArea::<'static>::new(component_index, "prompt".into(), prompt.into());
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

    {
        component_index += 1;
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
        let mut tts_model_select = TnSelect::<'static>::new(
            component_index,
            "tts_model_select".into(),
            default_model,
            model_options,
        );

        tts_model_select.set_attribute(
            "class".into(),
            "select select-bordered w-full max-w-xs".into(),
        );
        context.add_component(tts_model_select);
    }

    {
        component_index += 1;
        let default_prompt = "drunk-bioinformatist".into();
        let prompt_options = vec![
            ("drunk-bioinformatist".into(), "Drunk Bioinformatist".into()),
            ("socrat".into(), "Socrat".into()),
            ("poet".into(), "Poet".into()),
        ];
        let mut preset_prompt_select = TnSelect::<'static>::new(
            component_index,
            "preset_prompt_select".into(),
            default_prompt,
            prompt_options,
        );
        preset_prompt_select.set_attribute(
            "class".into(),
            "select select-bordered w-full max-w-xs".into(),
        );
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
            .asset
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

        let (tts_tx, tts_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(32);
        context
            .blocking_write()
            .services
            .insert("tts_service".into(), (tts_tx.clone(), Mutex::new(None)));
        tokio::spawn(tts_service(tts_rx, context.clone()));
    }

    {
        // set up audio stream out
        let context_guard = context.blocking_write();
        let mut stream_data_guard = context_guard.stream_data.blocking_write();
        stream_data_guard.insert("player".into(), ("audio/mp3".into(), VecDeque::default()));
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
/// This function sets the initial values and states for the "recorder" and "rec_button" components.
/// It then retrieves the rendered HTML strings for various components and constructs an
/// `AppPageTemplate` struct with these strings. Finally, it renders the template to produce the
/// HTML layout for the application page.
///
/// Returns:
///     A `String` containing the rendered HTML layout for the application page.
fn layout(context: TnContext) -> String {
    context.set_value_for_component_blocking("recorder", TnComponentValue::String("Paused".into()));
    context.set_state_for_component_blocking("recorder", TnComponentState::Ready);

    context.set_value_for_component_blocking(
        "rec_button",
        TnComponentValue::String("Start Conversation".into()),
    );
    context.set_state_for_component_blocking("rec_button", TnComponentState::Ready);

    let guard = context.blocking_read();
    let btn = guard.render_to_string("rec_button");
    let recorder = guard.render_to_string("recorder");
    let player = guard.render_to_string("player");
    let transcript = guard.first_render_to_string("transcript");
    let status = guard.first_render_to_string("status");
    let prompt = guard.first_render_to_string("prompt");
    let reset_button = guard.first_render_to_string("reset_button");
    let tts_model_select = guard.first_render_to_string("tts_model_select");
    let llm_stream_output = guard.first_render_to_string("llm_stream_output");
    let preset_prompt_select = guard.first_render_to_string("preset_prompt_select");

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

/// Builds and initializes the event actions for the application.
///
/// This function sets up the event actions for various components such as the record button,
/// audio recorder, audio player, reset button, and preset prompt select dropdown. It associates
/// each component with a specific action function and an execution method (await or immediate).
///
/// Returns:
///     A `TnEventActions` instance
fn build_session_actions(context: TnContext) -> TnEventActions {
    let mut actions = Vec::<(String, TnActionExecutionMethod, TnActionFn)>::new();
    actions.push((
        "rec_button".into(),
        TnActionExecutionMethod::Await,
        toggle_recording,
    ));
    actions.push((
        "recorder".into(),
        TnActionExecutionMethod::Await,
        audio_input_stream_processing,
    ));
    actions.push((
        "player".into(),
        TnActionExecutionMethod::Await,
        audio_player::stop_audio_playing_action,
    ));
    actions.push((
        "reset_button".into(),
        TnActionExecutionMethod::Await,
        reset_conversation,
    ));
    actions.push((
        "preset_prompt_select".into(),
        TnActionExecutionMethod::Await,
        preset_prompt_select_change,
    ));

    actions
        .into_iter()
        .map(|(id, exe_method, action_fn)| {
            let idx = context.blocking_read().get_component_index(&id);
            (idx, (exe_method, Arc::new(action_fn)))
        })
        .collect::<TnEventActions>()
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
        tracing::info!(target: "tron_app", "value: {:?}", comp.read().await.value());
    };
    Box::pin(f)
}

/// Handles the change event for the preset prompt select dropdown.
///
/// This function retrieves the selected value from the "preset_prompt_select" component and uses
/// it to fetch the corresponding prompt from the application's asset store. It then sets the
/// fetched prompt as the value of the "prompt" text area component and marks both the "prompt"
/// and "preset_prompt_select" components as ready.
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
        let value = context
            .get_value_from_component("preset_prompt_select")
            .await;
        tracing::info!(target: "tron_app", "preset_prompt_select_change, value:{:?}", value);
        let s = if let TnComponentValue::String(s) = value {
            s
        } else {
            "".into()
        };
        let prompt = if let Some(m) = context.read().await.asset.read().await.get("prompts") {
            if let TnAsset::HashMapString(ref m) = m {
                m.get(&s).unwrap().clone()
            } else {
                "".to_string()
            }
        } else {
            "".into()
        };
        // tracing::info!(target: "tron_app", "preset_prompt_select_change, prompt:{}", prompt);
        context
            .set_value_for_component("prompt", TnComponentValue::String(prompt))
            .await;
        context.set_ready_for("prompt").await;
        context.set_ready_for("preset_prompt_select").await;

        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f)
}

/// Handles the reset conversation action.
///
/// This function is triggered when the "reset_button" component is clicked and in the "ready" state.
/// It sends a "clear-history" request to the "llm_service" to clear the conversation history.
/// Then, it cleans the chatbox component with the ID "transcript" and sets the "reset_button" component to the "ready" state.
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

            let llm_tx = context.get_service_tx("llm_service").await;
            let (tx, mut rx) = oneshot::channel::<String>();

            let llm_req_msg = TnServiceRequestMsg {
                request: "clear-history".into(),
                payload: TnAsset::String("".into()),
                response: tx,
            };

            let _ = llm_tx.send(llm_req_msg).await;
            let _ = rx.try_recv();
        }

        chatbox::clean_chatbox_with_context(context.clone(), "transcript").await;
        context.set_ready_for("reset_button").await;

        let html = context.render_component(&event.h_target.unwrap()).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f)
}

/// Toggles the recording state of the audio recorder component.
///
/// This function is triggered when the "rec_button" component is clicked or a server-side trigger
/// event occurs. It handles the start and stop of the audio recording process.
///
/// When the recording is started, it sets the "recorder" component's value to "Pending" and its
/// state to "Updating". It then clears the audio stream buffer and sends a server-sent event (SSE)
/// to start the audio recording on the client-side.
///
/// When the recording is stopped, it sets the "recorder" component's value to "Paused" and sends
/// an SSE to stop the audio recording on the client-side. It also sends a request to the
/// "transcript_service" to stop the transcription process.
///
/// After handling the start or stop of the recording, it sets the "rec_button" component's state
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
                                server_side_trigger_data: TnServerSideTriggerData {
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
                                TnComponentValue::String("Pending".into()),
                            )
                            .await;

                        context
                            .set_state_for_component("recorder", TnComponentState::Updating)
                            .await;

                        context.set_ready_for("player").await; // rest the player state to ready

                        {
                            let context_guard = context.write().await;
                            let mut stream_data_guard = context_guard.stream_data.write().await;
                            // clear the stream buffer
                            stream_data_guard.get_mut("player").unwrap().1.clear();
                        }

                        let msg = SseAudioRecorderTriggerMsg {
                            server_side_trigger_data: TnServerSideTriggerData {
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
/// It appends the audio data to the "recorder" component, updates the value and state of the
/// "recorder" component as needed, and sends the audio data to the "transcript_service" for
/// transcription.
///
/// Additionally, it logs a message to the "status" component with the chunk length of the audio
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

                let recorder_value = context.get_value_from_component("recorder").await;
                if let TnComponentValue::String(s) = recorder_value {
                    if s.as_str() == "Pending" {
                        context
                            .set_value_for_component(
                                "recorder",
                                TnComponentValue::String("Recording".into()),
                            )
                            .await;

                        context
                            .set_state_for_component("recorder", TnComponentState::Updating)
                            .await;

                        let msg = TnSseTriggerMsg {
                            server_side_trigger_data: TnServerSideTriggerData {
                                target: "recorder".into(),
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
    mut rx: Receiver<TnServiceRequestMsg>,
    tx: Sender<TnServiceResponseMsg>,
) {
    let (transcript_tx, mut transcript_rx) = tokio::sync::mpsc::channel::<StreamResponse>(16);

    // start a loop for maintaining connection with DG, this calls the DG WS,
    // it passes transcript_tx to deepgram_transcript_service(), so it can send the transcript back
    tokio::spawn(async move {
        tracing::debug!(target: "tran_app", "restart dg_trx");

        let (mut audio_tx, audio_rx) =
            tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(16);

        let mut handle = tokio::spawn(deepgram_transcript_service(audio_rx, transcript_tx.clone()));

        tokio::time::sleep(std::time::Duration::from_secs(1)).await;

        while let Some(req) = rx.recv().await {
            tracing::debug!(target: "tran_app", "req: {}", req.request);

            if handle.is_finished() {
                audio_tx.closed().await;
                let (audio_tx0, audio_rx) =
                    tokio::sync::mpsc::channel::<Result<Bytes, DeepgramError>>(16);
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
    let assets = context.read().await.asset.clone();
    let components = context.read().await.components.clone();
    let transcript_area_id = context
        .clone()
        .read()
        .await
        .get_component_index("transcript");

    let make_tss_request = |request: String, payload: String| async {
        let (tx, mut rx) = oneshot::channel::<String>();
        let msg = TnServiceRequestMsg {
            request,
            payload: TnAsset::String(payload),
            response: tx,
        };
        let tts_tx = context.get_service_tx("tts_service").await;
        let _ = tts_tx.send(msg).await;
        let _ = rx.try_recv();
    };

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

                        let llm_tx = context.get_service_tx("llm_service").await;

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
                            let msg = TnSseTriggerMsg {
                                server_side_trigger_data: TnServerSideTriggerData {
                                    target: "transcript".into(),
                                    new_state: "ready".into(),
                                },
                            };
                            let sse_tx = context.get_sse_tx().await;
                            send_sse_msg_to_client(&sse_tx, msg).await;
                        }
                    }
                }
            }
            "transcript_fragment" => {
                if let TnAsset::String(transcript) = response.payload {
                    if !transcript.is_empty() {
                        let mut assets_guard = assets.write().await;
                        let e = assets_guard
                            .entry("transcript".into())
                            .or_insert(TnAsset::VecString(Vec::default()));
                        if let TnAsset::VecString(ref mut v) = e {
                            v.push(transcript.clone());
                        }
                        //(*e).push(TnAsset::String(transcript.clone()));

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
