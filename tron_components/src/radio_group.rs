use super::*;
use futures_util::Future;
use tron_macro::*;

/// Defines a radio group component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnRadioGroup<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnRadioGroupBuilder<'static> {
    /// Creates a new instance of `TnRadioGroup`.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the component.
    /// * `tnid` - The name of the radio group.
    /// * `value` - The default value for the radio group.
    /// * `radio_group_items` - A vector of tuples containing the IDs and labels of the radio items in the group.
    ///
    /// # Returns
    ///
    /// A new instance of `TnRadioGroup`.
    pub fn init(
        mut self,
        tnid: String,
        value: String,
        radio_group_items: Vec<(String, String)>,
    ) -> Self {
        let component_type = TnComponentType::RadioGroup;
        TnComponentType::register_script(
            component_type.clone(),
            include_str!("../javascript/radio_group.html"),
        );
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), tnid, component_type)
            .set_value(TnComponentValue::String(value))
            .set_attribute("hx-trigger".into(), "server_side_trigger".into())
            .set_attribute("type".into(), "radio_group".into())
            .create_assets()
            .build();
        self.base.asset.as_mut().unwrap().insert(
            "radio_group_items".into(),
            TnAsset::VecString2(radio_group_items),
        );
        self
    }
}

/// Implements the default trait for `TnRadioGroup`.
impl<'a: 'static> Default for TnRadioGroup<'a> {
    /// Creates a default instance of `TnRadioGroup`.
    ///
    /// # Returns
    ///
    /// A default instance of `TnRadioGroup`.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

/// Implements methods for rendering `TnRadioGroup`.
impl TnRadioGroup<'static> {
    /// Renders the internal structure of the `TnRadioGroup`.
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

    /// Renders the internal structure of the `TnRadioGroup` for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Represents a radio item component within a radio group.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnRadioItem<'a: 'static> {
    base: TnComponentBase<'a>,
}

/// Creates a new radio item component.
///
/// # Arguments
///
/// * `idx` - The unique identifier of the radio item.
/// * `tnid` - The name of the radio item.
/// * `value` - The initial value of the radio item.
///
/// # Returns
///
/// A new `TnRadioItem` instance.
impl TnRadioItemBuilder<'static> {
    pub fn init(mut self, tnid: TnComponentId, value: bool) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("input".into(), tnid.clone(), TnComponentType::RadioItem)
            .set_value(TnComponentValue::RadioItem(value))
            .set_attribute("hx-trigger".into(), "change, server_side_trigger".into())
            .set_attribute("hx-target".into(), format!("#{}-container", tnid))
            .set_attribute(
                "hx-vals".into(),
                r##"js:{event_data: get_radio_group_event(event)}"##.into(),
            )
            .set_attribute("hx-swap".into(), "none".into())
            .set_action(TnActionExecutionMethod::Await, set_radio_item)
            .create_assets()
            .build();
        //component_self.base.set_attribute("type".into(), "checkbox".into());
        self
    }
}

/// Implements the default trait for `TnRadioItem`.
impl Default for TnRadioItem<'static> {
    /// Creates a default `TnRadioItem` instance.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl TnRadioItem<'static> {
    /// Renders the `TnRadioItem` component into HTML.
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
    /// Renders the first instance of the `TnRadioItem` component into HTML.

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

pub fn add_radio_group_to_context(
    context: &mut TnContextBase<'static>,
    radio_group_tron_id: &str,
    radio_group_items: Vec<(String, String)>,
    container_attributes: Vec<(String, String)>,
    default_item: String,
) {
    let children_ids = radio_group_items
        .iter()
        .map(|(child_tron_id, label)| {
            let is_default_item = *child_tron_id == default_item;
            let mut radio_item = TnRadioItem::builder()
                .init(child_tron_id.clone(), is_default_item)
                .build();

            let asset = radio_item.get_mut_assets().unwrap();
            asset.insert(
                "container_attributes".into(),
                TnAsset::VecString2(container_attributes.clone()),
            );
            asset.insert("label".into(), TnAsset::String(label.clone()));
            context.add_component(radio_item);

            child_tron_id.clone()
        })
        .collect::<Vec<_>>();

    let radio_group = TnRadioGroup::builder()
        .init(
            radio_group_tron_id.to_string(),
            default_item,
            radio_group_items,
        )
        .build();
    context.add_component(radio_group);
    let components = context.components.blocking_read();
    let radio_group = components.get(&radio_group_tron_id.to_string()).unwrap();
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

/// Adds a radio group component and its items to the given context.
///
/// # Arguments
///
/// * `context` - A mutable reference to the TnContextBase where the components will be added.
/// * `component_index` - A mutable reference to the index of the component.
/// * `radio_group_tron_id` - The TRON ID of the radio group.
/// * `radio_group_items` - A vector containing tuples of radio item TRON IDs and their labels.
/// * `container_attributes` - A vector containing attributes for the radio group container.
/// * `default_item` - The TRON ID of the default radio item.
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

/// Sets the value of a radio item in a radio group component based on the event trigger.
///
/// # Arguments
///
/// * `context` - The TnContext containing the radio group and radio item components.
/// * `event` - The TnEvent triggering the action.
/// * `_payload` - The payload associated with the event (not used in this function).
///
/// # Returns
///
/// A future resolving to a TnHtmlResponse representing the updated HTML response.
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
