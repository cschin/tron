use super::*;
use futures_util::Future;
use tron_macro::*;

/// Represents a checklist component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnCheckList<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnCheckListBuilder<'static> {
    /// Creates a new checklist component with the specified ID, name, and values.
    pub fn init(mut self, name: String, value: HashMap<String, bool>) -> Self {
        let component_type = TnComponentType::CheckList;
        TnComponentType::register_script(
            component_type.clone(),
            include_str!("../javascript/checklist.html"),
        );
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), name, component_type)
            .set_value(TnComponentValue::CheckItems(value))
            .set_attribute("hx-trigger".into(), "server_side_trigger".into())
            .set_attribute("type".into(), "checklist".into())
            .build();
        self
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
            .map(|c: &Arc<RwLock<Box<dyn TnComponentBaseTrait<'static>>>>| {
                c.blocking_read().render()
            })
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
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnCheckBox<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnCheckBoxBuilder<'static> {
    /// Creates a new checkbox component.
    pub fn init(mut self, name: String, value: bool) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("input".into(), name.clone(), TnComponentType::CheckBox)
            .set_value(TnComponentValue::Bool(value))
            .set_attribute("hx-trigger".into(), "change, server_side_trigger".into())
            .set_attribute("hx-target".into(), format!("#{}-container", name))
            .set_attribute(
                "hx-vals".into(),
                r##"js:{event_data: get_checkbox_event(event)}"##.into(),
            )
            .set_attribute("hx-swap".into(), "none".into())
            .create_assets()
            .set_action(TnActionExecutionMethod::Await, toggle_checkbox)
            .build();
        self
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
        let checked = if let &TnComponentValue::Bool(v) = self.value() {
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
    checklist_tron_id: &str,
    checklist_items: Vec<(String, String)>,
    container_attributes: Vec<(String, String)>,
) {
    let children_ids = checklist_items
        .into_iter()
        .map(|(child_tron_id, label)| {
            let mut checkbox = TnCheckBox::builder()
                .init(child_tron_id.clone(), false)
                .build();
            let asset = checkbox.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            asset.insert("label".into(), TnAsset::String(label));
            context.add_component(checkbox);
            child_tron_id
        })
        .collect::<Vec<_>>();

    let checklist = TnCheckList::builder()
        .init(checklist_tron_id.to_string(), HashMap::default())
        .build();
    context.add_component(checklist);
    let components = context.components.blocking_read();
    let checklist = components.get(&checklist_tron_id.to_string()).unwrap();
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
            if let TnComponentValue::Bool(b) = child.value() {
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
            let components_guard = context_guard.components.write().await;
            let mut checkbox = components_guard
                .get(&event.e_trigger)
                .unwrap()
                .write()
                .await;

            if checked.as_str() == "true" {
                // println!("set true");
                checkbox.set_value(TnComponentValue::Bool(true));
                let parent_guard = checkbox.get_parent().clone();
                let mut parent_guard = parent_guard.write().await;
                if let TnComponentValue::CheckItems(ref mut value) = parent_guard.get_mut_value() {
                    value.insert(event.e_trigger.clone(), true);
                };
            } else {
                // println!("set false");
                checkbox.set_value(TnComponentValue::Bool(false));
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
