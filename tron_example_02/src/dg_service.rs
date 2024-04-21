use bytes::Bytes;
use futures::channel::mpsc::{self, Receiver};
use futures::stream::StreamExt;
use futures::{sink::SinkExt, Stream};
use http::Request;
use serde::{Deserialize, Serialize};
use thiserror::Error;
use tokio_stream::wrappers::ReceiverStream;
use tokio_tungstenite::tungstenite::{self, Message};
use url::Url;

#[derive(Debug, Serialize, Deserialize)]
pub struct Word {
    pub word: String,
    pub start: f64,
    pub end: f64,
    pub confidence: f64,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Alternatives {
    pub transcript: String,
    pub words: Vec<Word>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Channel {
    pub alternatives: Vec<Alternatives>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(untagged)]
pub enum StreamResponse {
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
    #[error("No source was provided to the request builder.")]
    NoSource,

    /// The Deepgram API returned an error.
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

pub async fn trx_service(
    audio_rx: tokio::sync::mpsc::Receiver<Result<Bytes>>,
) -> Result<Receiver<Result<StreamResponse>>> {
    // This unwrap is safe because we're parsing a static.
    let mut base =
        Url::parse("wss://api.deepgram.com/v1/listen?endpointing=1000&model=nova-2-phonecall")
            .unwrap();
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
    let (ws_stream, response) = tokio_tungstenite::connect_async(request).await?;
    println!("response: {:?}", response);
    let (mut write, mut read) = ws_stream.split();
    let (mut tx, rx) = mpsc::channel::<Result<StreamResponse>>(1);
    println!("new dg ws established");

    let data_stream = Some(tokio_stream::wrappers::ReceiverStream::new(audio_rx));

    let mut source = data_stream
        .ok_or(DeepgramError::NoSource)?
        .map(|res| res.map(|bytes| Message::binary(Vec::from(bytes.as_ref()))));

    let send_task = async move {
        loop {
            match source.next().await {
                None => {
                    println!("source next is none");
                    write.close().await;
                    break;
                }
                Some(Ok(frame)) => {
                    // This unwrap is not safe.
                    println!("frame sent len={}", frame.len());
                    if let Err(_w) = write.send(frame).await {
                        break;
                    };
                }
                Some(e) => {
                    println!("error sending frame: {:?}", e);
                    let _ = dbg!(e);
                    break;
                }
            }
        }
        //write.send(Message::binary([])).await.expect("final empty send error");
        drop(source);
        println!("send loop end");
        // This unwrap is not safe.
    };

    let recv_task = async move {
        loop {
            match read.next().await {
                None => {
                    println!("received none");
                    tx.close().await;
                    break;
                }
                Some(Ok(msg)) => {
                    if let Message::Text(txt) = msg {
                        let resp = serde_json::from_str(&txt).map_err(DeepgramError::from);
                        println!("message received: {:?}", resp);
                        if let Ok(StreamResponse::TerminalResponse { .. }) = resp {
                            tx.send(resp)
                                .await
                                .expect("message sent from ws to rust fails");
                            //break
                        } else {
                            tx.send(resp)
                                .await
                                .expect("message sent from ws to rust fails");
                        }
                    }
                }
                Some(e) => {
                    println!("message received error len={:?}", e);
                    let _ = dbg!(e);
                    break;
                }
            }
        }
        drop(read);
        println!("recv task end");
    };

    tokio::spawn(async move {
        //tokio::join!(send_task, recv_task);
        tokio::select! {
            _ = send_task => {println!("send task finish first");},
            _ = recv_task => {println!("recv task finish first");},
        }
    });

    Ok(rx)
}