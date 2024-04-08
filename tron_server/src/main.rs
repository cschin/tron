use axum::{
    body::{to_bytes, Body},
    extract::{Host, Json, Path, Query, Request, State},
    handler::HandlerWithoutStateExt,
    http::{header, HeaderMap, StatusCode, Uri},
    response::{
        sse::{self, KeepAlive},
        Html, IntoResponse, Redirect, Sse,
    },
    routing::{get, post},
    BoxError, Error, Form, RequestExt, Router,
};
use axum_server::tls_rustls::RustlsConfig;
//use serde::{Deserialize, Serialize};
use tokio::sync::{
    mpsc::{channel, Receiver, Sender},
    Mutex, RwLock,
};

use serde_json::Value;
use tron::{ApplicationStates, ComponentBaseTrait, ComponentTypes, ComponentValue, TnButton};
//use std::sync::Mutex;
use std::{
    borrow::Borrow,
    collections::HashMap,
    convert::Infallible,
    net::SocketAddr,
    ops::{Deref, DerefMut},
    path::PathBuf,
    sync::Arc,
};
use time::Duration;
use tower_http::{cors::CorsLayer, follow_redirect::policy::PolicyExt, set_header::request};
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tower_sessions::{session_store, Expiry, MemoryStore, Session, SessionManagerLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use futures_util::{stream, Stream, StreamExt};
//use tokio_stream::StreamExt as _;
//use std::fs::File;
use base64::prelude::*;
use tokio_stream::wrappers::ReceiverStream;
//use tokio_tungstenite::{connect_async, tungstenite::protocol::Message};
//use tokio::io::{AsyncReadExt, AsyncWriteExt};
//use tungstenite::{connect, Message};
use bytes::Bytes;
use std::collections::VecDeque;
use tron_components as tron;

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}
struct SessionMessageChannel {
    tx: Sender<Json<Value>>,
    rx: Receiver<Json<Value>>,
}

type SessionApplicationStates =
    RwLock<HashMap<tower_sessions::session::Id, tron::ApplicationStates<'static>>>;

type SessionMessages = RwLock<HashMap<tower_sessions::session::Id, SessionMessageChannel>>;

struct AppShareData {
    app_states: SessionApplicationStates,
    app_messages: SessionMessages,
}

// type SessionApplicationSender = Arc<HashMap<tower_sessions::session::Id, Sender<String>>>;

// type SessionApplicationRecivers = Arc<HashMap<tower_sessions::session::Id, Receiver<String>>>;

#[tokio::main]
async fn main() {
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                //.unwrap_or_else(|_| "server=debug,tower_http=debug".into()),
                .unwrap_or_else(|_| "server=error,tower_http=error".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let session_store = MemoryStore::default();
    let session_layer = SessionManagerLayer::new(session_store.clone())
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(Duration::minutes(10)));

    let serve_dir = ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));

    let ports = Ports {
        http: 8080,
        https: 3001,
    };
    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports));

    // configure certificate and private key used by https
    let config = RustlsConfig::from_pem_file(
        PathBuf::from(".")
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(".").join("self_signed_certs").join("key.pem"),
    )
    .await
    .unwrap();

    // set app state
    let app_share_data = AppShareData {
        app_states: RwLock::new(HashMap::default()),
        app_messages: RwLock::new(HashMap::default()),
    };
    // build our application with a route
    let routes = Router::new()
        .route("/", get(index))
        .route("/server_events", get(sse_event_handler))
        .route("/get_session", post(get_session))
        .route("/load_page", get(load_page))
        .route("/tron/:tronid", get(tron_entry).post(tron_entry))
        .with_state(Arc::new(app_share_data));
    //.route("/button", get(tron::button));

    let app = Router::new()
        .merge(routes)
        .layer(session_layer)
        .layer(CorsLayer::very_permissive())
        .nest_service("/static", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([127, 0, 0, 1], 3001));

    axum_server::bind_rustls(addr, config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn index(session: Session, _: Request) -> Html<String> {
    let index_html = include_str!("../static/index.html");
    session.insert("session_set", true).await.unwrap();
    Html::from(index_html.to_string())
}

async fn get_session(session: Session, _: Request) {
    session.insert("session_set", true).await.unwrap();
    println!("session Id: {}", session.id().unwrap());
}

async fn load_page(
    State(app_share_data): State<Arc<AppShareData>>,
    session: Session,
) -> Html<String> {
    println!("session Id: {}", session.id().unwrap());
    let session_id = session.id().unwrap();
    {
        let mut session_app_states = app_share_data.app_states.write().await;
        if !session_app_states.contains_key(&session_id) {
            let e = session_app_states
                .entry(session_id)
                .or_insert(ApplicationStates {
                    components: HashMap::default(),
                    assets: HashMap::default(),
                });
            let c = &mut e.components;
            (0..100).for_each(|i| {
                let mut btn = TnButton::new(i, format!("btn-{:02}", i), format!("{:02}", i));
                btn.set_attribute("hx-post".to_string(), format!("/tron/{}", i));
                btn.set_attribute("hx-swap".to_string(), "outerHTML".to_string());
                btn.set_attribute("hx-target".to_string(), format!("#btn-{:02}", i));
                btn.set_attribute("id".to_string(), format!("btn-{:02}", i));
                c.insert(i, Box::new(btn));
            })
        }
        let mut session_app_messages = app_share_data.app_messages.write().await;
        if !session_app_messages.contains_key(&session_id) {
            let e = session_app_messages.entry(session_id).or_insert_with(|| {
                let (tx, mut rx) = tokio::sync::mpsc::channel(16);
                SessionMessageChannel { tx, rx }
            });
        }
    }
    let session_app_state = app_share_data.app_states.read().await;
    let c = &session_app_state.get(&session_id).unwrap().components;
    Html::from(
        c.iter()
            .map(|(k, v)| v.render().0)
            .collect::<Vec<String>>()
            .join(" "),
    )
}

// async fn get_session(session: Session, _: Request) {
//     session.insert("session_set", true).await.unwrap();
//     println!("{:?}", session.id());
// }

async fn tron_entry(
    State(app_share_data): State<Arc<AppShareData>>,
    session: Session,
    _headers: HeaderMap,
    Path(tron_id): Path<tron::ComponentId>,
    Json(payload): Json<Value>,
    //request: Request,
) -> Html<String> {
    // println!("req: {:?}", request);
    // println!("req body: {:?}", to_bytes(request.into_body(), usize::MAX).await );
    // let body_bytes =  to_bytes(request.into_body(), usize::MAX).await.unwrap();
    println!("payload: {:?}", payload);
    let session_id = session.id().expect("The session is expired");
    let mut session_app_states = app_share_data.app_states.write().await;
    let c = &mut session_app_states.get_mut(&session_id).unwrap().components;

    let e = c.get_mut(&tron_id).unwrap();
    let v = match e.value() {
        ComponentValue::String(s) => s.parse::<u32>().unwrap(),
        _ => 0,
    };
    e.set_value(ComponentValue::String(format!("{}", v + 1)));

    {
        let messages = app_share_data.app_messages.write().await;
        let mc = messages.get(&session_id).unwrap();
        let tx = mc.tx.clone();
        println!("write to msg queue: {:?}", payload);
        tx.send(axum::Json(payload)).await;
    }

    e.render()
}

#[allow(dead_code)]
async fn redirect_http_to_https(ports: Ports) {
    fn make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError> {
        let mut parts = uri.into_parts();

        parts.scheme = Some(axum::http::uri::Scheme::HTTPS);

        if parts.path_and_query.is_none() {
            parts.path_and_query = Some("/".parse().unwrap());
        }

        let https_host = host.replace(&ports.http.to_string(), &ports.https.to_string());
        parts.authority = Some(https_host.parse()?);

        Ok(Uri::from_parts(parts)?)
    }

    let redirect = move |Host(host): Host, uri: Uri| async move {
        match make_https(host, uri, ports) {
            Ok(uri) => Ok(Redirect::permanent(&uri.to_string())),
            Err(error) => {
                tracing::warn!(%error, "failed to convert URI to HTTPS");
                Err(StatusCode::BAD_REQUEST)
            }
        }
    };

    let addr = SocketAddr::from(([127, 0, 0, 1], ports.http));
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    tracing::debug!("listening on {}", listener.local_addr().unwrap());
    axum::serve(listener, redirect.into_make_service())
        .await
        .unwrap();
}

async fn sse_event_handler(
    State(app_share_data): State<Arc<AppShareData>>,
    session: Session,
) -> Sse<impl Stream<Item = Result<sse::Event, Infallible>>> {

    let stream = {
        let session_id = session.id().expect("The session is expired");
        let mut messages = app_share_data.app_messages.write().await;
        let (_, rx_dummy) = tokio::sync::mpsc::channel::<Json<Value>>(16);
        let rx = &mut messages.get_mut(&session_id).unwrap().rx;
        let rx = std::mem::replace(rx, rx_dummy);
        ReceiverStream::new(rx)
        .map(|v| Ok(sse::Event::default().data(v.clone().to_string())))
    };
    Sse::new(stream).keep_alive(KeepAlive::default())
}
