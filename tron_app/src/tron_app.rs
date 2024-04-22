use axum::{
    body::Body,
    extract::{Host, Json, Path, Request, State},
    handler::HandlerWithoutStateExt,
    http::{header, HeaderMap, StatusCode, Uri},
    response::{
        sse::{self, KeepAlive},
        Html, IntoResponse, Redirect, Sse,
    },
    routing::{get, post},
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::RwLock;
use tron_components::{
    ActionExecutionMethod, ComponentId, ComponentState, Context, SseMessageChannel, TnEvent,
    TnEventActions,
};
//use std::sync::Mutex;
use std::{collections::HashMap, convert::Infallible, net::SocketAddr, path::PathBuf, sync::Arc};
use time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}

pub type SessionContext =
    RwLock<HashMap<tower_sessions::session::Id, Arc<RwLock<Context<'static>>>>>;

pub type EventActions = RwLock<TnEventActions>;

type ContextBuilder = dyn Fn() -> Arc<RwLock<Context<'static>>> + Send + Sync;
type ActionFunctionTemplate = dyn Fn() -> TnEventActions + Send + Sync;
type LayoutFunction = dyn Fn(Arc<RwLock<Context<'static>>>) -> String + Send + Sync;

pub struct AppData {
    pub session_context: SessionContext,
    pub event_actions: EventActions,
    pub build_session_context: Arc<Box<ContextBuilder>>,
    pub build_session_actions: Arc<Box<ActionFunctionTemplate>>,
    pub build_layout: Arc<Box<LayoutFunction>>,
}

pub async fn run(app_share_data: AppData) {
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

    // build our application with a route
    let routes = Router::new()
        .route("/", get(index))
        .route("/server_events", get(sse_event_handler))
        .route("/get_session", post(get_session))
        .route("/load_page", get(load_page))
        .route("/tron/:tron_id", get(tron_entry).post(tron_entry))
        .route(
            "/tron_streaming/:stream_id",
            get(tron_stream).post(tron_stream),
        )
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
    //println!("session Id: {}", session.id().unwrap());
}

async fn load_page(
    State(app_data): State<Arc<AppData>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return Err(StatusCode::FORBIDDEN);
    };

    {
        let context = tokio::task::block_in_place(|| (*app_data.build_session_context)());
        let mut session_contexts = app_data.session_context.write().await;
        let e = session_contexts
            .entry(session_id)
            .or_insert(context);
        {
            let context_guard = e.read().await;
            let (tx, rx) = tokio::sync::mpsc::channel(16);
            let mut sse_channels_guard = context_guard.sse_channels.write().await;
            *sse_channels_guard = Some(SseMessageChannel { tx, rx: Some(rx) });
        }
    };

    let mut app_event_action_guard = app_data.event_actions.write().await;
    app_event_action_guard.clone_from(&(*app_data.build_session_actions)());

    let context_guard = app_data.session_context.read().await;
    let context = context_guard.get(&session_id).unwrap().clone();
    let layout = tokio::task::block_in_place(|| (*app_data.build_layout)(context));

    Ok(Html::from(layout))
}

#[derive(Clone, Debug, Deserialize)]
struct EventData {
    tn_event: TnEvent,
    #[allow(dead_code)]
    #[serde(default)]
    e_value: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    e_data: Option<String>,
}

#[derive(Clone, Debug, Deserialize)]
struct TnEventExtend {
    event_data: EventData,
}

async fn match_event(payload: &Value) -> Option<EventData> {
    let r: Option<TnEventExtend> = serde_json::from_value(payload.clone()).unwrap_or(None);
    if let Some(r) = r {
        Some(r.event_data)
    } else {
        None
    }
}

async fn tron_entry(
    State(app_data): State<Arc<AppData>>,
    session: Session,
    _headers: HeaderMap,
    Path(tron_id): Path<ComponentId>,
    Json(payload): Json<Value>,
    //request: Request,
) -> Result<Html<String>, StatusCode> {
    // println!("req: {:?}", request);
    // println!("req body: {:?}", to_bytes(request.into_body(), usize::MAX).await );
    // let body_bytes =  to_bytes(request.into_body(), usize::MAX).await.unwrap();
    // println!("header: {:?}", _headers);
    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return Err(StatusCode::FORBIDDEN);
    };
    {
        let context = app_data.session_context.read().await;
        if !context.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    //println!("payload: {:?}", payload);

    if let Some(event_data) = match_event(&payload).await {
        //println!("event matched, event_data: {:?}", event_data);
        let evt = event_data.tn_event;

        if evt.e_type == "change" {
            if let Some(value) = event_data.e_value {
                let session_context_guard = app_data.session_context.read().await;
                let session_context = session_context_guard.get(&session_id).unwrap().clone();
                let context_guard = session_context.write().await;
                let id = context_guard.get_component_id(&evt.e_target.clone());
                let mut components_guard = context_guard.components.write().await;
                let component = components_guard.get_mut(&id).unwrap();
                component.set_value(tron_components::ComponentValue::String(value));
            }
        }

        let has_event_action = {
            let event_action_guard = app_data.event_actions.read().await;
            event_action_guard.contains_key(&evt)
        };

        if has_event_action {
            {
                let session_context_guard = app_data.session_context.read().await;
                let session_context = session_context_guard.get(&session_id).unwrap().clone();
                let context_guard = session_context.write().await;
                let id = context_guard.get_component_id(&evt.e_target.clone());
                let mut components_guard = context_guard.components.write().await;
                let component = components_guard.get_mut(&id).unwrap();
                // println!("pending set");
                if *component.state() == ComponentState::Ready {
                    component.set_state(ComponentState::Pending);
                };
            }

            let context_guard = app_data.session_context.read().await;
            let context = context_guard.get(&session_id).unwrap().clone();

            let event_action_guard = app_data.event_actions.write().await;
            let (action_exec_method, action_generator) =
                event_action_guard.get(&evt).unwrap().clone();

            let action = action_generator(context, evt, payload);
            match action_exec_method {
                ActionExecutionMethod::Spawn => {
                    tokio::task::spawn(action);
                }
                ActionExecutionMethod::Await => action.await,
            }
        }
    };

    let session_context_guard = app_data.session_context.read().await;

    let components = &session_context_guard
        .get(&session_id)
        .unwrap()
        .read()
        .await
        .components;

    let mut target_guard = components.write().await;
    let target = target_guard.get_mut(&tron_id).unwrap();
    Ok(target.render())
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
    State(app_data): State<Arc<AppData>>,
    session: Session,
) -> Result<Sse<impl Stream<Item = Result<sse::Event, Infallible>>>, StatusCode> {
    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return Err(StatusCode::FORBIDDEN);
    };

    {
        let context_guard = app_data.session_context.read().await;
        if !context_guard.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let stream = {
        let mut session_guard = app_data.session_context.write().await;
        let context_guard = session_guard.get_mut(&session_id).unwrap().write().await;
        let mut channel_guard = context_guard.sse_channels.write().await;
        let rx = channel_guard.as_mut().unwrap().rx.take().unwrap();
        ReceiverStream::new(rx).map(|v| Ok(sse::Event::default().data(v.clone())))
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

async fn tron_stream(
    State(app_data): State<Arc<AppData>>,
    session: Session,
    _headers: HeaderMap,
    Path(stream_id): Path<String>,
    //Json(_payload): Json<Value>,
    // request: Request,
) -> impl IntoResponse {
    //println!("streaming with id: {}", stream_id);
    let default_header = [
        (header::CONTENT_TYPE, "application/json".to_string()),
        (header::TRANSFER_ENCODING, "chunked".to_string()),
        (
            header::CACHE_CONTROL,
            "no-cache, must-revalidate".to_string(),
        ),
        (header::PRAGMA, "no-cache".to_string()),
    ];

    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return (StatusCode::FORBIDDEN, default_header, Body::default());
    };
    {
        let context_guard = app_data.session_context.read().await;
        if !context_guard.contains_key(&session_id) {
            return (StatusCode::FORBIDDEN, default_header, Body::default());
        }
    }

    {
        let session_guard = app_data.session_context.read().await;
        let context_guard = session_guard.get(&session_id).unwrap().read().await;
        let stream_data_guard = &context_guard.stream_data.read().await;
        if !stream_data_guard.contains_key(&stream_id) {
            return (StatusCode::NOT_FOUND, default_header, Body::default());
        }
    }

    let (protocol, data_queue) = {
        let session_guard = app_data.session_context.read().await;
        let context_guard = session_guard.get(&session_id).unwrap().write().await;
        let mut channels = context_guard.stream_data.write().await;
        //let channels = &mut stream_data_guard.stream_data;

        let (protocol, data_queue) = channels.get_mut(&stream_id).unwrap();
        let data_queue = data_queue.iter().cloned().collect::<Vec<_>>();
        (protocol.clone(), data_queue)
    };
    let header = [
        (header::CONTENT_TYPE, protocol),
        (
            header::CACHE_CONTROL,
            "no-cache, must-revalidate".to_string(),
        ),
        (header::TRANSFER_ENCODING, "chunked".to_string()),
        (header::PRAGMA, "no-cache".to_string()),
    ];

    let data_queue = data_queue
        .into_iter()
        .map(|bytes| -> Result<Vec<u8>, axum::Error> { Ok(bytes.to_vec()) })
        .collect::<Vec<_>>();

    if data_queue.is_empty() {
        return (StatusCode::NOT_FOUND, default_header, Body::default());
    }

    let data_queue = futures_util::stream::iter(data_queue);

    let body = Body::from_stream(data_queue);
    (StatusCode::OK, header, body)
}
