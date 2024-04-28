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
use serde_json::json;
use tokio::sync::mpsc::Receiver;
use tron_app::send_sse_msg_to_client;
use tron_app::{SseTriggerMsg, TriggerData};
use tron_components::{
    audio_player::start_audio, chatbox, TnAsset, TnContext, TnServiceRequestMsg,
};
use tron_components::{text::append_and_send_stream_textarea_with_context, TnComponentValue};

pub async fn simulate_dialog(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    let client = Client::new();

    let reqwest_client = reqwest::Client::new();
    let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    let mut history = Vec::<(String, String)>::new();
    while let Some(r) = rx.recv().await {
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

            let response = client.chat().create(request).await.expect("error");
            {
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                stream_data_guard.get_mut("player").unwrap().1.clear();
                // clear the stream buffer
            }
            let duration = time.elapsed().unwrap();
            append_and_send_stream_textarea_with_context(
                context.clone(),
                "status",
                &format!("LLM request done: {duration:?}\n"),
            )
            .await;

            let llm_response = if let Some(choice) = response.choices.first() {
                let choice = choice.clone();
                tracing::debug!( target:"tron_app", "chat response {}: Role: {}  Content: {:?}", choice.index, choice.message.role, choice.message.content);
                if let Some(llm_response) = choice.message.content {
                    history.push(("bot".into(), llm_response.clone()));
                    let json_data = json!({"text": llm_response}).to_string();

                    let time = SystemTime::now();
                    append_and_send_stream_textarea_with_context(
                        context.clone(),
                        "status",
                        "TTS request request start:\n",
                    )
                    .await;

                    let mut response = reqwest_client
                        .post("https://api.deepgram.com/v1/speak?model=aura-zeus-en")
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
                    Some(llm_response.clone())
                } else {
                    None
                }
            } else {
                None
            };

            // trigger playing
            let player = context.get_component("player").await;
            let sse_tx = context.get_sse_tx_with_context().await;
            start_audio(player.clone(), sse_tx).await;

            if let Some(llm_response) = llm_response {
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
            }
        }
    }
}
