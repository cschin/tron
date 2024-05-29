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
use tron_app::tron_components::{self, TnFileUpload};
use tron_components::{
    checklist, radio_group,
    text::{self, append_stream_textarea, append_textarea_value},
    TnActionExecutionMethod, TnActionFn, TnButton, TnComponentBaseTrait, TnComponentState,
    TnComponentValue, TnContext, TnContextBase, TnEvent, TnEventActions, TnHtmlResponse,
    TnRangeSlider, TnSelect, TnStreamTextArea, TnTextArea, TnTextInput,
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
    let app_share_data = tron_app::AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_session_context)),
        build_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, app_config).await
}

/// Builds the initial context for the session.
/// This function creates and adds various components to the context,
/// such as buttons, textareas, checklists, radio groups, a select dropdown,
/// a range slider, and buttons for cleaning the textareas and text input.

fn build_session_context() -> TnContext {
    let mut context = TnContextBase::<'static>::default();

    let mut component_index = 0_u32;
    loop {
        let mut btn = TnButton::<'static>::new(
            component_index,
            format!("btn-{:02}", component_index),
            format!("{:02}", component_index),
        );

        btn.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );

        context.add_component(btn);

        component_index += 1;
        if component_index >= 10 {
            break;
        }
    }

    component_index += 1;
    let mut stream_textarea = TnStreamTextArea::<'static>::new(
        component_index,
        "stream_textarea".into(),
        vec!["This is a stream-able textarea\n".to_string()],
    );

    stream_textarea.set_attribute(
        "class".into(),
        "textarea textarea-bordered flex-1 h-20".into(),
    );

    context.add_component(stream_textarea);

    component_index += 1;
    let mut textarea = TnTextArea::<'static>::new(
        component_index,
        "textarea".into(),
        "This is a textarea\n".to_string(),
    );

    textarea.set_attribute(
        "class".into(),
        "textarea textarea-bordered flex-1 h-20".into(),
    );

    context.add_component(textarea);

    component_index += 1;

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
        &mut component_index,
        checklist_tron_id,
        checklist_items,
        container_attributes,
    );
    {
        let component_guard = context.components.blocking_read();
        let checklist_guard = component_guard.get(&component_index).unwrap();
        checklist_guard
            .blocking_write()
            .set_attribute("class".into(), "flex flex-row p-1 flex-1".into());
    }

    component_index += 1;
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
        &mut component_index,
        radio_group_tron_id,
        radio_group_items,
        container_attributes,
        "radio-1".into(),
    );
    {
        let component_guard = context.components.blocking_read();
        let radio_group_guard = component_guard.get(&component_index).unwrap();
        radio_group_guard
            .blocking_write()
            .set_attribute("class".into(), "flex flex-row p-1 flex-1".into());
    }
    {
        component_index += 1;
        let select_options = vec![
            ("one".into(), "One".into()),
            ("two".into(), "Two".into()),
            ("three".into(), "Three".into()),
        ];

        let select = TnSelect::<'static>::new(
            component_index,
            "select_one".into(),
            "one".into(),
            select_options,
        );
        context.add_component(select);
    }
    {
        component_index += 1;
        let mut slider =
            TnRangeSlider::<'static>::new(component_index, "slider".into(), 0.0, 0.0, 100.0);
        slider.set_attribute("class".to_string(), "flex-1".to_string());
        context.add_component(slider);
    }
    {
        component_index += 1;
        let mut clean_button = TnButton::<'static>::new(
            component_index,
            "clean_stream_textarea".into(),
            "clean_stream_textarea".into(),
        );
        clean_button.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        clean_button.set_attribute("hx-target".to_string(), "#stream_textarea".to_string());
        context.add_component(clean_button);
    }
    {
        component_index += 1;
        let mut clean_button = TnButton::<'static>::new(
            component_index,
            "clean_textarea".into(),
            "clean_textarea".into(),
        );
        clean_button.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        context.add_component(clean_button);
    }
    {
        component_index += 1;
        let mut clean_button = TnButton::<'static>::new(
            component_index,
            "clean_textinput".into(),
            "clean_textinput".into(),
        );
        clean_button.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        context.add_component(clean_button);
    }
    {
        component_index += 1;
        let mut textinput = TnTextInput::new(component_index, "textinput".into(), "".into());
        textinput.set_attribute("class".into(), "input input-bordered w-full".into());

        context.add_component(textinput);
    }
    {
        component_index += 1;
        let button_attributes = vec![(
            "class".into(),
            "btn btn-sm btn-outline btn-primary flex-1".into(),
        )]
        .into_iter()
        .collect::<HashMap<String, String>>();
        let file_upload = TnFileUpload::new(
            component_index,
            "file_upload".into(),
            "Upload File".into(),
            button_attributes,
        );

        context.add_component(file_upload);
    }

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}

/// Builds the event actions for the session.
///
/// This function creates a vector of tuples containing the component ID, execution method,
/// and action function for each component. It then maps these tuples to the corresponding
/// component indices in the `TnContext` and returns a `TnEventActions` object.
///
/// The event actions include:
///
/// - Test actions for buttons with IDs `btn-00` to `btn-09`
/// - Actions for the checklist component
/// - Actions for the radio group component
/// - Action for updating the slider value
/// - Actions for cleaning the stream textarea, textarea, and text input
///
/// # Arguments
///
/// * `context` - The `TnContext` object containing the components and their states.
///
/// # Returns
///
/// A `TnEventActions` object mapping component indices to their corresponding action functions.
fn build_session_actions(context: TnContext) -> TnEventActions {
    let mut actions = Vec::<(String, TnActionExecutionMethod, TnActionFn)>::new();
    for i in 0..10 {
        actions.push((
            format!("btn-{:02}", i),
            TnActionExecutionMethod::Spawn,
            test_event_actions,
        ));
    }
    {
        let checklist = context.blocking_get_component("checklist");
        let checklist_actions = checklist::get_checklist_actions(checklist);
        checklist_actions.into_iter().for_each(|(tron_id, action)| {
            actions.push((tron_id, TnActionExecutionMethod::Await, action));
        });
    }

    {
        let radio_group: Arc<RwLock<Box<dyn TnComponentBaseTrait<'_>>>> =
            context.blocking_get_component("radio_group");
        let radio_group_actions = radio_group::get_radio_group_actions(radio_group);
        radio_group_actions
            .into_iter()
            .for_each(|(tron_id, action)| {
                actions.push((tron_id, TnActionExecutionMethod::Await, action));
            });
    }

    actions.push((
        "slider".into(),
        TnActionExecutionMethod::Await,
        slider_value_update,
    ));

    actions.push((
        "clean_stream_textarea".into(),
        TnActionExecutionMethod::Await,
        clean_stream_textarea,
    ));

    actions.push((
        "clean_textarea".into(),
        TnActionExecutionMethod::Await,
        clean_textarea,
    ));

    actions.push((
        "clean_textinput".into(),
        TnActionExecutionMethod::Await,
        clean_textinput,
    ));

    actions
        .into_iter()
        .map(|(id, exe_method, action_fn)| {
            let idx = context.blocking_read().get_component_index(&id);
            (idx, (exe_method, Arc::new(action_fn)))
        })
        .collect::<TnEventActions>()
}

/// Test event actions for buttons with IDs `btn-00` to `btn-09`.
///
/// This function is called when a button with an ID from `btn-00` to `btn-09` is clicked.
/// It performs the following actions:
///
/// 1. Logs the event information to the console.
/// 2. If the event type is "server_side_trigger", it renders the component associated with the
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
fn test_event_actions(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = || async move {
        tracing::info!(target:"tron_app", "{:?}", event);
        if event.e_type == "server_side_trigger" {
            tracing::info!(target:"tron_app", "in server_side_trigger");
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
            {
                let context_guard = context.write().await;
                let v;
                {
                    let id = context_guard.get_component_index(&event.e_trigger.clone());
                    let mut components_guard = context_guard.components.write().await;
                    {
                        let btn = components_guard.get_mut(&id).unwrap().read().await;
                        v = match btn.value() {
                            TnComponentValue::String(s) => s.parse::<u32>().unwrap(),
                            _ => 0,
                        };
                    }
                    {
                        let mut btn = components_guard.get_mut(&id).unwrap().write().await;

                        btn.set_value(TnComponentValue::String(format!("{:02}", v + 1)));
                        btn.set_state(TnComponentState::Updating);
                    }
                }

                {
                    let id = context_guard.get_component_index("stream_textarea");
                    let mut components_guard = context_guard.components.write().await;
                    let stream_textarea = components_guard.get_mut(&id).unwrap().clone();
                    let new_str = format!("{} -- {:02};\n", event.e_trigger, v);
                    append_stream_textarea(stream_textarea, &new_str).await;
                };

                {
                    let id = context_guard.get_component_index("textarea");
                    let mut components_guard = context_guard.components.write().await;
                    let textarea = components_guard.get_mut(&id).unwrap().clone();
                    let new_str = format!("{} -- {:02};", event.e_trigger, v);
                    append_textarea_value(textarea, &new_str, Some("\n")).await;
                };
            }

            let msg = format!(
                r##"{{"server_side_trigger_data": {{ "target":"{}", "new_state":"updating" }} }}"##,
                event.e_trigger
            );
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
            }

            let msg =
            r##"{"server_side_trigger_data": { "target":"stream_textarea", "new_state":"ready" } }"##
                .to_string();
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
            }

            let msg =
                r##"{"server_side_trigger_data": { "target":"textarea", "new_state":"ready" } }"##
                    .to_string();
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
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
            let id = context_guard.get_component_index(&event.e_trigger.clone());
            let mut components_guard = context_guard.components.write().await;
            let mut btn = components_guard.get_mut(&id).unwrap().write().await;
            btn.set_state(TnComponentState::Ready);
            let data = format!(
                r##"{{"server_side_trigger_data": {{ "target":"{}", "new_state":"{}" }} }}"##,
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
        //text::clean_stream_textarea_with_context(context.clone(), "stream_textarea").await;
        tracing::info!(target: "tron_app", "event: {:?}", event);
        match event.e_type.as_str() {
            "click" => {
                if let Some(target) = event.h_target {
                    context
                        .set_value_for_component(&target, TnComponentValue::VecString(vec![]))
                        .await;
                    let html = "".to_string();
                    let mut header = HeaderMap::new();
                    header.insert(
                        HeaderName::from_str("hx-reswap").unwrap(),
                        HeaderValue::from_str("innerHTML").unwrap(),
                    );
                    Some((header, Html::from(html)))
                } else {
                    None
                }
            }
            "clean_stream_textarea" => {
                if let Some(target) = event.h_target {
                    let html = context.render_component(&target).await;
                    Some((HeaderMap::new(), Html::from(html)))
                } else {
                    None
                }
            }
            _ => None,
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
        text::clean_textarea_with_context(context.clone(), "textarea").await;
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
        let stream_textarea = context.get_component("stream_textarea").await;
        let slider = context.get_component(&event.e_trigger).await;
        if let TnComponentValue::String(s) = slider.read().await.value() {
            let new_str = format!("{} -- Value {};\n", event.e_trigger, s);
            append_stream_textarea(stream_textarea, &new_str).await;
            let msg =
        r##"{"server_side_trigger_data": { "target":"stream_textarea", "new_state":"ready" } }"##
            .to_string();
            let sse_tx = context.get_sse_tx().await;
            if sse_tx.send(msg).await.is_err() {
                debug!("tx dropped");
            }
        }
        let html: String = context.render_component("slider").await;
        Some((HeaderMap::new(), Html::from(html)))
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
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
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
    };
    html.render().unwrap()
}
