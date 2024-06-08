#![allow(dead_code)]
#![allow(unused_imports)]

mod llm_service;
use llm_service::llm_service;

use flate2::bufread::GzDecoder;
use serde::Deserialize;
use std::{
    cmp::{Ordering, Reverse},
    collections::HashSet,
    fs::File,
    hash::{DefaultHasher, Hash, Hasher},
};
use tower_sessions::Session;

use askama::Template;
use bytes::{BufMut, Bytes, BytesMut};
use futures_util::Future;

use axum::{
    extract::Json,
    http::HeaderMap,
    response::{Html, IntoResponse},
    routing::get,
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
        d3_plot::SseD3PlotTriggerMsg,
        text::{
            append_and_send_stream_textarea_with_context, clean_stream_textarea_with_context,
            clean_textarea_with_context, update_and_send_textarea_with_context,
        },
        TnActionExecutionMethod, TnAsset, TnChatBox, TnD3Plot, TnHtmlResponse, TnServiceRequestMsg,
        TnStreamTextArea,
    },
    AppData, TnServerSideTriggerData, TnSseTriggerMsg,
};
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnEventActions, TnTextArea,
};
//use std::sync::Mutex;
use once_cell::sync::Lazy;
use std::io::{BufRead, BufReader};
use std::{
    collections::{HashMap, VecDeque},
    pin::Pin,
    sync::Arc,
    task::Context,
    vec,
};

static TRON_APP: &str = "tron_app";

static LLM_SERVICE: &str = "llm_service";

static D3PLOT: &str = "d3_plot";
static RESET_BUTTON: &str = "reset_button";
static CONTEXT_QUERY_BUTTON: &str = "context_query_button";
static QUERY_BUTTON: &str = "query_button";
static QUERY_TEXT_INPUT: &str = "query_text_input";
static TOP_HIT_TEXTAREA: &str = "top_hit_textarea";
static QUERY_STREAM_TEXTAREA: &str = "query_stream_textarea";
static QUERY_RESULT_TEXTAREA: &str = "query_result_textarea";
static FIND_RELATED_BUTTON: &str = "find_related_text_button";

#[derive(Deserialize, Debug)]
struct DocumentChunk {
    text: String,
    span: (usize, usize),
    token_ids: Vec<u32>,
    two_d_embedding: (f32, f32),
    embedding_vec: Vec<f32>,
    filename: String,
    title: String,
}

struct DocumentChunks {
    chunks: Vec<DocumentChunk>,
    filename_to_id: HashMap<String, u32>,
}

static DOCUMENT_CHUNKS: OnceCell<DocumentChunks> = OnceCell::const_new();

impl DocumentChunks {
    pub fn global() -> &'static DocumentChunks {
        DOCUMENT_CHUNKS
            .get()
            .expect("document chunks are not initialized")
    }

    fn from_file(filename: String) -> DocumentChunks {
        let mut chunks = Vec::new();
        let file = BufReader::new(File::open(filename).unwrap());
        let decoder = GzDecoder::new(file);
        let reader = BufReader::new(decoder);

        println!("loading data");
        // Read the file line by line
        let mut count = 0;
        let mut filename_to_id = HashMap::<String, u32>::default();
        let mut fid = 0;
        for line in reader.lines() {
            let chunk: DocumentChunk = serde_json::from_str(&line.unwrap()).unwrap();
            let filename = chunk.filename.clone();
            filename_to_id.entry(filename).or_insert_with(|| {
                fid += 1;
                fid - 1
            });
            chunks.push(chunk);
            count += 1;
        }
        println!("{} records loaded", count);

        DocumentChunks {
            chunks,
            filename_to_id,
        }
    }
}

static CMAP: [&str; 97] = [
    "#870098", "#00aaa5", "#3bff00", "#ec0000", "#00a2c3", "#00f400", "#ff1500", "#0092dd",
    "#00dc00", "#ff8100", "#007ddd", "#00c700", "#ffb100", "#0038dd", "#00af00", "#fcd200",
    "#0000d5", "#009a00", "#f1e700", "#0000b1", "#00a55d", "#d4f700", "#4300a2", "#00aa93",
    "#a1ff00", "#dc0000", "#00aaab", "#1dff00", "#f40000", "#009fcb", "#00ef00", "#ff2d00",
    "#008ddd", "#00d700", "#ff9900", "#0078dd", "#00c200", "#ffb900", "#0025dd", "#00aa00",
    "#f9d700", "#0000c9", "#009b13", "#efed00", "#0300aa", "#00a773", "#ccf900", "#63009e",
    "#00aa98", "#84ff00", "#e10000", "#00a7b3", "#00ff00", "#f90000", "#009bd7", "#00ea00",
    "#ff4500", "#0088dd", "#00d200", "#ffa100", "#005ddd", "#00bc00", "#ffc100", "#0013dd",
    "#00a400", "#f7dd00", "#0000c1", "#009f33", "#e8f000", "#1800a7", "#00aa88", "#c4fc00",
    "#78009b", "#00aaa0", "#67ff00", "#e60000", "#00a4bb", "#00fa00", "#fe0000", "#0098dd",
    "#00e200", "#ff5d00", "#0082dd", "#00cc00", "#ffa900", "#004bdd", "#00b400", "#ffc900",
    "#0000dd", "#009f00", "#f4e200", "#0000b9", "#00a248", "#dcf400", "#2d00a4", "#00aa8d",
    "#bcff00",
];

// This is the main entry point of the application
// It sets up the application configuration and state
// and then starts the application by calling tron_app::run
#[tokio::main]
async fn main() {
    let _result = DOCUMENT_CHUNKS
        .get_or_init(|| async { DocumentChunks::from_file("data/all_embedding.jsonl.gz".into()) })
        .await;
    let api_routes: Router<()> = Router::new().route("/test", get(data));

    let app_config = tron_app::AppConfigure {
        http_only: true,
        api_router: Some(api_routes),
        cognito_login: false,
        ..Default::default()
    };
    // set app state
    let app_share_data = AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_context)),
        build_actions: Arc::new(Box::new(build_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };

    tron_app::run(app_share_data, app_config).await
}

// These functions are used to build the application context,
// layout, and event actions respectively
fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    let mut component_index = 0;

    let d3_plot_script = include_str!("../templates/d3_plot_script.html").to_string();
    let mut d3_plot = TnD3Plot::new(component_index, D3PLOT.into(), d3_plot_script);
    // override the default event handler so we can get the transformed coordinates in the plot
    d3_plot.set_attribute(
        "hx-vals".into(),
        r##"js:{event_data:get_event_with_transformed_coordinate(event)}"##.into(),
    );
    context.add_component(d3_plot);

    component_index += 1;
    let mut reset_btn = TnButton::new(component_index, RESET_BUTTON.into(), "Reset".into());
    reset_btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary w-full h-min p-1".to_string(),
    );

    reset_btn.set_attribute("hx-target".to_string(), format!("#{D3PLOT}"));
    reset_btn.set_attribute("hx-swap".to_string(), "none".to_string());
    context.add_component(reset_btn);

    component_index += 1;
    let mut top_hit_textarea = TnTextArea::new(component_index, TOP_HIT_TEXTAREA.into(), "".into());
    top_hit_textarea.set_attribute("class".to_string(), "w-full h-full".to_string());
    top_hit_textarea.set_attribute("style".to_string(), "resize:none".to_string());
    context.add_component(top_hit_textarea);

    component_index += 1;
    let mut context_query_btn = TnButton::new(
        component_index,
        CONTEXT_QUERY_BUTTON.into(),
        "Query With The Hits".into(),
    );
    context_query_btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary w-full h-min p-1 join-item".to_string(),
    );
    context.add_component(context_query_btn);

    component_index += 1;
    let mut query_btn = TnButton::new(component_index, QUERY_BUTTON.into(), "General Query".into());
    query_btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary w-full h-min p-1 join-item".to_string(),
    );
    context.add_component(query_btn);

    component_index += 1;
    let mut query_btn = TnButton::new(component_index, FIND_RELATED_BUTTON.into(), "Find Related Text".into());
    query_btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary w-full h-min p-1 join-item".to_string(),
    );
    context.add_component(query_btn);

    component_index += 1;
    let mut query_text_input = TnTextArea::new(component_index, QUERY_TEXT_INPUT.into(), "".into());
    query_text_input.set_attribute("class".to_string(), "min-h-32 w-full".to_string());
    query_text_input.set_attribute("style".to_string(), "resize:none".to_string());
    query_text_input.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
    query_text_input.set_attribute(
        "hx-vals".into(),
        r##"js:{event_data:get_input_event(event)}"##.into(),
    ); //over-ride the default as we need the value of the input text
    query_text_input.remove_attribute("disabled".into());
    context.add_component(query_text_input);

    component_index += 1;
    let mut query_stream_textarea = TnStreamTextArea::new(
        component_index,
        QUERY_STREAM_TEXTAREA.into(),
        VecDeque::new(),
    );
    query_stream_textarea.set_attribute("class".to_string(), "min-h-24 w-full".to_string());
    query_stream_textarea.set_attribute("style".to_string(), "resize:none".to_string());
    query_stream_textarea.remove_attribute("disabled".into());
    context.add_component(query_stream_textarea);

    // add a chatbox
    component_index += 1;
    let mut query_result_textarea =
        TnChatBox::<'static>::new(component_index, QUERY_RESULT_TEXTAREA.to_string(), vec![]);
    query_result_textarea.set_attribute(
        "class".to_string(),
        "min-h-96 max-h-96 overflow-auto flex-1 p-2".to_string(),
    );

    context.add_component(query_result_textarea);

    {
        // fill in the plot stream data
        let mut stream_data_guard = context.stream_data.blocking_write();
        stream_data_guard.insert(
            "plot_data".into(),
            ("application/text".into(), VecDeque::default()),
        );
        let mut data = VecDeque::default();
        //let raw_data = include_str!("../templates/2_TwoNum.csv").as_bytes();
        //let raw_data = BytesMut::from(raw_data);
        let mut two_d_embeddding = "x,y,c,o\n".to_string();
        let filename_to_id = &DOCUMENT_CHUNKS.get().unwrap().filename_to_id;
        two_d_embeddding.extend([DOCUMENT_CHUNKS
            .get()
            .unwrap()
            .chunks
            .iter()
            .map(|c| {
                let fid = filename_to_id.get(&c.filename).unwrap();
                format!(
                    "{},{},{},0.8",
                    c.two_d_embedding.0,
                    c.two_d_embedding.1,
                    CMAP[(fid % 97) as usize]
                )
            })
            .collect::<Vec<String>>()
            .join("\n")]);
        let two_d_embeddding = BytesMut::from_iter(two_d_embeddding.as_bytes());
        tracing::info!(target: "tron_app 1", "length:{}", two_d_embeddding.len());

        data.push_back(two_d_embeddding);
        stream_data_guard.insert("plot_data".into(), ("application/text".into(), data));
    }

    let context = TnContext {
        base: Arc::new(RwLock::new(context)),
    };
    // add service
    {
        // service handling the LLM and TTS at once
        let (llm_request_tx, llm_request_rx) = tokio::sync::mpsc::channel::<TnServiceRequestMsg>(1);
        context.blocking_write().services.insert(
            LLM_SERVICE.into(),
            (llm_request_tx.clone(), Mutex::new(None)),
        );
        let llm_service = tokio::task::spawn(llm_service(context.clone(), llm_request_rx));

        context.blocking_write().service_handles.push(llm_service);
    }

    context
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    d3_plot: String,
    reset_button: String,
    context_query_button: String,
    query_button: String,
    top_hit_textarea: String,
    query_stream_textarea: String,
    query_result_textarea: String,
    query_text_input: String,
    find_related_text_button: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let d3_plot = context_guard.render_to_string(D3PLOT);
    let reset_button = context_guard.render_to_string(RESET_BUTTON);
    let top_hit_textarea = context_guard.render_to_string(TOP_HIT_TEXTAREA);
    let context_query_button = context_guard.render_to_string(CONTEXT_QUERY_BUTTON);
    let query_stream_textarea = context_guard.first_render_to_string(QUERY_STREAM_TEXTAREA);
    let query_result_textarea = context_guard.first_render_to_string(QUERY_RESULT_TEXTAREA);
    let query_button = context_guard.render_to_string(QUERY_BUTTON);
    let query_text_input = context_guard.render_to_string(QUERY_TEXT_INPUT);
    let find_related_text_button = context_guard.render_to_string(FIND_RELATED_BUTTON);

    let html = AppPageTemplate {
        d3_plot,
        reset_button,
        top_hit_textarea,
        context_query_button,
        query_result_textarea,
        query_stream_textarea,
        query_button,
        query_text_input,
        find_related_text_button,
    };
    html.render().unwrap()
}

fn build_actions(context: TnContext) -> TnEventActions {
    let mut actions = TnEventActions::default();

    let index = context.blocking_read().get_component_index(D3PLOT);
    actions.insert(
        index,
        (TnActionExecutionMethod::Await, Arc::new(d3_plot_clicked)),
    );

    let index = context.blocking_read().get_component_index(RESET_BUTTON);
    actions.insert(
        index,
        (
            TnActionExecutionMethod::Await,
            Arc::new(reset_button_clicked),
        ),
    );

    let index = context.blocking_read().get_component_index(QUERY_BUTTON);
    actions.insert(
        index,
        (
            TnActionExecutionMethod::Await,
            Arc::new(query_button_clicked),
        ),
    );

    let index = context
        .blocking_read()
        .get_component_index(CONTEXT_QUERY_BUTTON);
    actions.insert(
        index,
        (TnActionExecutionMethod::Await, Arc::new(query_with_hits)),
    );

    actions
}
#[derive(Debug, Clone)]
struct TwoDPoint<'a> {
    d: OrderedFloat<f64>,
    point: (f64, f64),
    chunk: &'a DocumentChunk,
}

impl<'a> Ord for TwoDPoint<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        // Notice that the we flip the ordering on costs.
        // In case of a tie we compare positions - this step is necessary
        // to make implementations of `PartialEq` and `Ord` consistent.
        other.d.cmp(&self.d)
    }
}

// `PartialOrd` needs to be implemented as well.
impl<'a> PartialOrd for TwoDPoint<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> PartialEq for TwoDPoint<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.d == other.d
    }
}

impl<'a> Eq for TwoDPoint<'a> {}

use ordered_float::OrderedFloat;
use std::collections::BinaryHeap;

fn hex_color_rescale(hex_color: &str, rescale: f64) -> String {
    let r = i64::from_str_radix(&hex_color[1..3], 16).unwrap() as f64 * rescale;
    let g = i64::from_str_radix(&hex_color[3..5], 16).unwrap() as f64 * rescale;
    let b = i64::from_str_radix(&hex_color[5..7], 16).unwrap() as f64 * rescale;
    let r = r.trunc() as u32;
    let g = g.trunc() as u32;
    let b = b.trunc() as u32;
    let mut hex = 0_u32;
    hex |= r;
    hex <<= 8;
    hex |= g;
    hex <<= 8;
    hex |= b;

    format!("#{:06x}", hex)
}

fn d3_plot_clicked(
    context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app", "event {:?}", event);
        tracing::info!(target: "tron_app", "payload {:?}", payload);
        let mut all_points = Vec::new();
        let evt_x = serde_json::from_value::<f64>(payload["event_data"]["e_x"].clone()).unwrap();
        let evt_y = serde_json::from_value::<f64>(payload["event_data"]["e_y"].clone()).unwrap();
        tracing::info!(target: "tron_app", "e_x {:?}", evt_x);
        tracing::info!(target: "tron_app", "e_y {:?}", evt_y);
        //let filename_to_id = &DOCUMENT_CHUNKS.get().unwrap().filename_to_id;
        DOCUMENT_CHUNKS
            .get()
            .unwrap()
            .chunks
            .iter()
            .step_by(2)
            .for_each(|c| {
                let x = c.two_d_embedding.0 as f64;
                let y = c.two_d_embedding.1 as f64;
                let d = OrderedFloat::from((evt_x - x).powi(2) + (evt_y - y).powi(2));
                let point = TwoDPoint {
                    d,
                    point: (x, y),
                    chunk: c,
                };
                all_points.push(point);
            });
        all_points.sort();
        all_points.reverse();
        let ref_eb_vec = all_points.first().unwrap().chunk.embedding_vec.clone();
        let mut all_points_2 = Vec::new();
        DOCUMENT_CHUNKS
            .get()
            .unwrap()
            .chunks
            .iter()
            .step_by(2)
            .for_each(|c| {
                let x = c.two_d_embedding.0 as f64;
                let y = c.two_d_embedding.1 as f64;
                //let d = OrderedFloat::from((evt_x - x).powi(2) + (evt_y - y).powi(2));
                let d: f64 = (0..c.embedding_vec.len())
                    .map(|idx| (c.embedding_vec[idx] - ref_eb_vec[idx]).powi(2))
                    .sum::<f32>() as f64;
                let d = OrderedFloat::from(d);
                let point = TwoDPoint {
                    d,
                    point: (x, y),
                    chunk: c,
                };
                all_points_2.push(point);
            });

        let mut color_scale = 1.0;
        let mut d_color = 4.0 * color_scale / (all_points.len() as f64);

        all_points_2.sort();
        all_points_2.reverse();
        let top_10 = all_points_2[..10].to_vec();
        let mut two_d_embeddding = "x,y,c,o\n".to_string();
        let filename_to_id = &DOCUMENT_CHUNKS.get().unwrap().filename_to_id;
        two_d_embeddding.extend(
            all_points_2
                .into_iter()
                .map(|p| {
                    let c = p.chunk;
                    let fid = filename_to_id.get(&c.filename).unwrap();

                    color_scale = if color_scale > 0.0 { color_scale } else { 0.0 };

                    color_scale -= d_color;
                    d_color *= 0.999995;
                    let color = CMAP[(fid % 97) as usize];

                    format!("{},{},{},{}\n", p.point.0, p.point.1, color, color_scale)
                })
                .collect::<Vec<String>>(),
        );

        {
            let two_d_embeddding = BytesMut::from_iter(two_d_embeddding.as_bytes());
            let context_guard = context.write().await;
            let mut stream_data_guard = context_guard.stream_data.write().await;
            let data = stream_data_guard.get_mut("plot_data").unwrap();
            data.1.clear();
            tracing::info!(target: "tron_app", "length:{}", two_d_embeddding.len());
            data.1.push_back(two_d_embeddding);
            tracing::info!(target: "tron_app", "stream_data {:?}", data.1[0].len());
        }
        let sse_tx = context.get_sse_tx().await;
        let msg = SseD3PlotTriggerMsg {
            server_side_trigger_data: TnServerSideTriggerData {
                target: D3PLOT.into(),
                new_state: "ready".into(),
            },
            d3_plot: "re-plot".into(),
        };
        send_sse_msg_to_client(&sse_tx, msg).await;

        let mut docs = HashSet::<String>::new();
        let top_doc = top_10
            .iter()
            .flat_map(|p| {
                if docs.contains(&p.chunk.title) {
                    None
                } else {
                    docs.insert(p.chunk.title.clone());
                    Some(p.chunk.title.clone())
                }
            })
            .collect::<Vec<String>>();
        let top_doc = top_doc.join("\n\n");
        update_and_send_textarea_with_context(context.clone(), TOP_HIT_TEXTAREA, &top_doc).await;

        let top_chunk = top_10
            .into_iter()
            .map(|p| {
                let mut text = String::new();
                text.extend(format!("=== CHUNK BGN, TITLE: {}\n", p.chunk.title).chars());
                text.push_str(&p.chunk.text);
                text.push_str("\n=== CHUNK END \n");
                text
            })
            .collect::<Vec<String>>();
        let top_chunk = top_chunk.join("\n");

        clean_stream_textarea_with_context(context.clone(), QUERY_STREAM_TEXTAREA).await;
        append_and_send_stream_textarea_with_context(
            context.clone(),
            QUERY_STREAM_TEXTAREA,
            &top_chunk,
        )
        .await;

        {
            let context_guard = context.write().await;
            let mut asset = context_guard.assets.write().await;
            asset.insert("top_k_chunk".into(), TnAsset::String(top_chunk));
        }

        None
    };
    Box::pin(action)
}

fn reset_button_clicked(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app", "{:?}", event);
        if event.e_trigger != RESET_BUTTON {
            None
        } else {
            {
                let mut two_d_embeddding = "x,y,c,o\n".to_string();
                let filename_to_id = &DOCUMENT_CHUNKS.get().unwrap().filename_to_id;
                two_d_embeddding.extend([DOCUMENT_CHUNKS
                    .get()
                    .unwrap()
                    .chunks
                    .iter()
                    .map(|c| {
                        let fid = filename_to_id.get(&c.filename).unwrap();
                        format!(
                            "{},{},{},0.8",
                            c.two_d_embedding.0,
                            c.two_d_embedding.1,
                            CMAP[(fid % 97) as usize]
                        )
                    })
                    .collect::<Vec<String>>()
                    .join("\n")]);
                let two_d_embeddding = BytesMut::from_iter(two_d_embeddding.as_bytes());
                {
                    let context_guard = context.write().await;
                    let mut stream_data_guard = context_guard.stream_data.write().await;
                    let data = stream_data_guard.get_mut("plot_data").unwrap();
                    data.1.clear();
                    tracing::info!(target: "tron_app", "length:{}", two_d_embeddding.len());
                    data.1.push_back(two_d_embeddding);
                    tracing::info!(target: "tron_app", "stream_data {:?}", data.1[0].len());
                }
            }
            {
                let sse_tx = context.get_sse_tx().await;
                let msg = SseD3PlotTriggerMsg {
                    server_side_trigger_data: TnServerSideTriggerData {
                        target: D3PLOT.into(),
                        new_state: "ready".into(),
                    },
                    d3_plot: "re-plot".into(),
                };
                send_sse_msg_to_client(&sse_tx, msg).await;
            }

            clean_textarea_with_context(context.clone(), TOP_HIT_TEXTAREA).await;

            clean_stream_textarea_with_context(context.clone(), QUERY_STREAM_TEXTAREA).await;

            None
        }
    };
    Box::pin(action)
}

fn query_button_clicked(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        if event.e_trigger != QUERY_BUTTON {
            return None;
        };
        let query_text = context.get_value_from_component(QUERY_TEXT_INPUT).await;
        let query_text = if let TnComponentValue::String(s) = query_text {
            s
        } else {
            unreachable!()
        };

        let llm_tx = context.get_service_tx(LLM_SERVICE).await;
        let (tx, rx) = oneshot::channel::<String>();

        let llm_req_msg = TnServiceRequestMsg {
            request: "chat-complete".into(),
            payload: TnAsset::String(query_text.clone()),
            response: tx,
        };
        let _ = llm_tx.send(llm_req_msg).await;

        if let Ok(out) = rx.await {
            tracing::debug!(target: TRON_APP, "returned string: {}", out);
        };

        None
    };
    Box::pin(action)
}

fn query_with_hits(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        if event.e_trigger != CONTEXT_QUERY_BUTTON {
            return None;
        };

        let query_text = context.get_value_from_component(QUERY_TEXT_INPUT).await;

        let query_text = if let TnComponentValue::String(s) = query_text {
            s
        } else {
            unreachable!()
        };

        {
            let context_guard = context.read().await;
            let asset = context_guard.assets.read().await;

            let mut query_text = if query_text.len() > 5 {
                query_text.clone()
            } else {
                "Please summarize ".to_string()
            };

            if let Some(TnAsset::String(s)) = asset.get("top_k_chunk") {
                query_text.push_str("\n with the following chunks of text:\n");
                query_text.push_str(s);
            };

            let llm_tx = context.get_service_tx(LLM_SERVICE).await;
            let (tx, rx) = oneshot::channel::<String>();

            let llm_req_msg = TnServiceRequestMsg {
                request: "chat-complete".into(),
                payload: TnAsset::String(query_text.clone()),
                response: tx,
            };
            let _ = llm_tx.send(llm_req_msg).await;

            if let Ok(out) = rx.await {
                tracing::debug!(target: TRON_APP, "returned string: {}", out);
            };
        }

        None
    };
    Box::pin(action)
}

async fn data() -> impl IntoResponse {
    let len = DOCUMENT_CHUNKS.get().unwrap().chunks.len();
    Html::from(format!("test: {}", len)).into_response()
}
