use super::*;
use futures_util::Future;
use tron_macro::*;

/// Represents a checklist component.
#[derive(ComponentBase)]
pub struct TnCheckList<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnCheckList<'static> {
    /// Creates a new checklist component with the specified ID, name, and values.
    pub fn new(id: TnComponentIndex, name: String, value: HashMap<String, bool>) -> Self {
        let mut base = TnComponentBase::new("div".into(), id, name, TnComponentType::CheckList);
        base.set_value(TnComponentValue::CheckItems(value));
        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "checklist".into());
        base.script = Some(include_str!("../javascript/checklist.html").to_string());
        Self { base }
    }
}

impl Default for TnCheckList<'static> {
    /// Creates a default checklist component with an empty value.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl TnCheckList<'static> {
    /// Renders the checklist component including its children.
    pub fn internal_render(&self) -> String {
        let children_render_results = self
            .get_children()
            .iter()
            .map(|c: &Arc<RwLock<Box<dyn TnComponentBaseTrait<'static>>>>| c.blocking_read().render())
            .collect::<Vec<String>>()
            .join(" ");
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            children_render_results,
            self.base.tag
        )
    }

    /// Renders the checklist component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
/// Represents a checkbox component.
#[derive(ComponentBase)]
pub struct TnCheckBox<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnCheckBox<'static> {
    /// Creates a new checkbox component.
    pub fn new(id: TnComponentIndex, name: String, value: bool) -> Self {
        let mut base =
            TnComponentBase::new("input".into(), id, name.clone(), TnComponentType::CheckBox);
        base.set_value(TnComponentValue::CheckItem(value));
        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("hx-target".into(), format!("#{}-container", name));
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data: get_checkbox_event(event)}"##.into(),
        );
        base.set_attribute("hx-swap".into(), "none".into());
        //component_base.set_attribute("type".into(), "checkbox".into());
        base.asset = Some(HashMap::default());
        Self { base }
    }
}

impl Default for TnCheckBox<'static> {
    /// Creates a default checkbox component.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl TnCheckBox<'static> {
    /// Renders the checkbox component internally.
    pub fn internal_render(&self) -> String {
        let checked = if let &TnComponentValue::CheckItem(v) = self.value() {
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
        let label = if assets.contains_key("label") {
            if let TnAsset::String(label) = assets.get("label").unwrap() {
                label.clone()
            } else {
                tron_id.clone()
            }
        } else {
            tron_id.clone()
        };
        format!(
            r##"<div id="{tron_id}-container" {container_attributes}><{} {} type="checkbox" value="{tron_id}" name="{parent_tron_id}" {checked} /><label for="{tron_id}">&nbsp;{label}</label></div>"##,
            self.base.tag,
            self.generate_attr_string(),
        )
    }
    /// Renders the first instance of the checkbox component.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Adds a checklist component to the context along with its child checkboxes.
pub fn add_checklist_to_context(
    context: &mut TnContextBase<'static>,
    component_index: &mut u32,
    checklist_tron_id: String,
    checklist_items: Vec<(String, String)>,
    container_attributes: Vec<(String, String)>,
) {
    let children_ids = checklist_items
        .into_iter()
        .map(|(child_trod_id, label)| {
            *component_index += 1;
            let checkbox_index = *component_index;
            let mut checkbox = TnCheckBox::new(checkbox_index, child_trod_id.clone(), false);
            let asset = checkbox.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            asset.insert("label".into(), TnAsset::String(label));
            context.add_component(checkbox);
            context
                .tnid_to_index
                .insert(format!("{child_trod_id}-container"), checkbox_index);
            checkbox_index
        })
        .collect::<Vec<_>>();

    *component_index += 1;
    let checklist = TnCheckList::new(*component_index, checklist_tron_id, HashMap::default());
    context.add_component(checklist);
    let components = context.components.blocking_read();
    let checklist = components.get(component_index).unwrap();
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

/// Updates the value of the checklist component based on its child checkboxes.
pub async fn checklist_update_value(comp: TnComponent<'static>) {
    let mut comp_guard = comp.write().await;
    assert!(comp_guard.get_type() == TnComponentType::CheckList);
    let children = comp_guard.get_children().clone();
    if let TnComponentValue::CheckItems(ref mut value) = comp_guard.get_mut_value() {
        value.clear();
    };
    for child in children {
        let child = child.read().await;
        if let TnComponentValue::CheckItems(ref mut value) = comp_guard.get_mut_value() {
            if let TnComponentValue::CheckItem(b) = child.value() {
                value.insert(child.tron_id().clone(), *b);
            }
        }
    }
}

/// Retrieves the actions associated with the checkboxes within the checklist component.
pub fn get_checklist_actions(comp: TnComponent<'static>) -> Vec<(TnComponentId, TnActionFn)> {
    let comp_guard = comp.blocking_write();
    assert!(comp_guard.get_type() == TnComponentType::CheckList);
    let children = comp_guard.get_children().clone();
    let mut events: Vec<(TnComponentId, TnActionFn)> = Vec::default();
    for child in children {
        let child = child.blocking_read();

        events.push((child.tron_id().clone(), toggle_checkbox))
    }
    events
}

/// Toggles the state of the checkbox based on the event payload.
pub fn toggle_checkbox(
    context: TnContext,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        // println!("paylod value {payload}");
        if let Value::String(checked) = &payload["event_data"]["e_value"] {
            let context_guard = context.read().await;
            let checkbox_id = context_guard.get_component_index(&event.e_trigger);
            let components_guard = context_guard.components.write().await;
            let mut checkbox = components_guard.get(&checkbox_id).unwrap().write().await;

            if checked.as_str() == "true" {
                // println!("set true");
                checkbox.set_value(TnComponentValue::CheckItem(true));
                let parent_guard = checkbox.get_parent().clone();
                let mut parent_guard = parent_guard.write().await;
                if let TnComponentValue::CheckItems(ref mut value) = parent_guard.get_mut_value() {
                    value.insert(event.e_trigger.clone(), true);
                };
            } else {
                // println!("set false");
                checkbox.set_value(TnComponentValue::CheckItem(false));
                let parent_guard = checkbox.get_parent().clone();
                let mut parent_guard = parent_guard.write().await;
                if let TnComponentValue::CheckItems(ref mut value) = parent_guard.get_mut_value() {
                    value.insert(event.e_trigger.clone(), false);
                };
            }
            checkbox.set_state(TnComponentState::Ready);
            let checkbox_html = tokio::task::block_in_place(|| checkbox.render());
            Some((HeaderMap::new(), Html::from(checkbox_html)))
        } else {
            None
        }
    };

    Box::pin(f)
}
