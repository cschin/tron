use std::sync::Arc;

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
use tokio::{
    sync::{mpsc::Receiver, oneshot, RwLock},
    task::JoinHandle,
};
use tron_app::{send_sse_msg_to_client, tron_components::audio_player::stop_audio};
use tron_app::tron_components;
use tron_app::{TnServerSideTriggerData, TnSseTriggerMsg};

use crate::{LLM_STREAM_OUTPUT, PLAYER, STATUS, TRANSCRIPT_OUTPUT, TRON_APP};
use tron_components::{chatbox, text, TnAsset, TnContext, TnServiceRequestMsg};
use tron_components::{text::append_and_send_stream_textarea_with_context, TnComponentValue};

static SENTENCE_END_TOKEN: &str = "<br/>";
//static SENTENCE_END_TOKEN: &str = " "; // end on "word boundary"

pub async fn simulate_dialog(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    //let reqwest_client = reqwest::Client::new();
    //let dg_api_key = std::env::var("DG_API_KEY").unwrap();

    let history = Arc::new(RwLock::new(Vec::<(String, String)>::new()));
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

    let mut handle: Option<JoinHandle<()>> = None;
    while let Some(r) = rx.recv().await {
        if let Some(handle) = handle.as_ref() {
            // TODO: make the loop in the openai_stream_service terminate gracefully.
            handle.abort();
            make_tss_request("new_llm_response".into(), "".into()).await;
        }
        let context = context.clone();
        if r.request == "clear-history" {
            history.write().await.clear();
            let _ = r.response.send("got it".to_string());
            continue;
        }

        let interrupted = r.request == "chat-complete-interrupted";
        tracing::info!(target: "tron_app", "interrupted: {}", interrupted);
        if interrupted {
            {
                let player = context.get_component(PLAYER).await;
                let sse_tx = context.get_sse_tx().await;
                stop_audio(player.clone(), sse_tx).await;
            }
        };

        let _ = r.response.send("got it".to_string());
        let interrupted_prompt = if let TnComponentValue::String(prompt) =
            context.get_value_from_component("prompt").await
        {
            [
                    prompt.clone(),
                    format!(
                         "you need to put a token '{SENTENCE_END_TOKEN}' at the end of every sentences and the end of the message.",
                    ),
                    "response as you are interrupted while you are trying to say something. Start with 'Ok...' or 'sorry,..' or ask what the question was, don't say more than one sentence.".into(),
                ]
                .join(" ")
        } else {
            "You a useful assistant.".to_string()
        };

        let prompt1 = if let TnComponentValue::String(prompt) =
            context.get_value_from_component("prompt").await
        {
            [
                prompt.clone(),
                format!(
                     "you need to put a token '{SENTENCE_END_TOKEN}' at the end of every sentences and the end of the message.",
                )
            ]
            .join(" ")
        } else {
            "You a useful assistant.".to_string()
        };
        // tracing::info!(target: TRON_APP, "prompt: {}", prompt1);

        if let TnAsset::String(query) = r.payload {
            {
                history.write().await.push(("user".into(), query.clone()));
            }

            let prompt = if interrupted {
                interrupted_prompt
            } else {
                prompt1
            };

            let mut messages: Vec<ChatCompletionRequestMessage> =
                vec![ChatCompletionRequestSystemMessageArgs::default()
                    .content(&prompt)
                    .build()
                    .expect("error")
                    .into()];

            if !interrupted {
                let history_len = history.read().await.len();
                let start = if history_len > 6 {
                    history_len - 6
                } else {
                    0
                };
                let guard = history.read().await;
                let history = guard.iter().skip(start).take(6);
                messages.extend(
                    history
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
            }
            tracing::debug!(target: TRON_APP, "sending messages: {:?}", messages);
            handle = Some(tokio::spawn(openai_stream_service(
                context.clone(),
                query,
                messages,
                history.clone(),
            )));
        }
    }
}

async fn openai_stream_service(
    context: TnContext,
    query: String,
    messages: Vec<ChatCompletionRequestMessage>,
    history: Arc<RwLock<Vec<(String, String)>>>,
) {
    let make_tts_request = |request: String, payload: String| async {
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

    let client = Client::new();
    { history.write().await.push(("user".into(), query.clone())); }

    append_and_send_stream_textarea_with_context(
        context.clone(),
        STATUS,
        "LLM service request start\n",
    )
    .await;

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(512u16)
        //.model("gpt-4")
        .model("gpt-4o")
        .messages(messages)
        .build()
        .expect("error");

    tracing::debug!( target:TRON_APP, "chat request to open ai {}", serde_json::to_string(&request).unwrap());

    let mut llm_stream = client
        .chat()
        .create_stream(request)
        .await
        .expect("create stream fail for LLM API call");

    let mut llm_response = Vec::<String>::new();
    let mut llm_response_sentences = Vec::<String>::new();
    let mut last_end = 0_usize;

    // TODO: we need to find a way to break the loop when the user speaks
    while let Some(result) = llm_stream.next().await {
        match result {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    if let Some(ref content) = choice.delta.content {
                        tracing::debug!(target: TRON_APP, "LLM delta content: {}", content);
                        llm_response.push(content.clone());
                        let s = llm_response.join("");
                        let sentence_end = s.rfind(SENTENCE_END_TOKEN);
                        if let Some(sentence_end) = sentence_end {
                            if last_end < sentence_end {
                                if llm_response_sentences.is_empty() {
                                    llm_response_sentences.push(s[..sentence_end].to_string());

                                    make_tts_request(
                                        "new_llm_message".into(),
                                        s[..sentence_end].to_string(),
                                    )
                                    .await;
                                    last_end = sentence_end + SENTENCE_END_TOKEN.len();
                                } else {
                                    llm_response_sentences
                                        .push(s[last_end..sentence_end].to_string());

                                    make_tts_request(
                                        "llm_message".into(),
                                        s[last_end..sentence_end].to_string(),
                                    )
                                    .await;
                                    last_end = sentence_end + SENTENCE_END_TOKEN.len();
                                }
                            }
                        }

                        let s = s.chars().collect::<Vec<_>>();
                        let last_100 = s.len() % 100;

                        let s = if last_100 > 0 {
                            s[s.len() - last_100..].iter().cloned().collect::<String>()
                        } else {
                            "".into()
                        };
                        text::update_and_send_textarea_with_context(
                            context.clone(),
                            LLM_STREAM_OUTPUT,
                            &s,
                        )
                        .await;
                    }
                }
            }
            Err(_err) => {
                tracing::info!(target: TRON_APP, "LLM stream error");
            }
        }
    }
    let s = llm_response.join("");
    if last_end < s.len() - 1 {
        llm_response_sentences.push(s[last_end..].to_string());

        make_tts_request("llm_message".into(), s[last_end..].to_string()).await;
    };
    let s = s.chars().collect::<Vec<_>>();
    let last_100 = s.len() % 100;
    let s = if last_100 > 0 {
        s[s.len() - last_100..].iter().cloned().collect::<String>()
    } else {
        "".into()
    };

    text::update_and_send_textarea_with_context(context.clone(), LLM_STREAM_OUTPUT, &s).await;

    let llm_response = llm_response.join("");
    tracing::info!(target: TRON_APP, "LLM response: {}", llm_response);
    { history.write().await.push(("bot".into(), llm_response.clone())); }

    {
        let transcript_area = context.get_component(TRANSCRIPT_OUTPUT).await;

        chatbox::append_chatbox_value(
            transcript_area.clone(),
            ("bot".into(), llm_response_sentences.join("<br />")),
        )
        .await;
    }

    {
        let msg = TnSseTriggerMsg {
            server_side_trigger_data: TnServerSideTriggerData {
                target: TRANSCRIPT_OUTPUT.into(),
                new_state: "ready".into(),
            },
        };
        let sse_tx = context.get_sse_tx().await;
        send_sse_msg_to_client(&sse_tx, msg).await;
    }

    text::update_and_send_textarea_with_context(context.clone(), LLM_STREAM_OUTPUT, "").await;
}
