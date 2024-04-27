use askama::Template;
use futures_util::Future;

//use serde::{Deserialize, Serialize};
use tokio::sync::RwLock;

use serde_json::Value;

use tracing::debug;
use tron_components::{
    checklist, text::TnTextInput, ActionExecutionMethod, ComponentBaseTrait, ComponentState,
    ComponentValue, Context, TnButton, TnEvent, TnEventActions, TnSelect, TnTextArea,
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
    let mut context = Context::<'static>::default();
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

        context.add_component(btn);
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

    context.add_component(textarea);

    component_id += 1;

    let checklist_items = vec![
        "checkbox-1".to_string(),
        "checkbox-2".to_string(),
        "checkbox-3".to_string(),
        "checkbox-4".to_string(),
        "checkbox-5".to_string(),
        "checkbox-6".to_string(),
    ];
    let checklist_tron_id = "checklist".to_string();
    let container_attributes = vec![("class".to_string(),"flex-1".to_string())];
    checklist::add_checklist_to_context(
        &mut context,
        &mut component_id,
        checklist_tron_id,
        checklist_items,
        container_attributes
    );
    {
        let component_guard = context.components.blocking_read();
        let checklist_guard = component_guard.get(&component_id).unwrap();
        checklist_guard
            .blocking_write()
            .set_attribute("class".into(), "flex flex-row p-1 flex-1".into());
    }
    component_id += 1;
    let select_options = vec![
        ("one".into(), "One".into()),
        ("two".into(), "Two".into()),
        ("three".into(), "Three".into()),
    ];

    let select = TnSelect::<'static>::new(
        component_id,
        "select_one".into(),
        "one".into(),
        select_options,
    );
    context.add_component(select);

    component_id += 1;
    let mut textinput = TnTextInput::<'static>::new(component_id, "textinput".into(), "".into());
    textinput.set_attribute("class".into(), "input input-bordered w-full".into());

    context.add_component(textinput);

    Arc::new(RwLock::new(context))
}

fn build_session_actions(context: Arc<RwLock<Context<'static>>>) -> TnEventActions {
    let mut actions = TnEventActions::default();
    for i in 0..10 {
        let evt = TnEvent {
            e_target: format!("btn-{:02}", i),
            e_type: "click".to_string(),
            e_state: "ready".to_string(),
        };
        actions.insert(evt, (ActionExecutionMethod::Spawn, Arc::new(test_evt_task)));
    }
    {
        let context_guard = context.blocking_read();
        let checklist_id = context_guard.get_component_id("checklist");
        let component_guard = context_guard.components.blocking_read();
        let checklist = component_guard.get(&checklist_id).unwrap().clone();
        let checklist_actions = checklist::get_checklist_actions(checklist);
        checklist_actions.into_iter().for_each(|(evt, action)| {
            actions.insert(evt, (ActionExecutionMethod::Await, action));
        });
    }

    actions
}

#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    buttons: Vec<String>,
    textarea: String,
    textinput: String,
    checklist: String,
    select: String,
}

fn layout(context: Arc<RwLock<Context<'static>>>) -> String {
    let context_guard = context.blocking_read();
    let buttons = (0..10)
        .map(|i| context_guard.render_to_string(&format!("btn-{:02}", i)))
        .collect::<Vec<String>>();

    let context_guard = context.blocking_read();
    let textarea = context_guard.render_to_string("textarea");
    let textinput = context_guard.render_to_string("textinput");
    let checklist = context_guard.render_to_string("checklist");
    let select = context_guard.render_to_string("select_one");

    let html = AppPageTemplate {
        buttons,
        textarea,
        textinput,
        checklist,
        select,
    };
    html.render().unwrap()
}
