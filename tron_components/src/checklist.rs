use super::*;
use futures_util::Future;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnCheckList<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnCheckList<'a> {
    pub fn new(id: ComponentId, name: String, value: HashMap<String, bool>) -> Self {
        let mut component_base =
            ComponentBase::new("div".into(), id, name, TnComponentType::CheckList);
        component_base.set_value(ComponentValue::CheckItems(value));
        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute("type".into(), "checklist".into());
        component_base.script = Some(include_str!("../javascript/checklist.html").to_string());
        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnCheckList<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnCheckList<'a> {
    pub fn internal_render(&self) -> String {
        let childred_render_results = self
            .get_children()
            .iter()
            .map(|c: &Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>>| c.blocking_read().render())
            .collect::<Vec<String>>()
            .join(" ");
        format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            childred_render_results,
            self.inner.tag
        )
    }
}

#[derive(ComponentBase)]
pub struct TnCheckBox<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnCheckBox<'a> {
    pub fn new(id: ComponentId, name: String, value: bool) -> Self {
        let mut component_base =
            ComponentBase::new("input".into(), id, name.clone(), TnComponentType::CheckBox);
        component_base.set_value(ComponentValue::CheckItem(value));
        component_base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        component_base.set_attribute("hx-target".into(), format!("#{}-container", name));
        component_base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data: get_checkbox_event(event)}"##.into(),
        );
        component_base.set_attribute("hx-swap".into(), "none".into());
        //component_base.set_attribute("type".into(), "checkbox".into());
        component_base.assets = Some(HashMap::default());
        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnCheckBox<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnCheckBox<'a> {
    pub fn internal_render(&self) -> String {
        let checked = if let &ComponentValue::CheckItem(v) = self.value() {
            if v {
                "checked"
            } else {
                ""
            }
        } else {
            ""
        };
        let tron_id = self.tron_id();
        let parent_guard = self.get_parent().clone();
        let parent_guard = parent_guard.blocking_read();
        let parent_tron_id = parent_guard.tron_id().clone();
        let assets = self.get_assets().unwrap();
        let container_attributes = if assets.contains_key("container_attributes") {
            if let TnAsset::VecString2(container_attributes) =
                assets.get("container_attributes").unwrap()
            {
                container_attributes
                    .iter()
                    .map(|(k, v)| format!(r#"{}={}"#, k, v))
                    .collect::<Vec<String>>()
                    .join(" ")
            } else {
                "".to_string()
            }
        } else {
            "".to_string()
        };
        format!(
            r##"<div id="{tron_id}-container" {container_attributes}><{} {} type="checkbox" value="{tron_id}" name="{parent_tron_id}" {checked} /><label for="{tron_id}">&nbsp;{tron_id}</label></div>"##,
            self.inner.tag,
            self.generate_attr_string(),
        )
    }
}

pub fn add_checklist_to_context(
    context: &mut Context<'static>,
    component_id: &mut u32,
    checklist_tron_id: String,
    checklist_items: Vec<String>,
    container_attributes: Vec<(String, String)>,
) {
    let children_ids = checklist_items
        .into_iter()
        .map(|child_trod_id| {
            *component_id += 1;
            let checkbox_id = *component_id;
            let mut checkbox = TnCheckBox::new(checkbox_id, child_trod_id, false);
            let asset = checkbox.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            context.add_component(checkbox);
            checkbox_id
        })
        .collect::<Vec<_>>();

    *component_id += 1;
    let checklist = TnCheckList::new(*component_id, checklist_tron_id, HashMap::default());
    context.add_component(checklist);
    let components = context.components.blocking_read();
    let checklist = components.get(component_id).unwrap();
    children_ids.iter().for_each(|child_id| {
        {
            let mut checklist = checklist.blocking_write();
            checklist.add_child(
                // we need to get Arc from the context
                context
                    .components
                    .blocking_read()
                    .get(child_id)
                    .unwrap()
                    .clone(),
            );
        }
        {
            let components = context.components.blocking_read();
            let mut child = components.get(child_id).unwrap().blocking_write();
            child.add_parent(checklist.clone());
        }
    });
}

pub async fn checklist_update_value(comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>) {
    let mut comp_guard = comp.write().await;
    assert!(comp_guard.get_type() == TnComponentType::CheckList);
    let children = comp_guard.get_children().clone();
    if let ComponentValue::CheckItems(ref mut value) = comp_guard.get_mut_value() {
        value.clear();
    };
    for child in children {
        let child = child.read().await;
        if let ComponentValue::CheckItems(ref mut value) = comp_guard.get_mut_value() {
            if let ComponentValue::CheckItem(b) = child.value() {
                value.insert(child.tron_id().clone(), *b);
            }
        }
    }
}

pub fn get_checklist_actions(
    comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>,
) -> Vec<(TnEvent, Arc<ActionFn>)> {
    let comp_guard = comp.blocking_write();
    assert!(comp_guard.get_type() == TnComponentType::CheckList);
    let children = comp_guard.get_children().clone();
    let mut events: Vec<(TnEvent, Arc<ActionFn>)> = Vec::default();
    for child in children {
        let child = child.blocking_read();
        let evt = TnEvent {
            e_target: child.tron_id().clone(),
            e_type: "change".into(),
            e_state: "ready".into(),
        };
        events.push((evt, Arc::new(toggle_checkbox)))
    }
    events
}

pub fn toggle_checkbox(
    context: Arc<RwLock<Context<'static>>>,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        // println!("paylod value {payload}");
        if let Value::String(checked) = &payload["event_data"]["e_value"] {
            let context_guard = context.read().await;
            let checkbox_id = context_guard.get_component_id(&event.e_target);
            let components_guard = context_guard.components.write().await;
            let mut checkbox = components_guard.get(&checkbox_id).unwrap().write().await;

            if checked == &"true".to_string() {
                // println!("set true");
                checkbox.set_value(ComponentValue::CheckItem(true));
                let parent_guard = checkbox.get_parent().clone();
                let mut parent_guard = parent_guard.write().await;
                if let ComponentValue::CheckItems(ref mut value) = parent_guard.get_mut_value() {
                    value.insert(event.e_target.clone(), true);
                };
            } else {
                // println!("set false");
                checkbox.set_value(ComponentValue::CheckItem(false));
                let parent_guard = checkbox.get_parent().clone();
                let mut parent_guard = parent_guard.write().await;
                if let ComponentValue::CheckItems(ref mut value) = parent_guard.get_mut_value() {
                    value.insert(event.e_target.clone(), false);
                };
            }
            checkbox.set_state(ComponentState::Ready);
        };
    };

    Box::pin(f)
}
