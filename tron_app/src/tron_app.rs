use axum::{
    body::Body,
    extract::{Host, Json, OriginalUri, Path, Query, Request, State},
    handler::HandlerWithoutStateExt,
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri},
    middleware::{self, Next},
    response::{
        sse::{self, KeepAlive},
        Html, IntoResponse, Redirect, Sse,
    },
    routing::get,
    BoxError, Router,
};
use axum_server::tls_rustls::RustlsConfig;
use serde::Deserialize;
use serde_json::Value;
use tokio::sync::RwLock;
use tron_components::{
    TnActionExecutionMethod, TnAsset, TnComponentIndex, TnComponentValue, TnContext, TnEvent,
    TnEventActions, TnSseMsgChannel,
};
//use std::sync::Mutex;
use std::{
    collections::HashMap, convert::Infallible, env, net::SocketAddr, path::PathBuf, sync::Arc,
};
use time::Duration;
use tower_http::cors::CorsLayer;
use tower_http::{
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

/// Represents HTTP and HTTPS ports.
///
/// This struct encapsulates the HTTP and HTTPS ports used in a network configuration.
#[derive(Clone, Copy)]
pub struct Ports {
    http: u16,
    https: u16,
}

/// Represents a session ID used in Tower Sessions.
pub type SessionId = tower_sessions::session::Id;

/// Represents a session context containing mappings of session IDs to Tron contexts.
pub type SessionContext = RwLock<HashMap<SessionId, TnContext>>;

/// Represents event actions protected by a reader-writer lock.
pub type EventActions = RwLock<TnEventActions>;

/// Alias for a context builder function.
type ContextBuilder = Arc<Box<dyn Fn() -> TnContext + Send + Sync>>;

/// Alias for an action function template, which generates event actions from a context.
type ActionFunctionTemplate = Arc<Box<dyn Fn(TnContext) -> TnEventActions + Send + Sync>>;

/// Alias for a layout function, which generates HTML layout from a context.
type LayoutFunction = Arc<Box<dyn Fn(TnContext) -> String + Send + Sync>>;

/// Represents application data used in a Tron application.
///
/// This struct encapsulates various data components used in a Tron application,
/// including session context, session expiry mappings, event actions, context builder function,
/// action function template, and layout function.
pub struct AppData {
    pub context: SessionContext,
    pub session_expiry: RwLock<HashMap<SessionId, time::OffsetDateTime>>,
    pub event_actions: EventActions,
    pub build_context: ContextBuilder,
    pub build_actions: ActionFunctionTemplate,
    pub build_layout: LayoutFunction,
}
/// Represents configuration options for a Tron application.
///
/// This struct encapsulates various configuration options for a Tron application,
/// including the network address, ports, Cognito login flag, and log level.
pub struct AppConfigure {
    pub address: [u8; 4],
    pub ports: Ports,
    pub cognito_login: bool,
    pub log_level: Option<&'static str>,
}

/// Implements the default trait for creating a default instance of `AppConfigure`.
///
/// This implementation creates a default instance of `AppConfigure` with default values
/// for network address, ports, Cognito login flag, and log level.
impl Default for AppConfigure {
    /// Creates a default instance of `AppConfigure`.
    ///
    /// The default values are as follows:
    ///
    /// * `address`: `[127, 0, 0, 1]`
    /// * `ports`: `Ports { http: 8080, https: 3001 }`
    /// * `cognito_login`: `false`
    /// * `log_level`: `Some("server=info,tower_http=info,tron_app=info")`
    fn default() -> Self {
        let address = [127, 0, 0, 1];
        let ports = Ports {
            http: 8080,
            https: 3001,
        };
        let cognito_login = false;
        let log_level = Some("server=info,tower_http=info,tron_app=info");
        Self {
            address,
            ports,
            cognito_login,
            log_level,
        }
    }
}

/// Runs the Tron application with the provided application data and configuration.
///
/// This function starts the Tron application with the given `AppData` and `AppConfigure` parameters.
/// It configures logging, session management, routes, and serves the application over HTTPS.
///
/// # Arguments
///
/// * `app_share_data` - Application data shared across the application.
/// * `config` - Application configuration options.
///
/// # Examples
///
/// ```
/// // Usage example:
/// let app_data = AppData { /* AppData fields */ };
/// let config = AppConfigure { /* AppConfigure fields */ };
/// run(app_data, config).await;
/// `
pub async fn run(app_share_data: AppData, config: AppConfigure) {
    let log_level = config.log_level;
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
        .with_expiry(Expiry::OnInactivity(Duration::minutes(20)));

    let serve_dir = ServeDir::new("static").not_found_service(ServeFile::new("static/index.html"));

    let ports = config.ports;
    // optional: spawn a second server to redirect http requests to this server
    tokio::spawn(redirect_http_to_https(ports));

    // configure certificate and private key used by https
    let tls_config = RustlsConfig::from_pem_file(
        PathBuf::from(".")
            .join("self_signed_certs")
            .join("cert.pem"),
        PathBuf::from(".").join("self_signed_certs").join("key.pem"),
    )
    .await
    .unwrap();

    // build our application with a route
    let app_share_data = Arc::new(app_share_data);
    let routes = Router::new()
        .route("/", get(index))
        .route("/server_events", get(sse_event_handler))
        .route("/load_page", get(load_page))
        .route("/tron/:tron_id", get(tron_entry).post(tron_entry))
        .route(
            "/tron_streaming/:stream_id",
            get(tron_stream).post(tron_stream),
        )
        .with_state(app_share_data.clone());
    //.route("/button", get(tron::button));

    let auth_routes = Router::new()
        .route("/login", get(login_handler))
        .route("/logout", get(logout_handler))
        .route("/logged_out", get(logged_out))
        .route("/cognito_callback", get(cognito_callback))
        .with_state(app_share_data.clone());

    let app_routes = if config.cognito_login {
        Router::new().merge(routes).merge(auth_routes)
    } else {
        routes
    };

    let app = if config.cognito_login {
        app_routes
            .nest_service("/static", serve_dir.clone())
            .fallback_service(serve_dir)
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::very_permissive())
            .layer(middleware::from_fn(check_token))
            .layer(session_layer)
    } else {
        app_routes
            .nest_service("/static", serve_dir.clone())
            .fallback_service(serve_dir)
            .layer(TraceLayer::new_for_http())
            .layer(CorsLayer::very_permissive())
            .layer(session_layer)
    };

    let addr = SocketAddr::from((config.address, ports.https));

    axum_server::bind_rustls(addr, tls_config)
        .serve(app.into_make_service())
        .await
        .unwrap();
}

/// Handles requests to the index route.
///
/// This asynchronous function handles requests to the index route ("/").
/// It retrieves the HTML content of the index page, checks if the session is empty,
/// sets the session if it's empty, updates the session expiry, and returns the HTML response.
///
/// # Arguments
///
/// * `session` - Session information associated with the request.
/// * `app_data` - Application data shared across the application.
///
/// # Returns
///
/// An implementation of `IntoResponse` representing the response to the request.
///
async fn index(
    session: Session,
    State(app_data): State<Arc<AppData>>,
    _: Request,
) -> impl IntoResponse {
    let index_html = include_str!("../static/index.html");
    if session.is_empty().await {
        // the line below is necessary to make sure the session is set
        session.insert("session_set", true).await.unwrap();
        tracing::debug!(target:"tron_app", "set session");
        Redirect::to("/").into_response()
    } else {
        let mut session_expiry = app_data.session_expiry.write().await;
        session_expiry.insert(session.id().unwrap(), session.expiry_date());
        Html::from(index_html.to_string()).into_response()
    }
}


/// Loads the page content for the Tron application.
///
/// This asynchronous function loads the page content for the Tron application.
/// It retrieves or creates a session context, initializes SSE channels, generates event actions,
/// builds the page layout, and assembles the HTML content with scripts.
///
/// # Arguments
///
/// * `app_data` - Application data shared across the application.
/// * `session` - Session information associated with the request.
///
/// # Returns
///
/// A result containing the HTML content if successful, or a status code indicating an error.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if the session ID is not available.
///
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
        let mut session_contexts = app_data.context.write().await;
        let context = if session_contexts.contains_key(&session_id) {
            session_contexts.get_mut(&session_id).unwrap()
        } else {
            let new_context = tokio::task::block_in_place(|| (*app_data.build_context)());
            session_contexts.entry(session_id).or_insert(new_context)
        };

        {
            let context_guard = context.read().await;
            let (tx, rx) = tokio::sync::mpsc::channel(16);
            let mut sse_channels_guard = context_guard.sse_channel.write().await;
            *sse_channels_guard = Some(TnSseMsgChannel { tx, rx: Some(rx) });
        }
    };

    let context_guard = app_data.context.read().await;
    let context = context_guard.get(&session_id).unwrap().clone();
    let mut app_event_action_guard = app_data.event_actions.write().await;
    app_event_action_guard.clone_from(&tokio::task::block_in_place(|| {
        (*app_data.build_actions)(context.clone())
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

    let html = [layout, script].join("\n");
    Ok(Html::from(html))
}

/// Represents event data associated with a Tron event.
///
/// This struct encapsulates event data associated with a Tron event,
/// including the event itself (`tn_event`), an optional event value (`e_value`),
/// and an optional event data (`e_data`).
#[derive(Clone, Debug, Deserialize)]
struct TnEventData {
    tn_event: TnEvent,
    #[allow(dead_code)]
    #[serde(default)]
    e_value: Option<String>,
    #[allow(dead_code)]
    #[serde(default)]
    e_data: Option<String>,
}

/// Represents extended event data associated with a Tron event.
///
/// This struct encapsulates extended event data associated with a Tron event,
/// including the event data itself (`event_data`).
#[derive(Clone, Debug, Deserialize)]
struct TnEventExtend {
    event_data: TnEventData,
}

///
/// This asynchronous function matches event data from a payload. It attempts to deserialize
/// the payload into a `TnEventExtend` struct and extracts the `TnEventData` if successful.
///
/// # Arguments
///
/// * `payload` - The payload containing event data in JSON format.
///
/// # Returns
///
/// An option containing the event data if deserialization is successful, or `None` otherwise.
///
async fn match_event(payload: &Value) -> Option<TnEventData> {
    let r: Option<TnEventExtend> = serde_json::from_value(payload.clone()).unwrap_or(None);
    if let Some(r) = r {
        Some(r.event_data)
    } else {
        None
    }
}


/// Handles requests to the Tron entry endpoint.
///
/// This asynchronous function handles requests to the Tron entry endpoint, which is responsible
/// for processing Tron events. It extracts event data from the payload, sets up event handling,
/// executes event actions, and returns the appropriate response.
///
/// # Arguments
///
/// * `app_data` - Application data shared across the application.
/// * `session` - Session information associated with the request.
/// * `headers` - Headers included in the request.
/// * `tron_index` - The index of the Tron component associated with the request.
/// * `payload` - JSON payload containing event data.
///
async fn tron_entry(
    State(app_data): State<Arc<AppData>>,
    session: Session,
    headers: HeaderMap,
    Path(tron_index): Path<TnComponentIndex>,
    Json(payload): Json<Value>,
    //request: Request,
) -> impl IntoResponse {
    tracing::debug!(target: "tron_app", "headers: {:?}", headers);

    //let _hx_trigger = headers.get("hx-trigger");
    let hx_target: Option<String> = headers
        .get("hx-target")
        .map(|hx_target| hx_target.to_str().unwrap().to_string());

    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return (StatusCode::FORBIDDEN, response_headers, Body::default());
    };
    {
        let context = app_data.context.read().await;
        if !context.contains_key(&session_id) {
            //return Err(StatusCode::FORBIDDEN);
            return (StatusCode::FORBIDDEN, response_headers, Body::default());
        }
    }

    tracing::debug!(target: "tron_app", "event payload: {:?}", payload);

    let response = if let Some(event_data) = match_event(&payload).await {
        let mut evt = event_data.tn_event;
        evt.h_target.clone_from(&hx_target);
        tracing::debug!(target: "tron_app", "event tn_event: {:?}", evt);

        if evt.e_type == "change" {
            if let Some(value) = event_data.e_value {
                let context_guard = app_data.context.read().await;
                let context = context_guard.get(&session_id).unwrap().clone();
                context
                    .set_value_for_component(&evt.e_trigger, TnComponentValue::String(value))
                    .await;
            }
        }

        let has_event_action = {
            let event_action_guard = app_data.event_actions.read().await;
            event_action_guard.contains_key(&tron_index)
        };

        if has_event_action {
            let context_guard = app_data.context.read().await;
            let context = context_guard.get(&session_id).unwrap().clone();

            let event_action_guard = app_data.event_actions.write().await;
            let (action_exec_method, action_generator) =
                event_action_guard.get(&tron_index).unwrap().clone();

            let action = action_generator(context, evt, payload);
            match action_exec_method {
                TnActionExecutionMethod::Spawn => {
                    tokio::task::spawn(action);
                    None
                }
                TnActionExecutionMethod::Await => action.await,
            }
        } else {
            None
        }
    } else {
        None
    };

    if let Some((mut response_headers, html)) = response {
        response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());
        (StatusCode::OK, response_headers, Body::from(html.0))
    } else {
        // send default rendered element + header processing
        let mut response_headers = HeaderMap::new();
        response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());
        let context_guard = app_data.context.read().await;
        let context = &context_guard.get(&session_id).unwrap().read().await;

        let tron_index = if let Some(hx_target) = hx_target {
            context.get_component_index(&hx_target)
        } else {
            tron_index
        };

        let mut component_guard = context.components.write().await;
        let target_guard = component_guard.get_mut(&tron_index).unwrap();
        let body = Body::new({
            let target = target_guard.read().await;
            tokio::task::block_in_place(|| target.render())
        });

        let mut header_to_be_removed = Vec::<String>::new();

        target_guard
            .write()
            .await
            .extra_headers()
            .iter()
            .for_each(|(k, v)| {
                response_headers.insert(
                    HeaderName::from_bytes(k.as_bytes()).unwrap(),
                    HeaderValue::from_bytes(v.0.as_bytes()).unwrap(),
                );
                if v.1 {
                    header_to_be_removed.push(k.clone());
                };
            });

        // remove the header items that we only want to use it once
        for k in header_to_be_removed {
            target_guard.write().await.remove_header(k);
        }

        (StatusCode::OK, response_headers, body)
    }
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

/// Handles server-sent events (SSE) for the Tron application.
///
/// This asynchronous function handles server-sent events (SSE) for the Tron application.
/// It retrieves the session ID, checks for session existence, and establishes a stream
/// for sending SSE events to the client.
///
/// # Arguments
///
/// * `app_data` - Application data shared across the application.
/// * `session` - Session information associated with the request.
///
/// # Returns
///
/// A result containing the SSE stream if successful, or a status code indicating an error.
///
/// # Errors
///
/// Returns `StatusCode::FORBIDDEN` if the session ID is not available or the session does not exist.
/// Returns `StatusCode::SERVICE_UNAVAILABLE` if the SSE channel is unavailable.
///
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
        let context_guard = app_data.context.read().await;
        if !context_guard.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let stream = {
        let mut session_guard = app_data.context.write().await;
        let context_guard = session_guard.get_mut(&session_id).unwrap().write().await;
        let mut channel_guard = context_guard.sse_channel.write().await;
        if let Some(rx) = channel_guard.as_mut().unwrap().rx.take() {
            ReceiverStream::new(rx).map(|v| Ok(sse::Event::default().data(v.clone())))
        } else {
            return Err(StatusCode::SERVICE_UNAVAILABLE);
        }
    };

    Ok(Sse::new(stream).keep_alive(KeepAlive::default()))
}

/// Handles streaming for Tron application.
///
/// This asynchronous function handles streaming for the Tron application. It retrieves session ID, checks
/// for session existence, and processes stream data associated with the given stream ID. It constructs
/// and returns a response containing the stream data.
///
/// # Arguments
///
/// * `app_data` - Application data shared across the application.
/// * `session` - Session information associated with the request.
/// * `stream_id` - The ID of the stream for which data is being streamed.
///
/// # Returns
///
/// A tuple containing the HTTP status code, response headers, and response body.
///
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
        let context_guard = app_data.context.read().await;
        if !context_guard.contains_key(&session_id) {
            return (StatusCode::FORBIDDEN, default_header, Body::default());
        }
    }

    {
        let session_guard = app_data.context.read().await;
        let context_guard = session_guard.get(&session_id).unwrap().read().await;
        let stream_data_guard = &context_guard.stream_data.read().await;
        if !stream_data_guard.contains_key(&stream_id) {
            return (StatusCode::NOT_FOUND, default_header, Body::default());
        }
    }

    let (protocol, data_queue) = {
        let session_guard = app_data.context.read().await;
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

/// Handles login redirection for authentication.
///
/// This asynchronous function constructs and returns a redirect to the login page of the configured
/// Cognito authentication service. It retrieves necessary configuration parameters from the environment
/// variables and constructs the login URL accordingly.
///
/// # Returns
///
/// A redirect to the login page of the configured Cognito authentication service.
///
/// # Panics
///
/// Panics if the required environment variables (`CLIENT_ID`, `COGNITO_DOMAIN`, `COGNITO_RESPONSE_TYPE`,
/// `REDIRECT_URI`) are not set.
async fn login_handler() -> Redirect {
    let cognito_client_id = env::var("CLIENT_ID").expect("CLIENT_ID env not set");
    let cognito_domain = env::var("COGNITO_DOMAIN").expect("COGNITO_DOMAIN not set");
    let cognito_response_type =
        env::var("COGNITO_RESPONSE_TYPE").expect("COGNITO_RESPONSE_TYPE not set");
    let redirect_uri = env::var("REDIRECT_URI").expect("REDIRECT_URI not set");

    Redirect::to(&format!("https://{cognito_domain}/login?client_id={cognito_client_id}&response_type={cognito_response_type}&redirect_uri={redirect_uri}"))
}


/// Handles logout redirection for authentication.
///
/// This asynchronous function constructs and returns a redirect to the logout page of the configured
/// Cognito authentication service. It retrieves necessary configuration parameters from the environment
/// variables and constructs the logout URL accordingly.
///
/// # Returns
///
/// A redirect to the logout page of the configured Cognito authentication service.
///
/// # Panics
///
/// Panics if the required environment variables (`CLIENT_ID`, `COGNITO_DOMAIN`, `LOGOUT_REDIRECT_URI`)
/// are not set.
async fn logout_handler() -> Redirect {
    let cognito_client_id = env::var("CLIENT_ID").expect("CLIENT_ID env not set");
    let cognito_domain = env::var("COGNITO_DOMAIN").expect("COGNITO_DOMAIN not set");
    let logout_uri = env::var("LOGOUT_REDIRECT_URI").expect("LOGOUT_REDIRECT_URI not set");

    Redirect::to(&format!(
        "https://{cognito_domain}/logout?client_id={cognito_client_id}&logout_uri={logout_uri}"
    ))
}

/// Handles the logged-out state of the application.
///
/// This asynchronous function retrieves the HTML content for the logged-out page from the
/// application data. It removes the session data associated with the user from the application
/// data and removes the "token" value from the session. It returns HTML content for the logged-out
/// page.
///
/// # Arguments
///
/// * `app_data` - Application data shared across the application.
/// * `session` - Session information associated with the request.
///
/// # Returns
///
/// HTML content for the logged-out page.
async fn logged_out(State(app_data): State<Arc<AppData>>, session: Session) -> impl IntoResponse {
    let logout_html = {
        let session_id = session.id().unwrap();
        let context_guard = app_data.context.write().await;
        let context = context_guard.get(&session_id).unwrap();
        let base = context.base.read().await;
        let logout_html = base.asset.read().await;
        if let Some(TnAsset::String(logout_html)) = logout_html.get("logout_page") {
            logout_html.clone()
        } else {
            r#"logged out"#.into()
        }
    };
    // remove data from the session in app_data
    {
        let session_id = session.id().unwrap();
        let mut context_guard = app_data.context.write().await;

        context_guard.remove(&session_id);
    }
    let _ = session.remove_value("token").await;
    Html::from(logout_html)
}

/// Represents a deserialized JSON Web Token (JWT) containing authentication information.
///
/// This struct is used to deserialize a JWT token obtained from the authentication service.
/// It contains fields representing different parts of the JWT token, such as access token,
/// ID token, refresh token, token type, and expiration duration.
///
/// This struct is marked with `#[allow(dead_code)]` attribute to suppress warnings about
/// unused fields during development or if some fields are not used in the application logic.
///
/// # Fields
///
/// * `access_token` - The access token obtained from the authentication service.
/// * `expires_in` - The duration in seconds for which the token is valid before expiration.
/// * `id_token` - The ID token obtained from the authentication service.
/// * `refresh_token` - The refresh token obtained from the authentication service.
/// * `token_type` - The type of the token (e.g., "Bearer").
#[allow(dead_code)]
#[derive(Deserialize, Debug)]
struct JWTToken {
    access_token: String,
    expires_in: usize,
    id_token: String,
    refresh_token: String,
    token_type: String,
}

/// Represents the claims extracted from a JSON Web Token (JWT) payload.
///
/// This struct is used to deserialize the claims section of a JWT token, which typically
/// contains information about the token's expiration time, issuer, subject, and other custom
/// claims. The fields in this struct correspond to the standard JWT claims and any additional
/// custom claims included in the token.
///
/// This struct is marked with `#[allow(dead_code)]` attribute to suppress warnings about
/// unused fields during development or if some fields are not used in the application logic.
///
/// # Fields
///
/// * `exp` - Required field indicating the expiration time of the token as a UTC timestamp.
/// * `iat` - Optional field indicating the time at which the token was issued as a UTC timestamp.
/// * `iss` - Optional field indicating the issuer of the token.
/// * `sub` - Optional field indicating the subject whom the token refers to.
/// * `email` - The email address associated with the token.
/// * `username` - The username associated with the token, with serde renaming to "cognito:username".
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
struct Claims {
    exp: usize, // Required (validate_exp defaults to true in validation). Expiration time (as UTC timestamp)
    iat: usize, // Optional. Issued at (as UTC timestamp)
    iss: String, // Optional. Issuer
    sub: String, // Optional. Subject (whom token refers to)
    email: String,
    #[serde(rename(deserialize = "cognito:username"))]
    username: String,
}

/// Handles the callback from the Cognito authentication service after a user logs in.
///
/// # Arguments
///
/// * `session`: Session object representing the user session.
/// * `headers`: HeaderMap containing the HTTP headers received in the request.
/// * `query`: HashMap<String, String> containing the query parameters received in the request.
///
/// # Returns
///
/// An HTML response containing a JavaScript redirect script to the desired page.
///
/// # Panics
///
/// Panics if required environment variables are not set or if there are errors in the HTTP requests.
///
/// # Notes
///
/// This function retrieves the necessary environment variables such as the Cognito client ID,
/// domain, user pool ID, and AWS region. It then constructs a POST request to the Cognito token
/// endpoint with the authorization code received from the callback. The token response is parsed
/// and decoded using the JSON Web Key Set (JWKS) obtained from the Cognito service. The decoded
/// token is then validated using the RSA public key obtained from the JWKS. If the session ID
/// is not present, it is inserted along with the token into the session. Finally, the function
/// responds with a simple HTML script to redirect the user to the desired page.
async fn cognito_callback(
    session: Session,
    headers: HeaderMap,
    Query(query): Query<HashMap<String, String>>,
) -> impl IntoResponse {
    tracing::info!(target = "tron_app", "query: {:?}", query);
    tracing::info!(target = "tron_app", "cognito_header: {:?}", headers);
    let cognito_client_id = env::var("CLIENT_ID").expect("CLIENT_ID env not set");
    let redirect_uri = env::var("REDIRECT_URI").expect("REDIRECT_URI not set");
    let cognito_domain = env::var("COGNITO_DOMAIN").expect("COGNITO_DOMAIN not set");
    let cognito_user_pool_id =
        env::var("COGNITO_USER_POOL_ID").expect("COGNITO_USER_POOL_ID not set");
    let cognito_aws_region = env::var("COGNITO_AWS_REGION").expect("COGNITO_AWS_REGION not set");
    let client = reqwest::Client::new();
    let data = [
        ("grant_type", "authorization_code"),
        ("client_id", cognito_client_id.as_str()),
        ("code", query.get("code").unwrap().as_str()),
        ("redirect_uri", redirect_uri.as_str()),
    ];

    let token_endpoint = format!("https://{cognito_domain}/oauth2/token");
    let res = client
        .post(&token_endpoint)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&data)
        .send()
        .await
        .expect("cognito call fail");

    let raw_text = res.text().await.unwrap();
    tracing::debug!(target = "tron_app", "cognito res raw: {}", raw_text);
    let jwt_value: JWTToken = serde_json::from_str(&raw_text).unwrap();
    tracing::debug!(target = "tron_app", "cognito res: {:?}", jwt_value);
    let header = jsonwebtoken::decode_header(&jwt_value.id_token).unwrap();
    tracing::debug!(target = "tron_app", "id_token header: {:?}", header);

    let jwks_url = format!("https://cognito-idp.{cognito_aws_region}.amazonaws.com/{cognito_user_pool_id}/.well-known/jwks.json");
    let res = client
        .get(jwks_url)
        .send()
        .await
        .expect("cognito call fail");
    let value: Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
    tracing::debug!(
        target = "tron_app",
        "cognito jwks res: {:?}",
        value["keys"].as_array().unwrap()
    );
    let header_kid = header.kid.unwrap();
    let header_kid = header_kid.as_str();
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.validate_aud = false; // there is no audience available in cognito's JWT
    value["keys"].as_array().unwrap().iter().for_each(|obj| {
        if obj["kid"].as_str().unwrap() == header_kid {
            let token = jsonwebtoken::decode::<Claims>(
                &jwt_value.id_token,
                &jsonwebtoken::DecodingKey::from_rsa_components(
                    obj["n"].as_str().unwrap(),
                    obj["e"].as_str().unwrap(),
                )
                .unwrap(),
                &validation,
            )
            .unwrap();
            tracing::info!(target = "tron_app", "cognito decode token: {:?}", token);
        }
    });

    if session.id().is_none() {
        session.insert("session_set", true).await.unwrap();
    };
    let _ = session.insert("token", jwt_value.id_token).await;
    tracing::debug!(target:"tron_app", "in congito_callback session_id: {:?}", session.id());
    tracing::debug!(target:"tron_app", "in congito_callback session: {:?}", session);
    tracing::debug!(target:"tron_app", "in congito_callback token: {:?}", session.get_value("token").await.unwrap());

    // we can't use the axum redirect response as it won't set the session cookie
    Html::from(r#"<script> window.location.replace("/"); </script>"#)
}

/// Middleware for checking the presence of a JWT token in the user's session.
///
/// This middleware intercepts incoming requests and checks if they require authentication.
/// If the request URI is `/login`, `/cognito_callback`, or `/logged_out/`, the middleware
/// allows the request to proceed without checking for a token.
///
/// If the request URI requires authentication and a JWT token is present in the user's session,
/// the middleware allows the request to proceed.
///
/// If the request URI requires authentication but no JWT token is present in the user's session,
/// the middleware redirects the user to the login page.
///
/// # Arguments
///
/// * `uri`: OriginalUri representing the requested URI.
/// * `session`: Session object representing the user session.
/// * `request`: Request object representing the HTTP request.
/// * `next`: Next object representing the next middleware or handler in the chain.
///
/// # Returns
///
/// An implementation of `IntoResponse`.
///
/// # Note
///
/// This middleware assumes that the presence of a JWT token in the user's session indicates
/// that the user is authenticated. It redirects users to the login page if they attempt to
/// access protected resources without a valid token.
async fn check_token(
    uri: OriginalUri,
    session: Session,
    request: Request,
    next: Next,
) -> impl IntoResponse {
    // do something with `request`...
    tracing::debug!(target:"tron_app", "in log_session session_id: {:?}", session.id());
    tracing::debug!(target:"tron_app", "session: {:?}", session);
    tracing::debug!(target:"tron_app", "path: {:?}", uri.path());
    tracing::debug!(target:"tron_app", "token: {:?}", session.get_value("token").await.unwrap());
    if uri.path() == "/login" || uri.path() == "/cognito_callback" || uri.path() == "/logged_out/" {
        let response = next.run(request).await;
        response.into_response()
    } else if let Some(token) = session.get_value("token").await.unwrap() {
        tracing::debug!(target:"tron_app", "has jwt token {:?}", token);
        let response = next.run(request).await;
        response.into_response()
    } else {
        tracing::debug!(target:"tron_app", "has NO jwt token, session: {:?}", session);
        Redirect::permanent("/login").into_response()
    }
}
