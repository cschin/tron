#![allow(dead_code)]
#![allow(unused_imports)]
use serde::Deserialize;
use std::{
    cmp::{Ordering, Reverse},
    collections::HashSet,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
    str::FromStr,
};

use tower_sessions::Session;

use askama::Template;
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::Future;

use axum::{
    body::Body,
    extract::{Json, Path, State},
    http::{header, HeaderMap, HeaderName, HeaderValue, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
    Router,
};
//use serde::{Deserialize, Serialize};
use tokio::sync::{mpsc::Sender, oneshot, Mutex, OnceCell, RwLock};

use serde_json::{Number, Value};

use tracing::debug;
use tron_app::{
    send_sse_msg_to_client,
    tron_components::{
        self, button,
        chatbox::clean_chatbox_with_context,
        d3_plot::SseD3PlotTriggerMsg,
        div::{clean_div_with_context, update_and_send_div_with_context, TnDivBuilder},
        file_upload::TnDnDFileUploadBuilder,
        text::{
            append_and_update_stream_textarea_with_context, clean_stream_textarea_with_context,
            clean_textarea_with_context, update_and_send_textarea_with_context,
        },
        TnActionExecutionMethod, TnActionFn, TnAsset, TnChatBox, TnD3Plot, TnDiv, TnDnDFileUpload,
        TnHtmlResponse, TnServiceRequestMsg, TnStreamTextArea,
    },
    AppData, TnServerSideTriggerData, TnSseTriggerMsg, TRON_APP,
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnTextArea,
};

use once_cell::sync::Lazy;
use std::io::{BufRead, BufReader};
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
    task::Context,
    vec,
};

mod gs_service;
use gs_service::gs_service;

#[tokio::main]
async fn main() {
    let api_routes =
        Router::<Arc<AppData>>::new().route("/result_images/:image_id", get(get_image));

    let app_config = tron_app::AppConfigure {
        address: [0, 0, 0, 0],
        http_only: true,
        api_router: Some(api_routes),
        cognito_login: false,
        ..Default::default()
    };
    // set app state
    let app_share_data = AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        build_context: Arc::new(Box::new(build_context)),
        build_layout: Arc::new(Box::new(layout)),
    };

    tron_app::run(app_share_data, app_config).await
}

static DND_FILE_UPLOAD: &str = "dnd_file_upload";
static IMAGE_OUTPUT_AREA: &str = "image_output";
static INPUT_IMAGE_AREA: &str = "input_image";
static GS_SERVICE: &str = "gs_service";

fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    add_dnd_file_upload(&mut context, DND_FILE_UPLOAD);

    TnDiv::builder()
        .init(IMAGE_OUTPUT_AREA.into(), "".into())
        .add_to_context(&mut context);

    TnDiv::builder()
        .init(INPUT_IMAGE_AREA.into(), "".into())
        .add_to_context(&mut context);

    let context = TnContext {
        base: Arc::new(RwLock::new(context)),
    };

    // add service
    {
        // service for generating the 2d image gaussian splatter output
        let (gs_request_tx, gs_request_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(1);
        context
            .blocking_write()
            .services
            .insert(GS_SERVICE.into(), (gs_request_tx.clone(), Mutex::new(None)));
        let gs_service = tokio::task::spawn(gs_service(context.clone(), gs_request_rx));

        context.blocking_write().service_handles.push(gs_service);
    }
    context
}

fn add_dnd_file_upload(context: &mut TnContextBase, tnid: &str) {
    let button_attributes = vec![(
        "class".into(),
        "btn btn-sm btn-outline btn-primary flex-1".into(),
    )]
    .into_iter()
    .collect::<HashMap<String, String>>();

    TnDnDFileUpload::builder()
        .init(tnid.into(), "Drop A File".into(), button_attributes)
        .set_action(TnActionExecutionMethod::Await, handle_file_upload)
        .add_to_context(context);
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    input_image_area: String,
    image_output_area: String,
    dnd_file_upload: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let input_image_area = context_guard.first_render_to_string(INPUT_IMAGE_AREA);
    let image_output_area = context_guard.first_render_to_string(IMAGE_OUTPUT_AREA);
    let dnd_file_upload = context_guard.first_render_to_string(DND_FILE_UPLOAD);
    let html = AppPageTemplate {
        input_image_area,
        image_output_area,
        dnd_file_upload,
    };
    html.render().unwrap()
}

async fn get_image(
    State(_app_data): State<Arc<AppData>>,
    session: Session,
    _image_id: String,
) -> impl IntoResponse {
    let mut response_headers = HeaderMap::new();
    response_headers.insert(header::CONTENT_TYPE, "text/html".parse().unwrap());

    let _session_id = if let Some(session_id) = session.id() {
        session_id
    } else {
        return (StatusCode::FORBIDDEN, response_headers, Body::default());
    };

    let body = "";
    let body = Body::from(body);

    (StatusCode::OK, response_headers, body)
}

fn handle_file_upload(
    context: TnContext,
    _event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        // process the "finished" event

        tracing::info!(target: TRON_APP, "process file_upload finish");
        let file_list = payload["event_data"]["e_file_list"].as_array();

        let file_list = if let Some(file_list) = file_list {
            file_list
                .iter()
                .flat_map(|v| {
                    if let Value::Array(v) = v {
                        tracing::debug!(target: TRON_APP, "v:{:?}", v);
                        let filename = v[0].as_str();
                        let size = v[1].as_u64();
                        let t = v[2].as_str();
                        match (filename, size, t) {
                            (Some(filename), Some(size), Some(t)) => Some((filename, size, t)),
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        };
        tracing::info!( target: TRON_APP, "{:?}", file_list);
        if !file_list.is_empty() {
            let gs_service_tx = context.get_service_tx(GS_SERVICE).await;
            let (tx, rx) = oneshot::channel::<String>();

            let &(filename, _size, t) = file_list.last().unwrap();
            if t == "image/png" || t == "image/jpeg" {
                let gs_req_msg = TnServiceRequestMsg {
                    request: "process_image".into(),
                    payload: TnAsset::String(filename.to_string()),
                    response: tx,
                };
                let _ = gs_service_tx.send(gs_req_msg).await;

                if let Ok(out) = rx.await {
                    tracing::debug!(target: TRON_APP, "gs_service returned string: {}", out);
                };
            };
        }

        let header = HeaderMap::new();
        Some((header, Html::from("".to_string())))
    };
    Box::pin(f())
}
