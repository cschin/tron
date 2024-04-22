use std::{error::Error, sync::Arc};

use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestSystemMessageArgs,
        ChatCompletionRequestUserMessageArgs, CreateChatCompletionRequestArgs,
    },
    Client,
};
use axum::extract::Query;
use bytes::{BufMut, BytesMut};
use reqwest;
use serde_json::json;
use tokio::sync::{mpsc::{Receiver, Sender}, RwLock};
use tron_app::{SseTriggerMsg, TriggerData};
use tron_components::{audio_player, ComponentState, Context, ServiceRequestMessage, TnAsset};
use tron_app::{
    utils::send_sse_msg_to_client, SseAudioRecorderTriggerMsg,

};
use tron_components::text;

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
                .model("gpt-4")
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

            println!("{}", serde_json::to_string(&request).unwrap());

            let response = client.chat().create(request).await.expect("error");
            println!("\nResponse:\n");
            {
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                stream_data_guard.get_mut("player").unwrap().1.clear();
                // clear the stream buffer
            }
            let mut llm_response = String::default();
            for choice in response.choices {
                println!(
                    "{}: Role: {}  Content: {:?}",
                    choice.index, choice.message.role, choice.message.content
                );
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
                    println!("response: {:?}", response);
                    while let Some(chunk) = response.chunk().await.unwrap() {
                        println!("Chunk: {}", chunk.len());
                        {
                            let context_guard = context.write().await;
                            let mut stream_data_guard = context_guard.stream_data.write().await;
                            let player_data = stream_data_guard.get_mut("player").unwrap();
                            let mut data = BytesMut::new();
                            data.put(&chunk[..]);
                            player_data.1.push_back(data);
                        }
                    }
                    // let buf = response.bytes().await.unwrap();
                    // let uid = uuid::Uuid::new_v4();
                    // let filename = std::path::Path::new("output").join(format!("{}.mp4", uid));
                    // let mut file = std::fs::File::create(filename.clone()).unwrap();
                    // std::io::Write::write_all(&mut file, &buf).unwrap();
                }
            }
            {
                let context_guard = context.write().await;
                let mut components_guard = context_guard.components.write().await;
                let player_id = context_guard.get_component_id("player");
                let player = components_guard.get_mut(&player_id).unwrap();
                player.set_attribute("autoplay".into(), "true".into());
                player.set_attribute("src".into(), "/tron_streaming/player".into());
                player.set_state(ComponentState::Updating);
            }

            let msg = SseTriggerMsg {
                server_side_trigger: TriggerData {
                    target: "player".into(),
                    new_state: "updating".into(),
                },
            };
            let sse_tx = get_sse_tx(context.clone()).await; 
            send_sse_msg_to_client(&sse_tx, msg).await;

            {
                let components = context.read().await.components.clone();
                let transcript_area_id = context.read().await.get_component_id("transcript");
                let mut components_guard = components.write().await;
                let transcript_area =
                    components_guard.get_mut(&transcript_area_id).unwrap();
                text::append_textarea_value(transcript_area, &format!(">> {} \n\n<<", llm_response), None);
            }
            {
                let msg = SseTriggerMsg {
                    server_side_trigger: TriggerData {
                        target: "transcript".into(),
                        new_state: "ready".into(),
                    },
                };
                send_sse_msg_to_client(&sse_tx, msg).await;
            }
        }
    }
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

