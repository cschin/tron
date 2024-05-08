use super::*;
use futures_util::Future;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnRadioGroup<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnRadioGroup<'a> {
    pub fn new(id: TnComponentIndex, name: String, value: HashMap<String, bool>) -> Self {
        let mut base = TnComponentBase::new("div".into(), id, name, TnComponentType::RadioGroup);
        base.set_value(TnComponentValue::RadioItems(value));
        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "radio_group".into());
        base.script = Some(include_str!("../javascript/radio_group.html").to_string());
        Self { base }
    }
}

impl<'a: 'static> Default for TnRadioGroup<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnRadioGroup<'a> {
    pub fn internal_render(&self) -> String {
        let children_render_results = self
            .get_children()
            .iter()
            .map(|c: &Arc<RwLock<Box<dyn TnComponentBaseTrait<'a>>>>| c.blocking_read().render())
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

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

#[derive(ComponentBase)]
pub struct TnRadioItem<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnRadioItem<'a> {
    pub fn new(id: TnComponentIndex, name: String, value: bool) -> Self {
        let mut base =
            TnComponentBase::new("input".into(), id, name.clone(), TnComponentType::RadioItem);
        base.set_value(TnComponentValue::RadioItem(value));
        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("hx-target".into(), format!("#{}-container", name));
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data: get_radio_group_event(event)}"##.into(),
        );
        base.set_attribute("hx-swap".into(), "none".into());
        //component_base.set_attribute("type".into(), "checkbox".into());
        base.asset = Some(HashMap::default());
        Self { base }
    }
}

impl<'a: 'static> Default for TnRadioItem<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnRadioItem<'a> {
    pub fn internal_render(&self) -> String {
        let checked = if let &TnComponentValue::RadioItem(v) = self.value() {
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
            r##"<div id="{tron_id}-container" {container_attributes}><{} {} type="radio" value="{tron_id}" name="{parent_tron_id}" {checked} /><label for="{tron_id}">&nbsp;{tron_id}</label></div>"##,
            self.base.tag,
            self.generate_attr_string(),
        )
    }
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

pub fn add_radio_group_to_context(
    context: &mut TnContextBase<'static>,
    component_index: &mut u32,
    radio_group_tron_id: String,
    radio_group_items: Vec<String>,
    container_attributes: Vec<(String, String)>,
    defult_item: String,
) {
    let mut parent_value = HashMap::<String, bool>::default();
    let children_ids = radio_group_items
        .into_iter()
        .map(|child_trod_id| {
            *component_index += 1;
            let radio_item_index = *component_index;
            let is_default_item = child_trod_id == defult_item;
            let mut radio_item =
                TnRadioItem::new(radio_item_index, child_trod_id.clone(), is_default_item);
            parent_value.insert(child_trod_id.clone(), is_default_item);

            let asset = radio_item.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            context.add_component(radio_item);
            context
                .tnid_to_index
                .insert(format!("{child_trod_id}-container"), radio_item_index);

            radio_item_index
        })
        .collect::<Vec<_>>();

    *component_index += 1;
    let radio_group = TnRadioGroup::new(*component_index, radio_group_tron_id, parent_value);
    context.add_component(radio_group);
    let components = context.components.blocking_read();
    let radio_group = components.get(component_index).unwrap();
    children_ids.iter().for_each(|child_id| {
        {
            let mut radio_group = radio_group.blocking_write();
            radio_group.add_child(
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
            child.add_parent(radio_group.clone());
        }
    });
}

pub async fn radio_group_update_value(comp: TnComponent<'static>) {
    let mut comp_guard = comp.write().await;
    assert!(comp_guard.get_type() == TnComponentType::RadioGroup);
    let children = comp_guard.get_children().clone();
    if let TnComponentValue::RadioItems(ref mut value) = comp_guard.get_mut_value() {
        value.clear();
    };
    for child in children {
        let child = child.read().await;
        if let TnComponentValue::RadioItems(ref mut value) = comp_guard.get_mut_value() {
            if let TnComponentValue::RadioItem(b) = child.value() {
                value.insert(child.tron_id().clone(), *b);
            }
        }
    }
}

pub fn get_radio_group_actions(comp: TnComponent<'static>) -> Vec<(TnComponentId, ActionFn)> {
    let comp_guard = comp.blocking_write();
    assert!(comp_guard.get_type() == TnComponentType::RadioGroup);
    let children = comp_guard.get_children().clone();
    let mut events: Vec<(TnComponentId, ActionFn)> = Vec::default();
    for child in children {
        let child = child.blocking_read();
        events.push((child.tron_id().clone(), set_radio_item))
    }
    events
}

pub fn set_radio_item(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        let keys = {
            let radio_item_guard = context.get_component(&event.e_trigger).await;
            let radio_item = radio_item_guard.write().await;
            let parent_guard = radio_item.get_parent().clone();
            let mut parent_guard = parent_guard.write().await;
            let keys =
                if let TnComponentValue::RadioItems(ref mut value) = parent_guard.get_mut_value() {
                    let keys = value.keys().cloned().collect::<Vec<String>>();
                    for k in keys.clone() {
                        if k == event.e_trigger {
                            value.insert(k, true);
                        } else {
                            value.insert(k, false);
                        }
                    }
                    keys
                } else {
                    vec![]
                };
            keys
        };

        for k in keys {
            if k == event.e_trigger {
                context
                    .set_value_for_component(&k, TnComponentValue::RadioItem(true))
                    .await;
            } else {
                context
                    .set_value_for_component(&k, TnComponentValue::RadioItem(false))
                    .await;
            }
        }

        {
            let radio_item_guard = context.get_component(&event.e_trigger).await;
            let mut radio_item = radio_item_guard.write().await;
            radio_item.set_state(TnComponentState::Ready);
            let radio_item_html = tokio::task::block_in_place(|| radio_item.render());
            Some((HeaderMap::new(), Html::from(radio_item_html)))
        }
    };

    Box::pin(f)
}
