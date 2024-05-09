use super::*;
use futures_util::Future;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnRadioGroup<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnRadioGroup<'a> {
    pub fn new(
        id: TnComponentIndex,
        name: String,
        value: String,
        radio_group_items: Vec<(String, String)>,
    ) -> Self {
        let mut base = TnComponentBase::new("div".into(), id, name, TnComponentType::RadioGroup);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute("type".into(), "radio_group".into());
        let mut asset = HashMap::default();
        asset.insert(
            "radio_group_items".into(),
            TnAsset::VecString2(radio_group_items),
        );
        base.asset = Some(asset);
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
            r##"<div id="{tron_id}-container" {container_attributes}><{} {} type="radio" value="{tron_id}" name="{parent_tron_id}" {checked} /><label for="{tron_id}">&nbsp;{label}</label></div>"##,
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
    radio_group_items: Vec<(String, String)>,
    container_attributes: Vec<(String, String)>,
    default_item: String,
) {
    let children_ids = radio_group_items
        .iter()
        .map(|(child_trod_id, label)| {
            *component_index += 1;
            let radio_item_index = *component_index;
            let is_default_item = *child_trod_id == default_item;
            let mut radio_item =
                TnRadioItem::new(radio_item_index, child_trod_id.clone(), is_default_item);

            let asset = radio_item.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            asset.insert("label".into(), TnAsset::String(label.clone()));
            context.add_component(radio_item);
            context
                .tnid_to_index
                .insert(format!("{}-container", child_trod_id), radio_item_index);

            radio_item_index
        })
        .collect::<Vec<_>>();

    *component_index += 1;
    let radio_group = TnRadioGroup::new(
        *component_index,
        radio_group_tron_id,
        default_item,
        radio_group_items,
    );
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

pub fn get_radio_group_actions(comp: TnComponent<'static>) -> Vec<(TnComponentId, TnActionFn)> {
    let comp_guard = comp.blocking_write();
    assert!(comp_guard.get_type() == TnComponentType::RadioGroup);
    let children = comp_guard.get_children().clone();
    let mut events: Vec<(TnComponentId, TnActionFn)> = Vec::default();
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
            let mut parent = parent_guard.write().await;
            parent.set_value(TnComponentValue::String(event.e_trigger.clone()));
            if let TnAsset::VecString2(items) = parent
                .get_assets()
                .unwrap()
                .get("radio_group_items")
                .unwrap()
            {
                items.clone()
            } else {
                vec![]
            }
        };

        for k in keys {
            if *k.0 == event.e_trigger {
                context
                    .set_value_for_component(&k.0, TnComponentValue::RadioItem(true))
                    .await;
            } else {
                context
                    .set_value_for_component(&k.0, TnComponentValue::RadioItem(false))
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
