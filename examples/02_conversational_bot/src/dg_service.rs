use std::{collections::VecDeque, sync::Arc, time::SystemTime};

use crate::{PLAYER, STATUS, TRON_APP};
use bytes::{BufMut, Bytes, BytesMut};
use futures::channel::mpsc::Receiver;
use futures::sink::SinkExt;
use futures::stream::StreamExt;
use http::Request;
use serde::{Deserialize, Serialize};
use serde_json::json;
use thiserror::Error;
use tokio::sync::{mpsc::Sender, Mutex};
use tokio_tungstenite::tungstenite::{self, Message};
use tron_app::tron_components::{
    audio_player::start_audio, text::append_and_send_stream_textarea_with_context, TnAsset,
    TnComponentState, TnComponentValue, TnContext, TnServiceRequestMsg,
};
use url::Url;

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Alternatives {
    pub transcript: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct Channel {
    pub alternatives: Vec<Alternatives>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamResponse {
    UtteranceEnd {
        last_word_end: f64,
    },
    TranscriptResponse {
        duration: f64,
        is_final: bool,
        speech_final: bool,
        channel: Channel,
    },
    TerminalResponse {
        request_id: String,
        created: String,
        duration: f64,
        channels: u32,
    },
}

#[derive(Debug, Error)]
pub enum DeepgramError {
    /// No source was provided to the request builder.
    #[allow(dead_code)]
    #[error("No source was provided to the request builder.")]
    NoSource,

    /// The Deepgram API returned an error.
    #[allow(dead_code)]
    #[error("The Deepgram API returned an error.")]
    DeepgramApiError {
        /// Error message from the Deepgram API.
        body: String,

        /// Underlying [`reqwest::Error`] from the HTTP request.
        err: reqwest::Error,
    },

    /// Something went wrong when generating the http request.
    #[error("Something went wrong when generating the http request: {0}")]
    HttpError(#[from] http::Error),

    /// Something went wrong when making the HTTP request.
    #[error("Something went wrong when making the HTTP request: {0}")]
    ReqwestError(#[from] reqwest::Error),

    /// Something went wrong during I/O.
    #[error("Something went wrong during I/O: {0}")]
    IoError(#[from] std::io::Error),

    /// Something went wrong with WS.
    #[error("Something went wrong with WS: {0}")]
    WsError(#[from] tungstenite::Error),

    /// Something went wrong during serialization/deserialization.
    #[error("Something went wrong during serialization/deserialization: {0}")]
    SerdeError(#[from] serde_json::Error),
}

type Result<T> = std::result::Result<T, DeepgramError>;

/// Generate a random key for the `Sec-WebSocket-Key` header.
pub fn generate_key() -> String {
    // a base64-encoded (see Section 4 of [RFC4648]) value that,
    // when decoded, is 16 bytes in length (RFC 6455)
    let r: [u8; 16] = rand::random();
    data_encoding::BASE64.encode(&r)
}

pub async fn deepgram_transcript_service(
    audio_rx: tokio::sync::mpsc::Receiver<core::result::Result<Bytes, DeepgramError>>,
    transcript_tx: Sender<StreamResponse>,
) -> core::result::Result<(), DeepgramError> {
    let mut dg_response = trx_service(audio_rx).await.unwrap();
    while let Some(result) = dg_response.next().await {
        match result {
            Ok(r) => {
                transcript_tx.send(r).await.expect("transcript send fail");
            }
            Err(e) => {
                tracing::debug!(target: TRON_APP, "in deepgram_transcript_service, Err {:?}", e );
                return Err(e);
            }
        }
    }
    drop(dg_response);
    Ok(())
}

pub async fn trx_service(
    audio_rx: tokio::sync::mpsc::Receiver<Result<Bytes>>,
) -> Result<Receiver<Result<StreamResponse>>> {
    // This unwrap is safe because we're parsing a static.
    let mut base = Url::parse("wss://api.deepgram.com/v1/listen").unwrap();

    {
        let query_pairs = &mut base.query_pairs_mut();
        query_pairs.append_pair("endpointing", "1250");
        query_pairs.append_pair("utterance_end_ms", "1250");
        query_pairs.append_pair("interim_results", "true");
        query_pairs.append_pair("model", "nova-2-conversationalai");
        query_pairs.append_pair("punctuate", "true");
    }

    let api_key = std::env::var("DG_API_KEY").unwrap();

    let request = Request::builder()
        .method("GET")
        .uri(base.to_string())
        .header("authorization", format!("token {}", api_key))
        .header("sec-websocket-key", generate_key())
        .header("host", "api.deepgram.com")
        .header("connection", "upgrade")
        .header("upgrade", "websocket")
        .header("sec-websocket-version", "13")
        .body(())?;
    let (ws_stream, _response) = tokio_tungstenite::connect_async(request).await?;
    let (mut write, mut read) = ws_stream.split();
    let (mut tx, rx) = futures::channel::mpsc::channel::<Result<StreamResponse>>(1);
    tracing::info!(target: TRON_APP, "new dg ws established");

    let data_stream = tokio_stream::wrappers::ReceiverStream::new(audio_rx);

    let mut source =
        data_stream.map(|res| res.map(|bytes| Message::binary(Vec::from(bytes.as_ref()))));

    let send_task = async move {
        loop {
            match source.next().await {
                None => {
                    let _ = write.close().await;
                    break;
                }
                Some(Ok(frame)) => {
                    if let Err(_w) = write.send(frame).await {
                        break;
                    };
                }
                Some(e) => {
                    let _ = dbg!(e);
                    break;
                }
            }
        }
        drop(write);
        drop(source);
    };

    let recv_task = async move {
        loop {
            match read.next().await {
                None => {
                    let _ = tx.close().await;
                    break;
                }
                Some(Ok(msg)) => {
                    if let Message::Text(txt) = msg {
                        let resp = serde_json::from_str(&txt).map_err(DeepgramError::from);
                        tx.send(resp)
                            .await
                            .expect("message sent from ws to rust fails");
                    }
                }
                Some(e) => {
                    tracing::info!(target: TRON_APP, "in recv_task, message received error len={:?}", e);
                    let _ = dbg!(e);
                    break;
                }
            }
        }
        drop(tx);
        drop(read);
    };

    tokio::spawn(async move {
        //tokio::join!(send_task, recv_task);
        tokio::select! {
            _ = send_task => {
                tracing::debug!(target: TRON_APP, "dg websocket send task end first");
            },
            _ = recv_task => {
                tracing::debug!(target: TRON_APP, "dg websocket recv task end first");
            },
        }
    });

    Ok(rx)
}

pub async fn tts_service(
    mut rx: tokio::sync::mpsc::Receiver<TnServiceRequestMsg>,
    context: TnContext,
) {
    let reqwest_client = reqwest::Client::new();
    let dg_api_key: String = std::env::var("DG_API_KEY").unwrap();
    let msg_queue = Arc::new(Mutex::new(VecDeque::<String>::new()));
    let msg_queue1 = msg_queue.clone();

    // put the message into a buffer
    // when a new LLM response comes in, clean up the buffer to stop TTS to emulate the
    // bot's response is interrupted. Currently, the TTS is done by sentence by sentence.
    // The interruption will be done after the current sentence is finished.
    tokio::spawn(async move {
        while let Some(req_msg) = rx.recv().await {
            if req_msg.request == "new_llm_response" {
                msg_queue1.lock().await.clear();
                if let TnAsset::String(llm_response) = req_msg.payload {
                    tracing::info!(target: TRON_APP, "new llm response received: {}", llm_response);
                    if !llm_response.is_empty() {
                        msg_queue1.lock().await.push_back(llm_response);
                    }
                }
            } else if let TnAsset::String(llm_response) = req_msg.payload {
                if !llm_response.is_empty() {
                    msg_queue1.lock().await.push_back(llm_response);
                }
            };
        }
    });

    loop {
        let llm_response = if msg_queue.lock().await.is_empty() {
            tokio::time::sleep(std::time::Duration::from_millis(200)).await;
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
        tracing::info!(target:TRON_APP, "json_data: {}", json_data);
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
        tracing::debug!( target:TRON_APP, "response: {:?}", response);

        let duration = time.elapsed().unwrap();
        append_and_send_stream_textarea_with_context(
            context.clone(),
            STATUS,
            &format!("TTS request request done: {duration:?}\n"),
        )
        .await;
        // send TTS data to the player stream out
        while let Some(chunk) = response.chunk().await.unwrap() {
            tracing::debug!( target:TRON_APP, "Chunk: {}", chunk.len());
            {
                let context_guard = context.write().await;
                let mut stream_data_guard = context_guard.stream_data.write().await;
                let player_data = stream_data_guard.get_mut(PLAYER).unwrap();
                let mut data = BytesMut::new();
                data.put(&chunk[..]);
                player_data.1.push_back(data);
            }
        }
        {
            let context_guard = context.write().await;
            let mut stream_data_guard = context_guard.stream_data.write().await;
            let player_data = stream_data_guard.get_mut(PLAYER).unwrap();
            // ensure we don't send empty data to the player, or the empty data can trigger an infinite loop
            if player_data.1.is_empty() {
                continue;
            }
        }

        // for debug
        // {
        //     let player_guard = context.get_component(PLAYER).await;
        //     let player = player_guard.read().await;
        //     let audio_ready = player.state();
        //     let context_guard = context.write().await;
        //     let mut stream_data_guard = context_guard.stream_data.write().await;
        //     let player_data = stream_data_guard.get_mut(PLAYER).unwrap();

        //     tracing::info!( target:TRON_APP, "Start: player status {:?}", audio_ready);
        //     tracing::info!( target:TRON_APP, "player data len: {:?}", player_data.1.len());
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
                let player_guard = context.get_component(PLAYER).await;
                let player = player_guard.read().await;
                player.state().clone()
            };
            tracing::debug!( target:TRON_APP, "wait for the player to ready to play {:?}", audio_ready);
            match audio_ready {
                TnComponentState::Ready => {
                    let player = context.get_component(PLAYER).await;
                    let sse_tx = context.get_sse_tx().await;
                    start_audio(player.clone(), sse_tx).await;
                    tracing::debug!( target:TRON_APP, "set audio to play");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    break;
                }
                _ => {
                    tracing::debug!( target:TRON_APP, "Waiting for audio to be ready");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }

        // This loop waits for the audio player component to be in the "Ready" state.
        // Once the player is ready, it clean up the stream buffer.
        loop {
            let audio_ready = {
                let player_guard = context.get_component(PLAYER).await;
                let player = player_guard.read().await;
                player.state().clone()
            };
            tracing::debug!( target:TRON_APP, "wait for the player in the ready state to clean up the stream data: {:?}", audio_ready);
            match audio_ready {
                TnComponentState::Ready => {
                    // clear the stream buffer
                    let context_guard = context.write().await;
                    let mut stream_data_guard = context_guard.stream_data.write().await;
                    stream_data_guard.get_mut(PLAYER).unwrap().1.clear();
                    tracing::debug!( target:TRON_APP, "clean audio stream data");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                    break;
                }
                _ => {
                    tracing::debug!( target:TRON_APP, "Waiting for audio to be ready");
                    tokio::time::sleep(std::time::Duration::from_millis(200)).await;
                }
            }
        }
    }
}
