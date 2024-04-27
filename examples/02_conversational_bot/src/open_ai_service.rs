use std::sync::Arc;

#[allow(unused_imports)]
use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use bytes::{BufMut, BytesMut};
use serde_json::json;
use tokio::sync::{mpsc::Receiver, RwLock};
use tron_app::send_sse_msg_to_client;
use tron_app::{SseTriggerMsg, TriggerData};
use tron_components::{
    audio_player::start_audio, chatbox, get_component_with_contex, Context, ServiceRequestMessage,
    TnAsset,
};
use tron_components::get_sse_tx_with_context;

pub async fn simulate_dialog(
    context: Arc<RwLock<Context<'static>>>,
    mut rx: Receiver<ServiceRequestMessage>,
) {
    let client = Client::new();
    let prompt1 = include_str!("../templates/prompt1.txt");
    let reqwest_client = reqwest::Client::new();
    let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    while let Some(r) = rx.recv().await {
        let _ = r.response.send("got it".to_string());
        if let TnAsset::String(query) = r.payload {
            let request = CreateChatCompletionRequestArgs::default()
                .max_tokens(512u16)
                //.model("gpt-4")
                .model("gpt-4-turbo")
                .messages([
                    ChatCompletionRequestSystemMessageArgs::default()
                        .content(prompt1)
                        .build()
                        .expect("error")
                        .into(),
                    ChatCompletionRequestUserMessageArgs::default()
                        .content(query)
                        .build()
                        .expect("error")
                        .into(),
                ])
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
            let mut llm_response = String::default();
            for choice in response.choices {
                tracing::debug!( target:"tron_app", "chat response {}: Role: {}  Content: {:?}", choice.index, choice.message.role, choice.message.content);
                if let Some(s) = choice.message.content {
                    llm_response = s.clone();
                    let json_data = json!({"text": s}).to_string();
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
                }
            }

            // trigger playing
            let player = get_component_with_contex(context.clone(), "player").await;
            let sse_tx = get_sse_tx_with_context(context.clone()).await;
            start_audio(player.clone(), sse_tx).await;

            {
                let transcript_area =
                    get_component_with_contex(context.clone(), "transcript").await;

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
                let sse_tx = get_sse_tx_with_context(context.clone()).await;
                send_sse_msg_to_client(&sse_tx, msg).await;
            }
        }
    }
}
