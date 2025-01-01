use askama::Template;
use axum::{
    body::Body, extract::{DefaultBodyLimit, Json, Multipart, OriginalUri, Path, Query, Request, State}, handler::HandlerWithoutStateExt, http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode, Uri}, middleware::{self, Next}, response::{
        sse::{self, KeepAlive},
        Html, IntoResponse, Redirect, Sse,
    }, routing::{get, post}, BoxError, Router
};
use axum_extra::extract::Host;
use axum_server::tls_rustls::RustlsConfig;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tokio::sync::RwLock;
use tron_components::{
    TnActionExecutionMethod, TnAsset, TnComponent, TnComponentId, TnComponentValue, TnContext,
    TnEvent, TnSseMsgChannel,
};
//use std::sync::Mutex;
use std::{
    collections::HashMap, convert::Infallible, env, net::SocketAddr, path::PathBuf, sync::Arc,
};
use time::{Duration, OffsetDateTime};
use tower_http::{
    cors::CorsLayer,
    services::{ServeDir, ServeFile},
    trace::TraceLayer,
};
use tower_sessions::{Expiry, MemoryStore, Session, SessionManagerLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

use tokio_stream::{wrappers::ReceiverStream, Stream, StreamExt};

pub static TRON_APP: &str = "tron_app";

/// Represents HTTP and HTTPS ports.
///
/// This struct encapsulates the HTTP and HTTPS ports used in a network configuration.
#[derive(Clone, Copy)]
pub struct Ports {
    pub http: u16,
    pub https: u16,
}

/// Represents a session ID used in Tower Sessions.
pub type SessionId = tower_sessions::session::Id;

/// Represents a session context containing mappings of session IDs to Tron contexts.
pub type SessionContextStore = RwLock<HashMap<SessionId, TnContext>>;

/// Alias for a context builder function.
type ContextBuilder = Arc<Box<dyn Fn() -> TnContext + Send + Sync>>;

/// Alias for a layout function, which generates HTML layout from a context.
/// type LayoutFunction = Arc<Box<dyn Fn(TnContext) -> String + Send + Sync>>;
use std::pin::Pin;

type LayoutFunction = Arc<
    Box<
        dyn Fn(TnContext) -> Pin<Box<dyn futures_util::Future<Output = String> + Send>>
            + Send
            + Sync,
    >,
>;

type SessionExpiry = RwLock<HashMap<SessionId, time::OffsetDateTime>>;

/// Represents application data used in a Tron application.
///
/// This struct encapsulates various data components used in a Tron application,
/// including session context, session expiry mappings, event actions, context builder function,
/// action function template, and layout function.
pub struct AppData {
    pub head: Option<String>, // for inject head section to the html index page
    pub html_attributes: Option<String>,
    pub context_store: SessionContextStore,
    pub session_expiry: SessionExpiry,
    pub build_context: ContextBuilder,
    pub build_layout: LayoutFunction,
}

pub struct AppDataBuilder {
    pub head: Option<String>, // for inject head section to the html index page
    pub html_attributes: Option<String>,
    pub context_store: SessionContextStore,
    pub session_expiry: RwLock<HashMap<SessionId, time::OffsetDateTime>>,
    pub build_context: fn() -> TnContext,
    pub build_layout: fn(TnContext) -> Pin<Box<dyn futures_util::Future<Output = String> + Send>>,
}

impl AppData {
    pub fn builder(
        build_context: fn() -> TnContext,
        build_layout: fn(TnContext) -> Pin<Box<dyn futures_util::Future<Output = String> + Send>>,
    ) -> AppDataBuilder {
        AppDataBuilder {
            head: Some(include_str!("../templates/head.html").to_string()),
            html_attributes: Some(r#"leng="en""#.into()),
            context_store: RwLock::new(HashMap::default()),
            session_expiry: RwLock::new(HashMap::default()),
            build_context,
            build_layout,
        }
    }
}

impl AppDataBuilder {
    pub fn set_head(mut self, head: &str) -> Self {
        self.head = Some(head.into());
        self
    }
    pub fn set_html_attributes(mut self, html_attributes: &str) -> Self {
        self.html_attributes = Some(html_attributes.into());
        self
    }
    pub fn set_context(mut self, context: SessionContextStore) -> Self {
        self.context_store = context;
        self
    }
    pub fn session_expiry(mut self, session_expiry: SessionExpiry) -> Self {
        self.session_expiry = session_expiry;
        self
    }
    pub fn build(self) -> AppData {
        AppData {
            head: self.head,
            html_attributes: self.html_attributes,
            context_store: self.context_store,
            session_expiry: self.session_expiry,
            build_context: Arc::new(Box::new(self.build_context)),
            build_layout: Arc::new(Box::new(self.build_layout)),
        }
    }
}

/// Represents configuration options for a Tron application.
///
/// This struct encapsulates various configuration options for a Tron application,
/// including the network address, ports, Cognito login flag, log level, and session expiry duration.
///
/// # Fields
///
/// * `address` - A static array of 4 bytes representing the network address.
/// * `ports` - A `Ports` struct containing the HTTP and HTTPS ports.
/// * `cognito_login` - A boolean flag indicating whether Cognito login is enabled.
/// * `log_level` - An optional static string slice representing the log level.
/// * `session_expiry` - An optional `Duration` representing the session expiry duration.
pub struct AppConfigure {
    pub address: [u8; 4],
    pub ports: Ports,
    pub cognito_login: bool,
    pub http_only: bool,
    pub log_level: Option<&'static str>,
    pub session_expiry: Option<time::Duration>,
    pub api_router: Option<Router<Arc<AppData>>>,
    pub static_html_path: &'static str,
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
    /// - TODO: update doc for api_router
    fn default() -> Self {
        let address = [127, 0, 0, 1];
        let ports = Ports {
            http: 8080,
            https: 3001,
        };
        let cognito_login = false;
        let log_level = Some("server=info,tower_http=info,tron_app=info");
        let http_only = false;
        let static_html_path = "static";
        Self {
            address,
            ports,
            http_only,
            cognito_login,
            log_level,
            session_expiry: None,
            api_router: None,
            static_html_path,
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
/// ```
pub async fn run(app_share_data: AppData, config: AppConfigure) {
    let log_level = config.log_level;
    let log_level = log_level.unwrap_or("server=info,tower_http=info,tron_app=info");

    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                //.unwrap_or_else(|_| "server=debug,tower_http=debug".into()),
                .unwrap_or_else(|_| log_level.into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    let session_store = MemoryStore::default();

    let expiry_duration = if let Some(expiry_duration) = config.session_expiry {
        expiry_duration
    } else {
        Duration::minutes(20)
    };

    let session_layer = SessionManagerLayer::new(session_store.clone())
        .with_same_site(tower_sessions::cookie::SameSite::Lax)
        .with_secure(false)
        .with_expiry(Expiry::OnInactivity(expiry_duration));

    let serve_dir = ServeDir::new(config.static_html_path)
        .not_found_service(ServeFile::new("static/index.html"));

    let ports = config.ports;

    // build our application with a route
    let app_share_data = Arc::new(app_share_data);

    let routes = Router::new()
        .route("/", get(index))
        .route("/server_events", get(sse_event_handler))
        .route("/load_page", get(load_page))
        .route("/tron/{tron_id}", get(tron_entry).post(tron_entry))
        .route(
            "/tron_streaming/{stream_id}",
            get(tron_stream).post(tron_stream),
        )
        .route("/upload/{tron_id}", post(upload))
        .layer(DefaultBodyLimit::max(64 * 1024 * 1024));

    let routes = if let Some(api_router) = config.api_router {
        //let api_routes = Router::new().nest_service("/api", api_router);
        Router::new().merge(routes).nest("/api", api_router)
    } else {
        routes
    };

    let auth_routes = Router::new()
        .route("/login", get(login_handler))
        .route("/logout", get(logout_handler))
        .route("/logged_out", get(logged_out))
        .route("/cognito_callback", get(cognito_callback));

    let app_routes = if config.cognito_login {
        Router::new().merge(routes).merge(auth_routes)
    } else {
        routes
    };

    let app_routes = app_routes
        .nest_service("/static", serve_dir.clone())
        .fallback_service(serve_dir)
        .layer(TraceLayer::new_for_http())
        .layer(CorsLayer::very_permissive());

    let app_routes = if config.cognito_login {
        app_routes.layer(middleware::from_fn(check_token))
    } else {
        app_routes
    };

    let app_routes = app_routes
        .layer(session_layer)
        .with_state(app_share_data.clone());

    tokio::task::spawn(clean_up_session(app_share_data.clone()));

  

    if config.http_only {
        let addr = SocketAddr::from((config.address, ports.http));
        tracing::info!(target:"tron_app", "Starting server at {}", addr);
        
        axum_server::bind(addr)
            .serve(app_routes.into_make_service())
            .await
            .unwrap();
    } else {
        let addr = SocketAddr::from((config.address, ports.https));
        // optional: spawn a second server to redirect http requests to this server
        tokio::spawn(redirect_http_to_https(config.address, ports));

        // per https://github.com/abdolence/slack-morphism-rust/issues/286 we need the follow line to get it to work
        // see also https://github.com/snapview/tokio-tungstenite/issues/336
        let _ = rustls::crypto::ring::default_provider().install_default();

        // configure certificate and private key used by https
        let tls_config = RustlsConfig::from_pem_file(
            PathBuf::from(".")
                .join("self_signed_certs")
                .join("cert.pem"),
            PathBuf::from(".").join("self_signed_certs").join("key.pem"),
        )
        .await
        .unwrap();

        tracing::info!(target:"tron_app", "Starting server at {}", addr);
        axum_server::bind_rustls(addr, tls_config)
            .serve(app_routes.into_make_service())
            .await
            .unwrap();
    }
}

#[derive(Template)] // this will generate the code...
#[template(path = "index.html", escape = "none")]
struct IndexPageTemplate {
    head: String,
    script: String,
    html_attributes: String,
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
    if session.is_empty().await {
        // the line below is necessary to make sure the session is set
        session
            .insert("session is not set, redirect to set the session", true)
            .await
            .unwrap();
        tracing::info!(target:"tron_app", "set session");
        Redirect::to("/").into_response()
    } else {
        tracing::info!(target:"tron_app", "setting session");
        let mut session_expiry = app_data.session_expiry.write().await;
        session_expiry.insert(session.id().unwrap(), session.expiry_date());

        // We need to insert the "userdata" into the session or when we call session.get("userdata"),
        // the session is reset, it maybe a bug in the library
        session.insert("userdata", "").await.unwrap();

        let script = {
            let new_context = tokio::task::block_in_place(|| (*app_data.build_context)());
            let guard = new_context.read().await;
            let script = tokio::task::block_in_place(move || {
                guard.scripts.values().cloned().collect::<Vec<String>>()
            })
            .join("\n");
            script
        };
        let head = if let Some(head) = &app_data.head {
            head.clone()
        } else {
            "".into()
        };
        let html_attributes = if let Some(html_attributes) = &app_data.html_attributes {
            html_attributes.clone()
        } else {
            "".into()
        };
        let html = IndexPageTemplate {
            head,
            script,
            html_attributes,
        };
        let html = html.render().unwrap();

        Html::from(html).into_response()
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
        let mut session_contexts = app_data.context_store.write().await;
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

    let context_store_guard = app_data.context_store.read().await;
    {
        let context = context_store_guard.get(&session_id).unwrap().clone();
        let mut context_guard = context.write().await;
        if context_guard.user_data.write().await.is_none() {
            let user_data = session
                .get::<String>("user_data")
                .await
                .expect("error on getting user data");
            context_guard.user_data = Arc::new(RwLock::new(user_data));
        }
    }
    
    let context = context_store_guard.get(&session_id).unwrap().clone();
    //let layout = tokio::task::block_in_place(|| (*app_data.build_layout)(context.clone()));
    let layout = (*app_data.build_layout)(context).await;
    Ok(Html::from(layout))
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
    Path(tron_index): Path<TnComponentId>,
    Json(payload): Json<Value>,
    //request: Request,
) -> impl IntoResponse {
    tracing::debug!(target: "tron_app", "headers: {:?}", headers);
    tracing::debug!(target: "tron_app", "event payload: {:?}", payload);

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

    let context_store_guard = app_data.context_store.read().await;
    if !context_store_guard.contains_key(&session_id) {
        //return Err(StatusCode::FORBIDDEN);
        return (StatusCode::FORBIDDEN, response_headers, Body::default());
    }

    let context = context_store_guard.get(&session_id).unwrap().clone();

    let response = if let Some(event_data) = match_event(&payload).await {
        let mut evt = event_data.tn_event;
        evt.h_target.clone_from(&hx_target);
        tracing::debug!(target: "tron_app", "event tn_event: {:?}", evt);

        if evt.e_type == "change" {
            if let Some(value) = event_data.e_value {
                context
                    .set_value_for_component(&evt.e_trigger, TnComponentValue::String(value))
                    .await;
            }
        }
        {
            let component_guard = context.get_component_by_id(&tron_index).await;

            let has_event_action = {
                let component = component_guard.read().await;
                component.get_action().is_some()
            };
            // we need to acquire a new lock for component, or it will have a deadlock
            if has_event_action {
                let (action_exec_method, action_generator) = {
                    let component = component_guard.read().await;
                    component.get_action().as_ref().unwrap().clone()
                };

                let action = action_generator(context.clone(), evt, payload);
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

        // when there a hx_target, we update the component indicated by hx_target than the triggering component
        let tron_index = if let Some(hx_target) = hx_target {
            hx_target
        } else {
            tron_index
        };

        let body = {
            let context_guard = context.read().await;
            let mut component_guard = context_guard.components.write().await;
            let target_guard = component_guard.get_mut(&tron_index).unwrap();
            let mut target = target_guard.write().await;

            target.pre_render(&context_guard).await;

            let body = Body::new(target.render().await);

            target.post_render(&context_guard).await;

            let mut header_to_be_removed = Vec::<String>::new();

            target.extra_headers().iter().for_each(|(k, v)| {
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
                target.remove_header(&k);
            }
            body
        };

        (StatusCode::OK, response_headers, body)
    }
}

/// Retrieves a specific component associated with a session from application data.
///
/// This asynchronous function accesses shared application data to retrieve a component
/// based on the session ID and a component index. The function assumes that the necessary
/// data structures are properly synchronized and accessible concurrently.
///
/// # Parameters
/// - `app_data: Arc<AppData>`:
///   A thread-safe reference-counted pointer to the shared application data.
///
/// - `session_id: tower_sessions::session::Id`:
///   The unique identifier for the session, used to retrieve session-specific context.
///
/// - `tron_idx: TnComponentIndex`:
///   An index that identifies the specific component within the session's context.
///
/// # Returns
/// `TnComponent<'static>`:
/// A clone of the component found at the specified index within the session's context.
/// The lifetime `'static` indicates that the component does not hold references to data
/// governed by lifetimes shorter than the program's execution.
///
/// # Behavior
/// 1. Acquires a read lock on the application's context to access the session data.
/// 2. Retrieves the context for the specific session identified by `session_id`.
/// 3. Acquires another read lock on the base context of the session to access its components.
/// 4. Retrieves and clones the component at the specified index `tron_idx`.
///
/// # Panics
/// The function unwraps the Option values directly, and will panic if:
/// - The session with the provided `session_id` does not exist.
/// - The component with the provided `tron_idx` does not exist.
async fn get_session_component_from_app_data(
    app_data: Arc<AppData>,
    session_id: tower_sessions::session::Id,
    tron_idx: TnComponentId,
) -> TnComponent<'static> {
    let guard = app_data.context_store.read().await;
    let session_context = guard.get(&session_id).unwrap();
    let context_guard = session_context.base.read().await;

    let components = context_guard.components.read().await;
    components.get(&tron_idx).unwrap().clone()
}

/// Handles the uploading of files for a specific session and component, saving the uploaded data.
///
/// This function processes multipart form data corresponding to a file upload within a session.
/// It ensures that the data belongs to the correct component by validating against a component-specific
/// identifier. If the session is valid and the data matches the component, the files are stored in the
/// application's asset storage.
///
/// # Parameters
/// - `State(app_data): State<Arc<AppData>>`:
///   Shared application data wrapped in an `Arc` for thread safety and wrapped in a `State` for
///   access within the web framework.
///
/// - `session: Session`:
///   The session from which the upload is initiated. Used to validate session existence and retrieve session-specific data.
///
/// - `Path(tron_index): Path<TnComponentIndex>`:
///   The index of the component associated with the upload, encapsulated in a `Path` for extraction from the request path.
///
/// - `mut multipart: Multipart`:
///   A stream of the multipart form data from the request, allowing for asynchronous processing of each field.
///
/// # Returns
/// `StatusCode`:
/// - Returns `StatusCode::OK` if the upload is successful and the data is stored.
/// - Returns `StatusCode::FORBIDDEN` if the session ID is not found or invalid.
///
/// # Behavior
/// 1. Validates the session's existence.
/// 2. Retrieves the specific component from the session data to validate the upload is pertinent to the correct component.
/// 3. Processes each field in the multipart form data, filtering out fields that do not match the component-specific identifier.
/// 4. Saves the uploaded file data into the application's asset storage under the "upload" key.
/// 5. Logs the size of each uploaded file.
///
/// # Panics
/// The function will panic if essential fields (like `ContentType`) cannot be parsed, or if the multipart data stream
/// contains unexpected null values.
///
async fn upload(
    State(app_data): State<Arc<AppData>>,
    session: Session,
    Path(tron_index): Path<TnComponentId>,
    mut multipart: Multipart,
) -> StatusCode {
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return StatusCode::FORBIDDEN;
    };

    let tnid = {
        let component =
            get_session_component_from_app_data(app_data.clone(), session_id, tron_index).await;
        let guard = component.read().await;
        [guard.tron_id().clone(), "_form".into()].join("")
    };

    let mut field_data = HashMap::<String, Vec<u8>>::new();
    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field.name().unwrap().to_string();
        if name != tnid {
            continue;
        };
        let file_name = field.file_name().unwrap().to_string();
        let data = field.bytes().await.unwrap();
        tracing::info!(target: TRON_APP, "Uploaded, Length of `{}` is {} bytes", file_name, data.len());
        let e = field_data.entry(file_name).or_default();
        e.extend_from_slice(&data);
    }
    {
        let guard = app_data.context_store.read().await;
        let context = guard.get(&session_id).unwrap();
        let asset_ref = context.get_asset_ref().await;
        let mut asset_guard = asset_ref.write().await;
        let e = asset_guard
            .entry("upload".into())
            .or_insert(TnAsset::HashMapVecU8(HashMap::default()));
        for (file_name, data) in field_data {
            if let TnAsset::HashMapVecU8(ref mut e) = e {
                e.insert(file_name, data);
            };
        }
    }
    StatusCode::OK
}

/// Redirects HTTP requests to HTTPS for a specified address and port configuration.
///
/// This function sets up an HTTP server that listens on a specified IP address and HTTP port. All incoming HTTP requests are
/// permanently redirected to HTTPS using the same host and the URI but on a different port as specified by the `ports` parameter.
///
/// # Attributes
/// - `#[allow(dead_code)]`: This attribute allows the function to be compiled and included even if it is not used anywhere in the code.
///
/// # Parameters
/// - `addr: [u8; 4]`: The IP address on which the server listens, specified as an array of four u8 integers representing an IPv4 address.
/// - `ports: Ports`: A struct that contains both the HTTP and HTTPS port numbers.
///
/// # Inner Function
/// - `make_https(host: String, uri: Uri, ports: Ports) -> Result<Uri, BoxError>`:
///   Attempts to convert an HTTP URI to HTTPS by modifying its scheme and port. Returns a `Result` with either the modified `Uri` or an error if conversion fails.
///
/// # Behavior
/// 1. Binds a TCP listener to the provided IPv4 address and the HTTP port.
/// 2. Uses Axum to serve incoming requests, converting each request's URI from HTTP to HTTPS.
/// 3. Logs a warning if the URI conversion fails and returns a `BAD_REQUEST` status code.
/// 4. Logs the address it is listening on upon successful setup.
///
/// # Panics
/// This function will panic if:
/// - It fails to bind the TCP listener to the specified address and port.
/// - There is a failure in parsing the URI during the redirection process.
#[allow(dead_code)]
async fn redirect_http_to_https(addr: [u8; 4], ports: Ports) {
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

    let addr = SocketAddr::from((addr, ports.http));
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
        let context_guard = app_data.context_store.read().await;
        if !context_guard.contains_key(&session_id) {
            return Err(StatusCode::FORBIDDEN);
        }
    }

    let stream = {
        let mut session_guard = app_data.context_store.write().await;
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
        let context_guard = app_data.context_store.read().await;
        if !context_guard.contains_key(&session_id) {
            return (StatusCode::FORBIDDEN, default_header, Body::default());
        }
    }

    {
        let session_guard = app_data.context_store.read().await;
        let context_guard = session_guard.get(&session_id).unwrap().read().await;
        let stream_data_guard = &context_guard.stream_data.read().await;
        if !stream_data_guard.contains_key(&stream_id) {
            return (StatusCode::NOT_FOUND, default_header, Body::default());
        }
    }

    let (protocol, data_queue) = {
        let session_guard = app_data.context_store.read().await;
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
        let context_store_guard = app_data.context_store.write().await;
        let context = context_store_guard.get(&session_id).unwrap();
        let base = context.base.read().await;
        let logout_html = base.assets.read().await;
        if let Some(TnAsset::String(logout_html)) = logout_html.get("logout_page") {
            logout_html.clone()
        } else {
            r#"<html><body>logged out, <a href="/login">click here</a> to login again</body></html>"#
               .into()
        }
    };
    // remove data from the session in app_data
    {
        let session_id = session.id().unwrap();
        let mut context_store_guard = app_data.context_store.write().await;

        context_store_guard.remove(&session_id);
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
#[derive(Deserialize, Serialize, Debug)]
pub struct JWTToken {
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

fn extract_user_info(
    token_value: &Value,
    header_kid: &str,
    jwt_value: &JWTToken,
) -> Result<(String, String), Box<dyn std::error::Error>> {
    let keys = token_value["keys"]
        .as_array()
        .ok_or("Invalid token structure: 'keys' is not an array")?;
    let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    validation.validate_aud = false; // there is no audience available in cognito's JWT
    for obj in keys {
        if obj["kid"].as_str() == Some(header_kid) {
            let _n = obj["n"].as_str().ok_or("Missing 'n' in key")?;
            let _e = obj["e"].as_str().ok_or("Missing 'e' in key")?;

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

            return Ok((token.claims.username, token.claims.email));
        }
    }

    Err("No matching key found".into())
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
    let token_value: Value = serde_json::from_str(&res.text().await.unwrap()).unwrap();
    tracing::debug!(
        target = "tron_app",
        "cognito jwks res: {:?}",
        token_value["keys"].as_array().unwrap()
    );
    let header_kid = header.kid.unwrap();
    let header_kid = header_kid.as_str();
    // let mut validation = jsonwebtoken::Validation::new(jsonwebtoken::Algorithm::RS256);
    // validation.validate_aud = false; // there is no audience available in cognito's JWT

    if session.id().is_none() {
        session.insert("session_set", true).await.unwrap();
    };
    let _ = session.insert("token", jwt_value.id_token.clone()).await;
    // TODO: use a proper struct to pass the user_data
    let (username, email) = extract_user_info(&token_value, header_kid, &jwt_value)
        .expect("error on get user name and email from JWT");
    let user_data = format!(r#"{{ "username":"{}", "email":"{}" }}"#, username, email);
    let _ = session.insert("user_data", user_data).await;
    tracing::debug!(target:"tron_app", "in congito_callback session_id: {:?}", session.id());
    tracing::debug!(target:"tron_app", "in congito_callback session: {:?}", session);
    tracing::info!(target:"tron_app", "in congito_callback token: {:?}", session.get_value("token").await.unwrap());

    // we can't use the axum redirect response as it won't set the session cookie
    // Html::from(r#"<script> window.location.replace("/"); </script>"#)
    // Using the .with_same_site(tower_sessions::cookie::SameSite::Lax)
    // The re-direction work for 127.0.0.1 but not sure about behind AWS ALB
    Redirect::to("/")
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
        Redirect::to("/login").into_response()
    }
}

/// Periodically cleans up expired sessions from the application data.
///
/// This function runs in an infinite loop and performs the following tasks:
///
/// 1. Reads the `session_expiry` map from the application data.
/// 2. Determines the current time.
/// 3. Iterates over the `session_expiry` map and collects the session IDs of expired sessions.
/// 4. Acquires a write lock on the `context` map in the application data.
/// 5. Removes the session data for the expired sessions from the `context` map.
/// 6. Sleeps for 20 minutes before repeating the process.
///
/// This function is designed to run as a separate task or thread to periodically clean up
/// expired sessions and free up memory used by stale session data.
///
/// # Arguments
///
/// * `app_data` - An `Arc` (atomic reference-counted) pointer to the `AppData` struct containing
///   the application data, including the `session_expiry` and `context` maps.
///
/// # Notes
///
/// - This function runs in an infinite loop and never returns.
/// - It acquires read and write locks on the `session_expiry` and `context` maps, respectively,
///   to ensure thread safety when accessing and modifying the shared data.
/// - The sleep duration of 2.5 minutes can be adjusted as needed to control the frequency of
///   session cleanup.
async fn clean_up_session(app_data: Arc<AppData>) {
    use memory_stats::memory_stats;

    loop {
        let to_remove = {
            let guard = app_data.session_expiry.read().await;
            let now = OffsetDateTime::now_utc();
            tracing::info!(target: "tron_app", "in clean up expiry sessions {}", now);
            let mut to_remove = Vec::new();
            let context_guard = app_data.context_store.read().await;
            for (key, value) in guard.iter() {
                tracing::debug!(target: "tron_app", "in clean up expiry session: {} {} {}", key, now, value );
                if *value < now && context_guard.contains_key(key) {
                    to_remove.push(*key);
                }
            }
            to_remove
        };
        {
            if let Some(usage) = memory_stats() {
                tracing::info!(target:"tron_app", "Current physical memory usage: {}", usage.physical_mem);
            } else {
                tracing::info!(target:"tron_app","Couldn't get the current memory usage :(");
            }

            let mut context_guard = app_data.context_store.write().await;
            for key in to_remove {
                tracing::info!(target: "tron_app", "session removed: {} ", key );
                let mut session_context = context_guard.remove(&key);
                if let Some(context) = session_context.as_mut() {
                    tracing::info!(target: "tron_app", "ctx rc count:{}", Arc::strong_count(&context.base) );
                    context.abort_all_services().await;
                    tokio::time::sleep(std::time::Duration::from_secs(5)).await;
                    tracing::info!(target: "tron_app", "after clean up: ctx rc count:{}", Arc::strong_count(&context.base) );
                };
                if let Some(context) = session_context.clone() {
                    let mut guard = context.base.write().await;
                    guard.components.write().await.clear();
                    guard.stream_data.write().await.clear();
                    guard.services.clear();
                    guard.service_handles.clear();
                    guard.assets.write().await.clear();
                }
                if let Some(context) = session_context {
                    assert!(Arc::strong_count(&context.base) == 1);
                    drop(context);
                    tracing::info!(target: "tron_app", "drop session context" );
                }
            }
            if let Some(usage) = memory_stats() {
                tracing::info!(target:"tron_app", "Current physical memory usage: {}", usage.physical_mem);
            } else {
                tracing::info!(target:"tron_app", "Couldn't get the current memory usage :(");
            }
        }
        tokio::time::sleep(tokio::time::Duration::from_secs(60)).await;
    }
}
