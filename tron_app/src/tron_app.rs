use axum::{
    body::Body,
    extract::{Host, Json, Path, Request, State},
    handler::HandlerWithoutStateExt,
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
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
    ActionExecutionMethod, TnComponentId, TnComponentState, TnComponentValue, TnContext, TnEvent, TnEventActions, TnSseMsgChannel
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

pub type SessionContext = RwLock<HashMap<tower_sessions::session::Id, TnContext>>;

pub type EventActions = RwLock<TnEventActions>;

type ContextBuilder = Arc<Box<dyn Fn() -> TnContext + Send + Sync>>;
type ActionFunctionTemplate = Arc<Box<dyn Fn(TnContext) -> TnEventActions + Send + Sync>>;
type LayoutFunction = Arc<Box<dyn Fn(TnContext) -> String + Send + Sync>>;

pub struct AppData {
    pub session_context: SessionContext,
    pub event_actions: EventActions,
    pub build_session_context: ContextBuilder,
    pub build_session_actions: ActionFunctionTemplate,
    pub build_layout: LayoutFunction,
}

pub async fn run(app_share_data: AppData, log_level: Option<&str>) {
    let log_level = if let Some(log_level) = log_level {
        log_level
    } else {
        "server=info,tower_http=info,tron_app=info"
    };

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                //.unwrap_or_else(|_| "server=debug,tower_http=debug".into()),
                .unwrap_or_else(|_| log_level.into()),
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
    if session.id().is_none() {
        session.insert("session_set", true).await.unwrap();
    };
    Html::from(index_html.to_string())
}

async fn get_session(session: Session, _: Request) {
    session.insert("session_set", true).await.unwrap();
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
        let mut session_contexts = app_data.session_context.write().await;
        let context = if session_contexts.contains_key(&session_id) {
            session_contexts.get_mut(&session_id).unwrap()
        } else {
            let new_context = tokio::task::block_in_place(|| (*app_data.build_session_context)());
            session_contexts.entry(session_id).or_insert(new_context)
        };

        {
            let context_guard = context.read().await;
            let (tx, rx) = tokio::sync::mpsc::channel(16);
            let mut sse_channels_guard = context_guard.sse_channels.write().await;
            *sse_channels_guard = Some(TnSseMsgChannel { tx, rx: Some(rx) });
        }
    };

    let context_guard = app_data.session_context.read().await;
    let context = context_guard.get(&session_id).unwrap().clone();
    let mut app_event_action_guard = app_data.event_actions.write().await;
    app_event_action_guard.clone_from(&tokio::task::block_in_place(|| {
        (*app_data.build_session_actions)(context.clone())
    }));

    let context = context_guard.get(&session_id).unwrap().clone();
    let layout = tokio::task::block_in_place(|| (*app_data.build_layout)(context.clone()));

    let context = context_guard.get(&session_id).unwrap().read().await;
    let components = context.components.read().await;
    let script = tokio::task::block_in_place(move || {
        components
            .iter()
            .flat_map(|(_, component)| component.blocking_read().get_script())
            .collect::<Vec<String>>()
    })
    .join("\n");

    let html = [layout, "<div>".to_string(), script, "</div>".to_string()].join("\n");
    Ok(Html::from(html))
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
    headers: HeaderMap,
    Path(tron_id): Path<TnComponentId>,
    Json(payload): Json<Value>,
    //request: Request,
) -> impl IntoResponse {
    tracing::debug!(target: "tron_app", "headers: {:?}", headers);

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return (StatusCode::FORBIDDEN, response_headers, Body::default());
    };
    {
        let context = app_data.session_context.read().await;
        if !context.contains_key(&session_id) {
            //return Err(StatusCode::FORBIDDEN);
            return (StatusCode::FORBIDDEN, response_headers, Body::default());
        }
    }

    tracing::debug!(target: "tron_app", "payload: {:?}", payload);

    if let Some(event_data) = match_event(&payload).await {
        let evt = event_data.tn_event;

        if evt.e_type == "change" {
            if let Some(value) = event_data.e_value {
                let context_guard = app_data.session_context.read().await;
                let context = context_guard.get(&session_id).unwrap().clone();
                context
                    .set_value_for_component(
                        &evt.e_target,
                        TnComponentValue::String(value),
                    )
                    .await;
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
                if *component.read().await.state() == TnComponentState::Ready {
                    component.write().await.set_state(TnComponentState::Pending);
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

    let context_guard = app_data.session_context.read().await;

    let components = &context_guard
        .get(&session_id)
        .unwrap()
        .read()
        .await
        .components;

    let mut component_guard = components.write().await;
    let target_guard = component_guard.get_mut(&tron_id).unwrap();

    let body = Body::new({
        let target = target_guard.read().await;
        tokio::task::block_in_place(|| target.render())
    });
    target_guard
        .write()
        .await
        .extra_headers()
        .iter()
        .for_each(|(k, v)| {
            response_headers.insert(
                HeaderName::from_bytes(k.as_bytes()).unwrap(),
                HeaderValue::from_bytes(v.as_bytes()).unwrap(),
            );
        });
    // println!("response_headers: {:?}", response_headers);
    (StatusCode::OK, response_headers, body)
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
        if let Some(rx) = channel_guard.as_mut().unwrap().rx.take() {
            ReceiverStream::new(rx).map(|v| Ok(sse::Event::default().data(v.clone())))
        } else {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
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
        tracing::debug!(target: "tron_app", "stream data_queue empty");
        return (StatusCode::NOT_FOUND, default_header, Body::default());
    }
    tracing::debug!(target: "tron_app", "stream data_queue NOT empty");
    let data_queue = futures_util::stream::iter(data_queue);

    let body = Body::from_stream(data_queue);
    (StatusCode::OK, header, body)
}
