
#[allow(unused_imports)]
use async_openai::{
    types::{
        ChatCompletionRequestAssistantMessageArgs, ChatCompletionRequestMessage,
        ChatCompletionRequestSystemMessageArgs, ChatCompletionRequestUserMessageArgs,
        CreateChatCompletionRequestArgs,
    },
    Client,
};
use futures::StreamExt;
use tokio::sync::{mpsc::Receiver, oneshot};
use tron_app::tron_components;
use tron_app::send_sse_msg_to_client;
use tron_app::{TnServerSideTriggerData, TnSseTriggerMsg};

use tron_components::{
    chatbox, text, TnAsset, TnContext, TnServiceRequestMsg,
};
use tron_components::{text::append_and_send_stream_textarea_with_context, TnComponentValue};

static SENTENCE_END_TOKEN: &str = "<!--EOS-->";

pub async fn simulate_dialog(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    let client = Client::new();

    //let reqwest_client = reqwest::Client::new();
    //let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    let mut history = Vec::<(String, String)>::new();

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
                    "Please append a token '{SENTENCE_END_TOKEN}' at the end of a sentence or every 12 words.",
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
                                    
                                            make_tss_request( "new_llm_message".into(),  s[..sentence_end].to_string() ).await;
                                            last_end = sentence_end + SENTENCE_END_TOKEN.len();
                                        } else {
                                            llm_response_sentences
                                                .push(s[last_end..sentence_end].to_string());
      
                                            make_tss_request( "llm_message".into(),  s[last_end..sentence_end].to_string() ).await;
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
