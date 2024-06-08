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
use tron_app::tron_components;

use crate::{QUERY_RESULT_TEXTAREA, QUERY_STREAM_TEXTAREA, TOP_HIT_TEXTAREA, TRON_APP};
use tron_components::{
    chatbox, text, text::append_and_send_stream_textarea_with_context, TnAsset, TnComponentValue,
    TnContext, TnServiceRequestMsg,
};

pub async fn llm_service(context: TnContext, mut rx: Receiver<TnServiceRequestMsg>) {
    let history = Arc::new(RwLock::new(Vec::<(String, String)>::new()));
    let mut handle: Option<JoinHandle<()>> = None;
    while let Some(r) = rx.recv().await {
        let context = context.clone();
        if r.request == "clear-history" {
            history.write().await.clear();
            let _ = r.response.send("got it".to_string());
            continue;
        }

        let _ = r.response.send("got it".to_string());

        let prompt = "You are a science scholar who are very good analyze snippet from research paper. You will be asked to compare chunks of content text from different paper or asked to summary them.";

        // tracing::info!(target: TRON_APP, "prompt: {}", prompt1);

        if let TnAsset::String(query) = r.payload {
            {
                history.write().await.push(("user".into(), query.clone()));
            }

            let mut messages: Vec<ChatCompletionRequestMessage> =
                vec![ChatCompletionRequestSystemMessageArgs::default()
                    .content(prompt)
                    .build()
                    .expect("error")
                    .into()];
            let history_len = history.read().await.len();
            let start = if history_len > 6 { history_len - 6 } else { 0 };
            let guard = history.read().await;
            let history_guard = guard.iter().skip(start).take(6);
            messages.extend(
                history_guard
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

            tracing::debug!(target: TRON_APP, "query: {:?}", query);
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
    
    let client = Client::new();
    {
        history.write().await.push(("user".into(), query.clone()));
    }

    {
        let query_result_area = context.get_component(QUERY_RESULT_TEXTAREA).await;
        let query = query.replace('\n', "<br>");

        chatbox::append_chatbox_value(query_result_area.clone(), ("user".into(), query)).await;
    }
    context.set_ready_for(QUERY_RESULT_TEXTAREA).await;

    let request = CreateChatCompletionRequestArgs::default()
        .max_tokens(2048u16)
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

    // TODO: we need to find a way to break the loop when the user speaks
    while let Some(result) = llm_stream.next().await {
        match result {
            Ok(response) => {
                if let Some(choice) = response.choices.first() {
                    if let Some(ref content) = choice.delta.content {
                        tracing::debug!(target: TRON_APP, "LLM delta content: {}", content);
                        llm_response.push(content.clone());
                        let s = content.clone();
                        let s = s.replace(' ', "&nbsp;");

                        text::append_and_send_stream_textarea_with_context(
                            context.clone(),
                            QUERY_STREAM_TEXTAREA,
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
    text::finish_stream_textarea_with_context(
        context.clone(),
        QUERY_STREAM_TEXTAREA,
    )
    .await;

    let llm_response = llm_response.join("");
    let llm_response = llm_response.replace('\n', "<br>");
    tracing::debug!(target: TRON_APP, "LLM response: {}", llm_response);

    {
        let query_result_area = context.get_component(QUERY_RESULT_TEXTAREA).await;

        chatbox::append_chatbox_value(query_result_area.clone(), ("bot".into(), llm_response))
            .await;
    }
    context.set_ready_for(QUERY_RESULT_TEXTAREA).await;

    //text::append_and_send_stream_textarea_with_context(context.clone(), QUERY_STREAM_TEXTAREA, "").await;
}
