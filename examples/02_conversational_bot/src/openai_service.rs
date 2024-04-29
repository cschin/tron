use std::time::SystemTime;

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
use tokio::sync::mpsc::Receiver;
use tron_app::send_sse_msg_to_client;
use tron_app::{SseTriggerMsg, TriggerData};
use tron_components::{
    audio_player::start_audio, chatbox, text, TnAsset, TnContext, TnServiceRequestMsg
};
use tron_components::{text::append_and_send_stream_textarea_with_context, TnComponentValue};

pub async fn simulate_dialog(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    let client = Client::new();

    let reqwest_client = reqwest::Client::new();
    let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    let mut history = Vec::<(String, String)>::new();
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
            prompt.clone()
        } else {
            "You a useful assistant.".to_string()
        };
        tracing::info!(target: "tron_app", "prompt: {}", prompt1);

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

            let time = SystemTime::now();
            append_and_send_stream_textarea_with_context(
                context.clone(),
                "status",
                "LLM request start\n",
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

            // let response = client.chat().create(request).await.expect("error");
            {
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                stream_data_guard.get_mut("player").unwrap().1.clear();
                // clear the stream buffer
            }

            let mut llm_stream = client
                .chat()
                .create_stream(request)
                .await
                .expect("create stream fail for LLM API call");

            let mut llm_response = Vec::<String>::new();

            while let Some(result) = llm_stream.next().await {
                match result {
                    Ok(response) => {
                        if let Some(choice) = response.choices.first() {
                            if let Some(ref content) = choice.delta.content {
                                // tracing::info!(target: "tron_app", "LLM delta content: {}", content);
                                llm_response.push(content.clone());
                                let s = llm_response.join("");
                                let last_100 = s.len() % 100;
                                let s = s[s.len()-last_100..].to_string(); 
                                text::update_and_send_textarea_with_context(context.clone(), "llm_stream_output", &s).await;
                            }
                        }
                    }
                    Err(_err) => {
                        tracing::info!(target: "tron_app", "LLM stream error");
                    }
                }
            }
            let s = llm_response.join("");
            let last_100 = s.len() % 100;
            let s = s[s.len()-last_100..].to_string(); 
            text::update_and_send_textarea_with_context(context.clone(), "llm_stream_output", &s).await;

            let llm_response = llm_response.join("");
            history.push(("bot".into(), llm_response.clone()));
            let json_data = json!({"text": llm_response}).to_string();
            append_and_send_stream_textarea_with_context(
                context.clone(),
                "status",
                "TTS request request start:\n",
            )
            .await;

            let duration = time.elapsed().unwrap();
            append_and_send_stream_textarea_with_context(
                context.clone(),
                "status",
                &format!("LLM request done: {duration:?}\n"),
            )
            .await;

            let tts_model = if let TnComponentValue::String(tts_model) =
                context.get_value_from_component("tts_model_select").await
            {
                tts_model
            } else {
                "aura-zeus-en".into()
            };

            tracing::info!(target: "tron_app", "tts model: {}", tts_model);
            {
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
                    // trigger playing
                    let player = context.get_component("player").await;
                    let sse_tx = context.get_sse_tx_with_context().await;
                    start_audio(player.clone(), sse_tx).await;
                }
                {
                    let transcript_area = context.get_component("transcript").await;

                    chatbox::append_chatbox_value(
                        transcript_area.clone(),
                        ("bot".into(), llm_response.clone()),
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
                text::update_and_send_textarea_with_context(context.clone(), "llm_stream_output", "").await;
            }
        }
    }
}
