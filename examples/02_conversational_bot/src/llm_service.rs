use std::{collections::VecDeque, sync::Arc, time::SystemTime};

#[allow(unused_imports)]
use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use bytes::{BufMut, BytesMut};
use futures::StreamExt;
use serde_json::json;
use tokio::sync::{mpsc::Receiver, Mutex};
use tron_app::tron_components;
use tron_app::{send_sse_msg_to_client, tron_components::TnComponentState};
use tron_app::{TnServerSideTriggerData, TnSseTriggerMsg};

use tron_components::{
    audio_player::start_audio, chatbox, text, TnAsset, TnContext, TnServiceRequestMsg,
};
use tron_components::{text::append_and_send_stream_textarea_with_context, TnComponentValue};

static SENTENCE_END_TOKEN: &str = "<!--EOS-->";

pub async fn simulate_dialog(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    let client = Client::new();

    //let reqwest_client = reqwest::Client::new();
    //let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    let mut history = Vec::<(String, String)>::new();
    let (audio_tx, audio_rx) = tokio::sync::mpsc::channel::<(bool, String)>(32);

    tokio::spawn(tts_service(audio_rx, context.clone()));
    while let Some(r) = rx.recv().await {
        let context = context.clone();
        if r.request == "clear-history" {
            history.clear();
            let _ = r.response.send("got it".to_string());
            continue;
        }

        let _ = r.response.send("got it".to_string());
        let prompt1 = if let TnComponentValue::String(prompt) =
            context.get_value_from_component("prompt").await
        {
            [
                prompt.clone(),
                format!(
                    "Please append a token '{SENTENCE_END_TOKEN}' at the end of each sentence."
                ),
            ]
            .join(" ")
        } else {
            "You a useful assistant.".to_string()
        };
        // tracing::info!(target: "tron_app", "prompt: {}", prompt1);

        if let TnAsset::String(query) = r.payload {
            history.push(("user".into(), query.clone()));
            let mut messages: Vec<ChatCompletionRequestMessage> =
                vec![ChatCompletionRequestSystemMessageArgs::default()
                    .content(&prompt1)
                    .build()
                    .expect("error")
                    .into()];
            messages.extend(
                history
                    .iter()
                    .filter_map(|(tag, msg)| match tag.as_str() {
                        "user" => Some(
                            ChatCompletionRequestUserMessageArgs::default()
                                .content(msg.clone())
                                .build()
                                .expect("error")
                                .into(),
                        ),
                        "bot" => Some(
                            ChatCompletionRequestAssistantMessageArgs::default()
                                .content(msg.clone())
                                .build()
                                .expect("error")
                                .into(),
                        ),
                        _ => None,
                    })
                    .collect::<Vec<ChatCompletionRequestMessage>>(),
            );

            append_and_send_stream_textarea_with_context(
                context.clone(),
                "status",
                "LLM service request start\n",
            )
            .await;

            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(512u16)
                //.model("gpt-4")
                .model("gpt-4-turbo")
                .messages(messages)
                .build()
                .expect("error");

            tracing::debug!( target:"tron_app", "chat request to open ai {}", serde_json::to_string(&request).unwrap());

            let mut llm_stream = client
                .chat()
                .create_stream(request)
                .await
                .expect("create stream fail for LLM API call");

            let mut llm_response = Vec::<String>::new();
            let mut llm_response_sentences = Vec::<String>::new();
            let mut last_end = 0_usize;

            while let Some(result) = llm_stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            if let Some(ref content) = choice.delta.content {
                                tracing::debug!(target: "tron_app", "LLM delta content: {}", content);
                                llm_response.push(content.clone());
                                let s = llm_response.join("");
                                let sentence_end = s.rfind(SENTENCE_END_TOKEN);
                                if let Some(sentence_end) = sentence_end {
                                    if last_end < sentence_end {
                                        if llm_response_sentences.is_empty() {
                                            llm_response_sentences
                                                .push(s[..sentence_end].to_string());
                                            let _ = audio_tx
                                                .send((true, s[..sentence_end].to_string()))
                                                .await;
                                            last_end = sentence_end + SENTENCE_END_TOKEN.len();
                                        } else {
                                            llm_response_sentences
                                                .push(s[last_end..sentence_end].to_string());
                                            let _ = audio_tx
                                                .send((
                                                    false,
                                                    s[last_end..sentence_end].to_string(),
                                                ))
                                                .await;
                                            last_end = sentence_end + SENTENCE_END_TOKEN.len();
                                        }
                                    }
                                };
                                let s = s.chars().collect::<Vec<_>>();
                                let last_100 = s.len() % 100;

                                let s = if last_100 > 0 {
                                    s[s.len() - last_100..].iter().cloned().collect::<String>()
                                } else {
                                    "".into()
                                };
                                text::update_and_send_textarea_with_context(
                                    context.clone(),
                                    "llm_stream_output",
                                    &s,
                                )
                                .await;
                            }
                        }
                    }
                    Err(_err) => {
                        tracing::info!(target: "tron_app", "LLM stream error");
                    }
                }
            }
            let s = llm_response.join("");
            let s = s.chars().collect::<Vec<_>>();
            let last_100 = s.len() % 100;
            let s = if last_100 > 0 {
                s[s.len() - last_100..].iter().cloned().collect::<String>()
            } else {
                "".into()
            };
            text::update_and_send_textarea_with_context(context.clone(), "llm_stream_output", &s)
                .await;

            let llm_response = llm_response.join("");
            tracing::info!(target: "tron_app", "LLM response: {}", llm_response);
            history.push(("bot".into(), llm_response.clone()));

            {
                let transcript_area = context.get_component("transcript").await;

                chatbox::append_chatbox_value(
                    transcript_area.clone(),
                    ("bot".into(), llm_response_sentences.join("<br />")),
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

            text::update_and_send_textarea_with_context(context.clone(), "llm_stream_output", "")
                .await;
        }
    }
}

async fn tts_service(mut rx: Receiver<(bool, String)>, context: TnContext) {
    let reqwest_client = reqwest::Client::new();
    let dg_api_key: String = std::env::var("DG_API_KEY").unwrap();
    let msg_queue = Arc::new(Mutex::new(VecDeque::<String>::new()));
    let msg_queue1 = msg_queue.clone();

    // put the message into a buffer
    // when a new LLM response comes in, clean up the buffer to stop TTS to emulate the 
    // bot's response is interrupted. Currently, the TTS is done by sentence by sentence.
    // The interuption will be done after the current sentence is finished.
    tokio::spawn(async move {
        while let Some((restart, llm_response)) = rx.recv().await {
            if restart {
                msg_queue1.lock().await.clear();
                msg_queue1.lock().await.push_back(llm_response);
            } else {
                msg_queue1.lock().await.push_back(llm_response);
            }
        }
    });

    loop {
        let llm_response = if msg_queue.lock().await.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            continue;
        } else {
            msg_queue.lock().await.pop_front().unwrap()
        };

        let tts_model = if let TnComponentValue::String(tts_model) =
            context.get_value_from_component("tts_model_select").await
        {
            tts_model
        } else {
            "aura-zeus-en".into()
        };

        let json_data = json!({"text": llm_response}).to_string();
        tracing::info!(target:"tron_app", "json_data: {}", json_data);
        let time = SystemTime::now();

        let mut response = reqwest_client
            .post(format!(
                "https://api.deepgram.com/v1/speak?model={tts_model}"
            ))
            //.post("https://api.deepgram.com/v1/speak?model=aura-stella-en")
            .header("Content-Type", "application/json")
            .header("Authorization", format!("Token {}", dg_api_key))
            .body(json_data.to_owned())
            .send()
            .await
            .unwrap();
        tracing::debug!( target:"tron_app", "response: {:?}", response);

        let duration = time.elapsed().unwrap();
        append_and_send_stream_textarea_with_context(
            context.clone(),
            "status",
            &format!("TTS request request done: {duration:?}\n"),
        )
        .await;
        // send TTS data to the player stream out
        while let Some(chunk) = response.chunk().await.unwrap() {
            tracing::debug!( target:"tron_app", "Chunk: {}", chunk.len());
            {
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                let player_data = stream_data_guard.get_mut("player").unwrap();
                let mut data = BytesMut::new();
                data.put(&chunk[..]);
                player_data.1.push_back(data);
            }
        }
        {
            let context_guard = context.write().await;
            let mut stream_data_guard = context_guard.stream_data.write().await;
            let player_data = stream_data_guard.get_mut("player").unwrap();
            // ensure we don't send empty data to the player, or the empty data can trigger an infinite loop
            if player_data.1.is_empty() {
                continue;
            }
        }

        // for debug
        // {
        //     let player_guard = context.get_component("player").await;
        //     let player = player_guard.read().await;
        //     let audio_ready = player.state();
        //     let context_guard = context.write().await;
        //     let mut stream_data_guard = context_guard.stream_data.write().await;
        //     let player_data = stream_data_guard.get_mut("player").unwrap();

        //     tracing::info!( target:"tron_app", "Start: player status {:?}", audio_ready);
        //     tracing::info!( target:"tron_app", "player data len: {:?}", player_data.1.len());
        // }

        // This loop waits for the audio player component to be in the "Ready" state.
        // Once the player is ready, it calls the `start_audio` function with the player
        // component and the server-sent event (SSE) transmitter.
        //
        // The loop checks the state of the player component by acquiring a read lock
        // on the component and cloning its state. If the state is "Ready", it proceeds
        // to start the audio playback. Otherwise, it logs a debug message and waits for
        // 100 milliseconds before checking the state again.
        //
        // After starting the audio playback, the loop breaks, allowing the program to
        // continue with other tasks.
        loop {
            let audio_ready = {
                let player_guard = context.get_component("player").await;
                let player = player_guard.read().await;
                player.state().clone()
            };
            tracing::debug!( target:"tron_app", "wait for the player to ready to play {:?}", audio_ready);
            match audio_ready {
                TnComponentState::Ready => {
                    let player = context.get_component("player").await;
                    let sse_tx = context.get_sse_tx().await;
                    start_audio(player.clone(), sse_tx).await;
                    tracing::debug!( target:"tron_app", "set audio to play");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    break;
                }
                _ => {
                    tracing::debug!( target:"tron_app", "Waiting for audio to be ready");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }

        // This loop waits for the audio player component to be in the "Ready" state.
        // Once the player is ready, it clean up the stream buffer.
        loop {
            let audio_ready = {
                let player_guard = context.get_component("player").await;
                let player = player_guard.read().await;
                player.state().clone()
            };
            tracing::debug!( target:"tron_app", "wait for the player in the ready state to clean up the stream data: {:?}", audio_ready);
            match audio_ready {
                TnComponentState::Ready => {
                    // clear the stream buffer
                    let context_guard = context.write().await;
                    let mut stream_data_guard = context_guard.stream_data.write().await;
                    stream_data_guard.get_mut("player").unwrap().1.clear();
                    tracing::debug!( target:"tron_app", "clean audio stream data");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    break;
                }
                _ => {
                    tracing::debug!( target:"tron_app", "Waiting for audio to be ready");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }
    }
}
