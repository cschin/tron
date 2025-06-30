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
    send_sse_msg_to_client, tron_components::{
        self, button,
        deckgl_plot::{
            TnDeckGLPlot,
        },
        div::{clean_div_with_context, update_and_send_div_with_context, TnDivBuilder},
        file_upload::TnDnDFileUploadBuilder,
        text::{
            append_and_update_stream_textarea_with_context, clean_stream_textarea_with_context,
            clean_textarea_with_context, update_and_send_textarea_with_context,
        },
        tn_future, TnActionExecutionMethod, TnActionFn, TnAsset, TnChatBox, TnD3Plot, TnDiv,
        TnDnDFileUpload, TnFutureHTMLResponse, TnFutureString, TnHtmlResponse, TnServiceRequestMsg,
        TnStreamTextArea,
    }, AppData, Ports, TnServerEventData, TnSseTriggerMsg, TRON_APP
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentState, TnComponentValue, TnContext, TnContextBase,
    TnEvent, TnTextArea,
};

use once_cell::sync::Lazy;
use std::io::{BufRead, BufReader};
use std::{
    collections::{HashMap, VecDeque},
    sync::Arc,
    task::Context,
    vec,
};


#[tokio::main]
async fn main() {

    let app_config = tron_app::AppConfigure {
        address: [0, 0, 0, 0],
        ports: Ports {http:8083, https:3003},
        http_only: true,
        cognito_login: false,
        ..Default::default()
    };
    // set app state
    let app_share_data = AppData::builder(build_context, layout).build();

    tron_app::run(app_share_data, app_config).await
}

static DECKGL_AREA_1: &str = "deck_gl_area_1";
static DECKGL_AREA_2: &str = "deck_gl_area_2";

fn build_context() -> TnContext {
    let mut context = TnContextBase::default();


    let deckgl_plot_script = include_str!("../templates/deckgl_plot_script.html").to_string();
    TnDeckGLPlot::builder()
        .init(DECKGL_AREA_1.into(), deckgl_plot_script)
        .set_attr("class", "w-full h-full p-1 border-solid")
        .add_to_context(&mut context);

    TnDeckGLPlot::builder()
        .init(DECKGL_AREA_2.into(), "".to_string())
        .set_attr("class", "w-full h-full p-1 border-solid")
        .add_to_context(&mut context);

    let context = TnContext {
        base: Arc::new(RwLock::new(context)),
    };

    context
}


#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    deckgl_area_1: String,
    deckgl_area_2: String,
}

fn layout(context: TnContext) -> TnFutureString {
    tn_future! {let context_guard = context.read().await;
        let deckgl_area_1 = context_guard.get_initial_rendered_string(DECKGL_AREA_1).await;
        let deckgl_area_2 = context_guard.get_initial_rendered_string(DECKGL_AREA_2).await;
        let html = AppPageTemplate {
            deckgl_area_1,
            deckgl_area_2,
        };
        html.render().unwrap()
    }
}

