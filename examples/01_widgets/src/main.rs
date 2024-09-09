use askama::Template;
use axum::{
    http::{HeaderMap, HeaderName, HeaderValue},
    response::Html,
};
use futures_util::Future;

//use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use serde_json::Value;

use tracing::debug;

use tron_app::tron_components::{
    checklist, radio_group,
    text::{self, append_and_update_stream_textarea_with_context, append_textarea_value},
    TnActionExecutionMethod, TnButton, TnComponentState, TnComponentValue, TnContext,
    TnContextBase, TnDnDFileUpload, TnEvent, TnFileUpload, TnHtmlResponse, TnRangeSlider, TnSelect,
};

//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, str::FromStr, sync::Arc};

// This is the main entry point of the application
// It sets up the application configuration and state
// and then starts the application by calling tron_app::run
#[tokio::main]
async fn main() {
    let app_config = tron_app::AppConfigure {
        cognito_login: false,
        http_only: true,
        log_level: Some("server=debug,tower_http=debug,tron_app=info"),
        ..Default::default()
    };
    // set app state
    let app_share_data = tron_app::AppData::builder(build_session_context, layout).build();
    tron_app::run(app_share_data, app_config).await
}

/// Builds the initial context for the session.
/// This function creates and adds various components to the context,
/// such as buttons, textareas, checklists, radio groups, a select dropdown,
/// a range slider, and buttons for cleaning the textareas and text input.

fn build_session_context() -> TnContext {
    let mut context = TnContextBase::<'static>::default();
    let mut btn_idx = 0_u32;
    loop {
        TnButton::builder()
            .init(format!("btn-{:02}", btn_idx), format!("{:02}", btn_idx))
            .set_attribute(
                "class",
                "btn btn-sm btn-outline btn-primary flex-1",
            )
            .set_action(TnActionExecutionMethod::Spawn, counter_btn_clicked)
            .add_to_context(&mut context);

        btn_idx += 1;
        if btn_idx >= 10 {
            break;
        }
    }

    text::TnStreamTextArea::builder()
        .init(
            "stream_textarea".into(),
            vec!["This is a streamable textarea\n".to_string()],
        )
        .set_attribute(
            "class",
            "textarea textarea-bordered flex-1 h-20",
        )
        .add_to_context(&mut context);

    text::TnTextArea::builder()
        .init("textarea".into(), "This is a textarea\n".to_string())
        .set_attribute(
            "class",
            "textarea textarea-bordered flex-1 h-20",
        )
        .add_to_context(&mut context);

    let checklist_items = vec![
        ("checkbox-1".to_string(), "CHECKBOX 1".to_string()),
        ("checkbox-2".to_string(), "CHECKBOX 2".to_string()),
        ("checkbox-3".to_string(), "CHECKBOX 3".to_string()),
        ("checkbox-4".to_string(), "CHECKBOX 4".to_string()),
        ("checkbox-5".to_string(), "CHECKBOX 5".to_string()),
        ("checkbox-6".to_string(), "CHECKBOX 6".to_string()),
    ];
    let checklist_tron_id = "checklist".to_string();
    let container_attributes = vec![("class".to_string(), "flex-1".to_string())];
    checklist::add_checklist_to_context(
        &mut context,
        &checklist_tron_id,
        checklist_items,
        container_attributes,
    );
    {
        let component_guard = context.components.blocking_read();
        let checklist_guard = component_guard.get(&checklist_tron_id).unwrap();
        checklist_guard
            .blocking_write()
            .set_attribute("class", "flex flex-row p-1 flex-1");
    }

    let radio_group_items = vec![
        ("radio-1".to_string(), "Radio 1".to_string()),
        ("radio-2".to_string(), "Radio 2".to_string()),
        ("radio-3".to_string(), "Radio 3".to_string()),
        ("radio-4".to_string(), "Radio 4".to_string()),
    ];
    let radio_group_tron_id = "radio_group".to_string();
    let container_attributes = vec![("class".to_string(), "flex-1".to_string())];
    radio_group::add_radio_group_to_context(
        &mut context,
        &radio_group_tron_id,
        radio_group_items,
        container_attributes,
        "radio-1".into(),
    );
    {
        let component_guard = context.components.blocking_read();
        let radio_group_guard = component_guard.get(&radio_group_tron_id).unwrap();
        radio_group_guard
            .blocking_write()
            .set_attribute("class", "flex flex-row p-1 flex-1");
    }
    {
        let select_options = vec![
            ("one".into(), "One".into()),
            ("two".into(), "Two".into()),
            ("three".into(), "Three".into()),
        ];

        TnSelect::builder()
            .init("select_one".into(), "one".into(), select_options)
            .add_to_context(&mut context);
    }
    {
        TnRangeSlider::builder()
            .init("slider".into(), 0.0, 0.0, 100.0)
            .set_attribute("class", "flex-1")
            .set_action(TnActionExecutionMethod::Await, slider_value_update)
            .add_to_context(&mut context);
    }
    {
        TnButton::builder()
            .init(
                "clean_stream_textarea".into(),
                "clean_stream_textarea".into(),
            )
            .set_attribute(
                "class",
                "btn btn-sm btn-outline btn-primary flex-1",
            )
            .set_attribute("hx-target", "#stream_textarea")
            .set_action(TnActionExecutionMethod::Await, clean_stream_textarea)
            .add_to_context(&mut context);
    }
    {
        TnButton::builder()
            .init("clean_textarea".into(), "clean_textarea".into())
            .set_attribute(
                "class",
                "btn btn-sm btn-outline btn-primary flex-1",
            )
            .set_action(TnActionExecutionMethod::Await, clean_textarea)
            .add_to_context(&mut context);
    }
    {
        TnButton::builder()
            .init("clean_textinput".into(), "clean_textinput".into())
            .set_attribute(
                "class",
                "btn btn-sm btn-outline btn-primary flex-1",
            )
            .set_action(TnActionExecutionMethod::Await, clean_textinput)
            .add_to_context(&mut context);
    }
    {
        text::TnTextInput::builder()
            .init("textinput".into(), "".into())
            .set_attribute("class", "input input-bordered w-full")
            .add_to_context(&mut context);
    }
    {
        let button_attributes = vec![(
            "class".into(),
            "btn btn-sm btn-outline btn-primary flex-1".into(),
        )]
        .into_iter()
        .collect::<HashMap<String, String>>();
        TnFileUpload::builder()
            .init(
                "file_upload".into(),
                "Upload File".into(),
                button_attributes,
            )
            .set_action(TnActionExecutionMethod::Await, handle_file_upload)
            .add_to_context(&mut context);
    }

    {
        let button_attributes = vec![(
            "class".into(),
            "btn btn-sm btn-outline btn-primary flex-1".into(),
        )]
        .into_iter()
        .collect::<HashMap<String, String>>();

        TnDnDFileUpload::builder()
            .init(
                "dnd_file_upload".into(),
                "Drop A File".into(),
                button_attributes,
            )
            .set_action(TnActionExecutionMethod::Await, handle_file_upload)
            .add_to_context(&mut context);
    }

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}

/// Test event actions for buttons with IDs `btn-00` to `btn-09`.
///
/// This function is called when a button with an ID from `btn-00` to `btn-09` is clicked.
/// It performs the following actions:
///
/// 1. Logs the event information to the console.
/// 2. If the event type is "server_event", it renders the component associated with the
///    event trigger and returns the rendered HTML.
/// 3. Otherwise, it starts a loop that runs for 10 iterations:
///     - Updates the value and state of the clicked button.
///     - Appends a new line to the "stream_textarea" component with the button ID and current value.
///     - Appends a new line to the "textarea" component with the button ID and current value.
///     - Sends a server-sent event (SSE) message to update the UI state of the button, "stream_textarea",
///       and "textarea" components.
///     - Waits for 100 milliseconds before the next iteration.
/// 4. After the loop, it sets the state of the clicked button to "ready" and sends a final SSE message
///    to update its UI state.
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
/// * `event` - The `TnEvent` object representing the triggered event.
/// * `_payload` - The payload data associated with the event (not used in this function).
///
/// # Returns
///
/// A `Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>>` representing the asynchronous
/// operation that generates the HTML response.
fn counter_btn_clicked(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        tracing::debug!(target:"tron_app", "{:?}", event);
        if event.e_type == "server_event" {
            tracing::debug!(target:"tron_app", "in server_event");
            let html = context.render_component(&event.e_trigger).await;
            return Some((HeaderMap::new(), Html::from(html)));
        };

        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(100));
        let mut i = 0;
        let sse_tx = {
            let context_guard = context.read().await;
            let channel_guard = context_guard.sse_channel.read().await;
            channel_guard.as_ref().unwrap().tx.clone()
        };

        debug!("Event: {:?}", event.clone());
        loop {
            let v = {
                let context_guard = context.write().await;
                let v;
                {
                    let mut components_guard = context_guard.components.write().await;
                    {
                        let btn = components_guard
                            .get_mut(&event.e_trigger.clone())
                            .unwrap()
                            .read()
                            .await;
                        v = match btn.value() {
                            TnComponentValue::String(s) => s.parse::<u32>().unwrap(),
                            _ => 0,
                        };
                    }
                    {
                        let mut btn = components_guard
                            .get_mut(&event.e_trigger.clone())
                            .unwrap()
                            .write()
                            .await;

                        btn.set_value(TnComponentValue::String(format!("{:02}", v + 1)));
                        btn.set_state(TnComponentState::Updating);
                    }
                }

                {
                    let mut components_guard = context_guard.components.write().await;
                    let textarea = components_guard.get_mut("textarea").unwrap().clone();
                    let new_str = format!("{} -- {:02};", event.e_trigger, v);
                    append_textarea_value(textarea, &new_str, Some("\n")).await;
                };
                v
            };

            let msg = format!(
                r##"{{"server_event_data": {{ "target":"{}", "new_state":"updating" }} }}"##,
                event.e_trigger
            );
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
            }

            let msg =
                r##"{"server_event_data": { "target":"textarea", "new_state":"ready" } }"##
                    .to_string();
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
            }

            {
                let new_str = format!("{} -- {:02};\n", event.e_trigger, v);
                append_and_update_stream_textarea_with_context(
                    &context,
                    "stream_textarea",
                    &new_str,
                )
                .await;
            }

            i += 1;
            interval.tick().await;
            debug!("loop triggered: {} {}", event.e_trigger, i);

            if i > 9 {
                break;
            };
        }
        {
            let context_guard = context.write().await;
            let mut components_guard = context_guard.components.write().await;
            let mut btn = components_guard
                .get_mut(&event.e_trigger.clone())
                .unwrap()
                .write()
                .await;
            btn.set_state(TnComponentState::Ready);
            let data = format!(
                r##"{{"server_event_data": {{ "target":"{}", "new_state":"{}" }} }}"##,
                event.e_trigger, "ready"
            );
            if sse_tx.send(data).await.is_err() {
                tracing::debug!(target: "tron_app", "tx dropped");
            }
        }
        None
    };
    Box::pin(f())
}

/// Cleans the stream textarea component.
///
/// This function handles two types of events:
///
/// 1. "click" event: It sets the value of the target component to an empty vector of strings,
///    and returns an HTML response with an empty string and a header indicating that the
///    response should replace the innerHTML of the target component.
///
/// 2. "clean_stream_textarea" event: It renders the target component and returns an HTML
///    response with the rendered HTML.
///
/// If the event type is neither "click" nor "clean_stream_textarea", it returns `None`.
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
/// * `event` - The `TnEvent` object representing the triggered event.
/// * `_payload` - The payload data associated with the event (not used in this function).
///
/// # Returns
///
/// A `Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>>` representing the asynchronous
/// operation that generates the HTML response, or `None` if the event type is not handled.

fn clean_stream_textarea(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        if let Some(target) = event.h_target {
            text::clean_stream_textarea_with_context(&context, &target).await;
            None
        } else {
            None
        }
    };
    Box::pin(f())
}

fn clean_textarea(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        text::clean_textarea_with_context(&context, "textarea").await;
        context.set_ready_for(&event.e_trigger).await;
        let html = context.render_component(&event.e_trigger).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f())
}

/// Cleans the textarea component.
///
/// This function cleans the value of the textarea component by calling the
/// `clean_textarea_with_context` function from the `text` module. It then sets
/// the state of the component that triggered the event to "ready" and renders
/// the HTML for that component.
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
/// * `event` - The `TnEvent` object representing the triggered event.
/// * `_payload` - The payload data associated with the event (not used in this function).
///
/// # Returns
///
/// A `Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>>` representing the asynchronous
/// operation that generates the HTML response for the component that triggered the event.
fn clean_textinput(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        text::clean_textinput_with_context(context.clone(), "textinput").await;
        context.set_ready_for(&event.e_trigger).await;
        let html = context.render_component(&event.e_trigger).await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f())
}

/// Updates the stream textarea with the new value of the slider component.
///
/// This function is triggered when the value of the slider component changes. It performs
/// the following actions:
///
/// 1. Retrieves the "stream_textarea" component from the context.
/// 2. Retrieves the slider component that triggered the event from the context.
/// 3. If the value of the slider component is a string, formats a new string with the
///    component ID and the new value of the slider.
/// 4. Appends the formatted string to the "stream_textarea" component.
/// 5. Sends a server-sent event (SSE) message to update the UI state of the "stream_textarea"
///    component to "ready".
/// 6. Renders the HTML for the slider component and returns it as the response, along with
///    an empty header map.
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
/// * `event` - The `TnEvent` object representing the triggered event.
/// * `_payload` - The payload data associated with the event (not used in this function).
///
/// # Returns
///
/// A `Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>>` representing the asynchronous
/// operation that generates the HTML response for the slider component.
fn slider_value_update(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        let slider = context.get_component(&event.e_trigger).await;
        if let TnComponentValue::String(s) = slider.read().await.value() {
            let new_str = format!("{} -- Value {};\n", event.e_trigger, s);
            append_and_update_stream_textarea_with_context(&context, "stream_textarea", &new_str)
                .await;
        }
        let html: String = context.render_component("slider").await;
        Some((HeaderMap::new(), Html::from(html)))
    };
    Box::pin(f())
}

/// Handles file upload events by processing uploaded file metadata and returning
/// an HTML response to be displayed on the client-side.
///
/// This function captures and logs the details of files uploaded via an event,
/// formats these details into HTML content, and returns a `Future` containing the
/// response to update the client-side view dynamically.
///
/// # Parameters
/// - `_context: TnContext`:
///   Context of the transaction, not used within this function but available for
///   potential future extensions or context-specific handling.
///
/// - `_event: TnEvent`:
///   Event details specific to the file upload. Currently not utilized directly
///   within the function, provided for compatibility and future use.
///
/// - `payload: Value`:
///   JSON-like structure containing file data. Expected to contain:
///   `["event_data"]["e_file_list"]`, an array where each element is an array representing
///   a file's metadata: `[filename (String), size (u64), type (String)]`.
///
/// # Returns
/// `Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>>`:
/// A `Future` that resolves to an HTML formatted response. The response is designed to be
/// compatible with multi-threaded environments, suitable for asynchronous execution.
///
fn handle_file_upload(
    _context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        // process the "finished" event
        tracing::info!(target: "tron_app", "event payload: {:?}", payload["event_data"]["e_file_list"]);
        let file_list = payload["event_data"]["e_file_list"].as_array();

        let file_list = if let Some(file_list) = file_list {
            file_list
                .iter()
                .flat_map(|v| {
                    if let Value::Array(v) = v {
                        tracing::debug!(target: "tron_app", "v:{:?}", v);
                        let filename = v[0].as_str();
                        let size = v[1].as_u64();
                        let t = v[2].as_str();
                        match (filename, size, t) {
                            (Some(filename), Some(size), Some(t)) => {
                                Some(format!("<p>{filename}:{size}:{t}</p>"))
                            }
                            _ => None,
                        }
                    } else {
                        None
                    }
                })
                .collect::<Vec<_>>()
        } else {
            vec![]
        }
        .join("\n");

        tracing::debug!(target: "tron_app", "file_list: {:?}", file_list);
        let mut header = HeaderMap::new();
        let tron_id = event.e_trigger;
        let html = format!(
            r##"
            <div id="file_uploaded_container">
                <dialog id="file_uploaded" class="modal">
                    <div class="modal-box">
                        <form method="dialog">
                            <button class="btn btn-sm btn-circle btn-ghost absolute right-2 top-2" onclick="htmx.find('#{tron_id}_progress').setAttribute('value',  0)">✕</button>
                        </form>
                        <h3 class="font-bold text-lg">File Uploaded</h3>
                        <p class="py-2"> {file_list} </p>
                        <p class="py-4">Press ESC key or click on ✕ button to close</p>
                    </div>
                </dialog>
                <script> 
                    htmx.on('#file_uploaded_container', 'htmx:afterSettle', function(evt) {{
                        document.getElementById('file_uploaded').showModal();
                }}); 
                </script>
            </div>"##
        );
        header.insert(
            HeaderName::from_str("hx-reswap").unwrap(),
            HeaderValue::from_str("outerHTML").unwrap(),
        );
        header.insert(
            HeaderName::from_str("hx-retarget").unwrap(),
            HeaderValue::from_str("#file_uploaded_container").unwrap(),
        );
        Some((header, Html::from(html.to_string())))
    };
    Box::pin(f())
}

/// Struct representing the HTML template for the application's main page.
///
/// This struct contains fields for the HTML content of various components, such as buttons,
/// textareas, checklist, radio group, select dropdown, and slider. The fields are populated
/// with the rendered HTML content for the respective components, and the struct is used
/// to generate the complete HTML for the main page by rendering the `app_page.html` template
/// with the provided field values.
///
/// # Fields
///
/// - `buttons`: A `Vec<String>` containing the HTML for the buttons.
/// - `textarea`: A `String` containing the HTML for the textarea.
/// - `stream_textarea`: A `String` containing the HTML for the stream textarea.
/// - `textinput`: A `String` containing the HTML for the text input.
/// - `checklist`: A `String` containing the HTML for the checklist.
/// - `radio_group`: A `String` containing the HTML for the radio group.
/// - `select`: A `String` containing the HTML for the select dropdown.
/// - `clean_stream_textarea`: A `String` containing the HTML for the button to clean the stream textarea.
/// - `clean_textarea`: A `String` containing the HTML for the button to clean the textarea.
/// - `clean_textinput`: A `String` containing the HTML for the button to clean the text input.
/// - `slider`: A `String` containing the HTML for the slider.
#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative  to the `templates` dir in the crate root
struct AppPageTemplate {
    buttons: Vec<String>,
    textarea: String,
    stream_textarea: String,
    textinput: String,
    checklist: String,
    radio_group: String,
    select: String,
    clean_stream_textarea: String,
    clean_textarea: String,
    clean_textinput: String,
    slider: String,
    file_upload: String,
    dnd_file_upload: String,
}

/// Generates the HTML layout for the application's main page.
///
/// This function uses the `AppPageTemplate` struct to render the HTML template for the main page.
/// It retrieves the HTML content for each component (buttons, textareas, checklist, radio group,
/// select dropdown, and slider) by rendering them individually using the `render_to_string` and
/// `first_render_to_string` methods of the `TnContext` struct.
///
/// The rendered HTML content for each component is then passed to an instance of the
/// `AppPageTemplate` struct, which is rendered using the `render` method to generate the complete
/// HTML for the main page.
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
///
/// # Returns
///
/// A `String` containing the HTML for the application's main page.

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let buttons = (0..10)
        .map(|i| context_guard.render_to_string(&format!("btn-{:02}", i)))
        .collect::<Vec<String>>();

    let context_guard = context.blocking_read();
    let textarea = context_guard.render_to_string("textarea");
    let stream_textarea = context_guard.first_render_to_string("stream_textarea");
    let textinput = context_guard.render_to_string("textinput");
    let checklist = context_guard.render_to_string("checklist");
    let radio_group = context_guard.render_to_string("radio_group");
    let select = context_guard.render_to_string("select_one");
    let clean_stream_textarea = context_guard.render_to_string("clean_stream_textarea");
    let clean_textarea = context_guard.render_to_string("clean_textarea");
    let clean_textinput = context_guard.render_to_string("clean_textinput");
    let slider = context_guard.render_to_string("slider");
    let file_upload = context_guard.render_to_string("file_upload");
    let dnd_file_upload = context_guard.render_to_string("dnd_file_upload");

    let html = AppPageTemplate {
        buttons,
        textarea,
        stream_textarea,
        textinput,
        checklist,
        radio_group,
        select,
        clean_stream_textarea,
        clean_textarea,
        clean_textinput,
        slider,
        file_upload,
        dnd_file_upload,
    };
    html.render().unwrap()
}
