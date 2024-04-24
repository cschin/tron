use askama::Template;
use futures_util::Future;

//use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use serde_json::Value;

use tracing::debug;
use tron_components::{
    text::TnTextInput, ActionExecutionMethod, ComponentBaseTrait, ComponentState, ComponentValue,
    Context, TnButton, TnEvent, TnEventActions, TnTextArea,
};
//use std::sync::Mutex;
use std::{collections::HashMap, pin::Pin, sync::Arc};

#[tokio::main]
async fn main() {
    // set app state
    let app_share_data = tron_app::AppData {
        session_context: RwLock::new(HashMap::default()),
        event_actions: RwLock::new(TnEventActions::default()),
        build_session_context: Arc::new(Box::new(build_session_context)),
        build_session_actions: Arc::new(Box::new(build_session_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data).await
}

fn test_evt_task(
    context: Arc<RwLock<Context<'static>>>,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = || async move {
        let mut interval = tokio::time::interval(tokio::time::Duration::from_millis(200));
        let mut i = 0;
        let sse_tx = {
            let context_guard = context.read().await;
            let channel_guard = context_guard.sse_channels.read().await;
            channel_guard.as_ref().unwrap().tx.clone()
        };

        debug!("Event: {:?}", event.clone());
        loop {
            {
                let context_guard = context.write().await;
                let v;
                {
                    let id = context_guard.get_component_id(&event.e_target.clone());
                    let mut components_guard = context_guard.components.write().await;
                    {
                        let btn = components_guard.get_mut(&id).unwrap().read().await;
                        v = match btn.value() {
                            ComponentValue::String(s) => s.parse::<u32>().unwrap(),
                            _ => 0,
                        };
                    }
                    {
                        let mut btn = components_guard.get_mut(&id).unwrap().write().await;

                        btn.set_value(ComponentValue::String(format!("{:02}", v + 1)));
                        btn.set_state(ComponentState::Updating);
                    }
                }

                {
                    let id = context_guard.get_component_id("textarea");
                    let mut components_guard = context_guard.components.write().await;
                    let mut text_area = components_guard.get_mut(&id).unwrap().write().await;
                    let origin_string =
                        if let ComponentValue::String(origin_string) = text_area.value() {
                            origin_string.clone()
                        } else {
                            "".to_string()
                        };
                    text_area.set_value(ComponentValue::String(format!(
                        "{}\n {} -- {:02};",
                        origin_string,
                        event.e_target,
                        v + 1
                    )));
                };
            }

            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"updating" }} }}"##,
                event.e_target
            );
            if sse_tx.send(data).await.is_err() {
                debug!("tx dropped");
            }

            let data =
                r##"{"server_side_trigger": { "target":"textarea", "new_state":"ready" } }"##
                    .to_string();
            if sse_tx.send(data).await.is_err() {
                debug!("tx dropped");
            }

            i += 1;
            interval.tick().await;
            debug!("loop triggered: {} {}", event.e_target, i);

            if i > 9 {
                break;
            };
        }
        {
            let context_guard = context.write().await;
            let id = context_guard.get_component_id(&event.e_target.clone());
            let mut components_guard = context_guard.components.write().await;
            let mut btn = components_guard.get_mut(&id).unwrap().write().await;
            btn.set_state(ComponentState::Ready);
            let data = format!(
                r##"{{"server_side_trigger": {{ "target":"{}", "new_state":"{}" }} }}"##,
                event.e_target, "ready"
            );
            if sse_tx.send(data).await.is_err() {
                println!("tx dropped");
            }
        }
    };
    Box::pin(f())
}

fn build_session_context() -> Arc<RwLock<Context<'static>>> {
    let mut components = Context::<'static>::default();
    let mut component_id = 0_u32;
    loop {
        let mut btn = TnButton::<'static>::new(
            component_id,
            format!("btn-{:02}", component_id),
            format!("{:02}", component_id),
        );
        btn.set_attribute(
            "class".to_string(),
            "btn btn-sm btn-outline btn-primary flex-1".to_string(),
        );
        components.add_component(btn);
        component_id += 1;
        if component_id >= 10 {
            break;
        }
    }

    let mut textarea = TnTextArea::<'static>::new(component_id, "textarea".into(), "".into());
    textarea.set_attribute(
        "class".into(),
        "textarea textarea-bordered flex-1 min-h-80v".into(),
    );
    textarea.set_attribute(
        "hx-swap".into(),
        "outerHTML scroll:bottom focus-scroll:true".into(),
    );
    components.add_component(textarea);

    component_id += 1;

    let mut textinput = TnTextInput::<'static>::new(component_id, "textinput".into(), "10".into());
    textinput.set_attribute("class".into(), "input w-full max-w-xs".into());
    textinput.set_attribute(
        "hx-swap".into(),
        "outerHTML scroll:bottom focus-scroll:true".into(),
    );
    components.add_component(textinput);

    Arc::new(RwLock::new(components))

    //Arc::new(RwLock::new(components))
}

fn build_session_actions() -> TnEventActions {
    let mut actions = TnEventActions::default();
    for i in 0..10 {
        let evt = TnEvent {
            e_target: format!("btn-{:02}", i),
            e_type: "click".to_string(),
            e_state: "ready".to_string(),
        };
        actions.insert(evt, (ActionExecutionMethod::Spawn, Arc::new(test_evt_task)));
    }
    actions
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    buttons: Vec<String>,
    textarea: String,
    textinput: String,
}

fn layout(context: Arc<RwLock<Context<'static>>>) -> String {
    let context_guard = context.blocking_read();
    let mut components_guard = context_guard.components.blocking_write();

    let buttons = (0..10)
        .map(|i| {
            let btn = components_guard.get_mut(&i).unwrap().blocking_read();
            btn.render()
        })
        .collect::<Vec<String>>();

    let textarea = {
        let id = context_guard.get_component_id("textarea");
        let textarea = components_guard.get_mut(&id).unwrap().blocking_read();
        textarea.render()
    };

    let textinput = {
        let id = context_guard.get_component_id("textinput");
        let textinput = components_guard.get_mut(&id).unwrap().blocking_read();
        textinput.render()
    };

    let html = AppPageTemplate {
        buttons,
        textarea,
        textinput,
    };
    html.render().unwrap()
}
