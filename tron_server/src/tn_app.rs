use axum::{
    extract::{Host, Json, Path, Request, State},
    handler::HandlerWithoutStateExt,
    http::{HeaderMap, StatusCode, Uri},
    response::{
        sse::{self, KeepAlive},
        Html, Redirect, Sse,
    },
    routing::{get, post},
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
//use serde::{Deserialize, Serialize};
use tokio::sync::{
    mpsc::{Receiver, Sender},
    RwLock,
};

use serde_json::Value;
use tron_components::{ComponentId, ComponentState, Components, TnEvent, TnEventActions};
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

use futures_util::{Stream, StreamExt};
use tokio_stream::wrappers::ReceiverStream;

#[derive(Clone, Copy)]
struct Ports {
    http: u16,
    https: u16,
}
pub struct SessionMessageChannel {
    tx: Sender<Json<Value>>,
    rx: Option<Receiver<Json<Value>>>, // this will be moved out and replaced by None
}

pub type SessionComponents =
    RwLock<HashMap<tower_sessions::session::Id, Arc<RwLock<Components<'static>>>>>;

pub type SessionSeeChannels = RwLock<HashMap<tower_sessions::session::Id, SessionMessageChannel>>;

pub type EventActions = RwLock<TnEventActions>;
pub struct AppData {
    pub session_components: SessionComponents,
    pub session_sse_channels: SessionSeeChannels,
    pub event_action: EventActions,
    pub build_default_session_components: Arc<Box<dyn Fn() -> Components<'static> + Send + Sync>>,
    pub build_default_session_actions: Arc<Box<dyn Fn() -> TnEventActions + Send + Sync>>,
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
    State(app_data): State<Arc<AppData>>,
    session: Session,
) -> Result<Html<String>, StatusCode> {
    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return Err(StatusCode::FORBIDDEN);
    };

    {
        let mut session_components = app_data.session_components.write().await;

        if !session_components.contains_key(&session_id) {
            let components = Arc::new(RwLock::new((*app_data.build_default_session_components)()));
            session_components.entry(session_id).or_insert(components);
        }
    }

    {
        let mut session_app_sse_channels = app_data.session_sse_channels.write().await;
        // we get new message channel for new session as the sse does not survived through reload
        let (tx, rx) = tokio::sync::mpsc::channel(16);
        session_app_sse_channels.insert(session_id, {
            SessionMessageChannel {
                tx: tx.clone(),
                rx: Some(rx),
            }
        });
    }

    let mut app_event_action_guard = app_data.event_action.write().await;
    let _ = (*app_data.build_default_session_actions)();
    app_event_action_guard.clone_from(&(*app_data.build_default_session_actions)());

    let session_components = app_data.session_components.read().await;
    let components = &session_components
        .get(&session_id)
        .unwrap()
        .read()
        .await
        .components;

    Ok(Html::from({
        let mut components = components
            .iter()
            .map(|(k, v)| (*k, v.render().0))
            .collect::<Vec<(u32, String)>>();
        components.sort();
        components
            .into_iter()
            .map(|v| v.1)
            .collect::<Vec<String>>()
            .join("\n")
    }))
}

async fn match_event(payload: &Value) -> Option<(String, String, String)> {
    let mut evt_target = String::default();
    let mut evt_type = String::default();
    let mut state = String::default();
    if let serde_json::Value::Object(obj) = payload.clone() {
        for (k, v) in obj.clone().iter() {
            if *k == "evt_target" {
                if let Value::String(s) = v {
                    evt_target.clone_from(s);
                }
            }
            if *k == "evt_type" {
                if let Value::String(s) = v {
                    evt_type.clone_from(s);
                }
            }
            if *k == "state" {
                if let Value::String(s) = v {
                    state.clone_from(s);
                }
            }
        }
    }
    if evt_target.is_empty() || evt_type.is_empty() {
        None
    } else {
        Some((evt_target, evt_type, state))
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
    //println!("header: {:?}", _headers);
    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return Err(StatusCode::FORBIDDEN);
    };
    {
        let components = app_data.session_components.read().await;
        if !components.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    println!("payload: {:?}", payload);

    if let Some((evt_target, evt_type, state)) = match_event(&payload).await {
        println!("event matched");
        let evt = TnEvent {
            evt_target,
            evt_type,
            state,
        };
        let has_event_action = {
            let event_action_guard = app_data.event_action.read().await;
            event_action_guard.contains_key(&evt)
        }; 
        if has_event_action {
            {
                let session_components_guard = app_data.session_components.read().await;
                let session_components = session_components_guard.get(&session_id).unwrap().clone();
                let mut components_guard = session_components.write().await;
                let component =
                    components_guard.get_mut_component_by_tron_id(&evt.evt_target.clone());
                component.set_state(ComponentState::Pending);
            }

            let session_components = app_data.session_components.read().await;
            let components = session_components.get(&session_id).unwrap().clone();

            let session_sse_channel_guard = app_data.session_sse_channels.read().await;
            let tx = session_sse_channel_guard
                .get(&session_id)
                .unwrap()
                .tx
                .clone();

            let event_action_guard = app_data.event_action.write().await;
            let action_generator = event_action_guard.get(&evt).unwrap().clone();

            let action = action_generator(components, tx, evt.clone());
            tokio::task::spawn(action);
            //action.await; // this won't allow two event triggered at the same time
        }
    };

    let session_components_guard = app_data.session_components.read().await;

    let components = &session_components_guard
        .get(&session_id)
        .unwrap()
        .read()
        .await
        .components;

    let target = components.get(&tron_id).unwrap();

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
        let channels_guard = app_data.session_sse_channels.read().await;
        if !channels_guard.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let stream = {
        let mut channels_guard = app_data.session_sse_channels.write().await;
        let channel = channels_guard.get_mut(&session_id).unwrap();
        let rx = channel.rx.take().unwrap();
        ReceiverStream::new(rx).map(|v| Ok(sse::Event::default().data(v.clone().to_string())))
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}
