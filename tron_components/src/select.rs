use super::*;
use futures_util::Future;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnSelect<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnSelect<'a> {
    pub fn new(
        id: ComponentId,
        tron_id: String,
        value: String,
        options: Vec<(String, String)>,
    ) -> Self {
        let mut component_base =
            ComponentBase::new("select".into(), id, tron_id, TnComponentType::Select);
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        component_base.set_attribute("type".into(), "select".into());
        component_base.set_attribute("hx-swap".into(), "none".into());
        component_base.assets = Some(HashMap::default());
        component_base
            .assets
            .as_mut()
            .unwrap()
            .insert("options".into(), TnAsset::VecString2(options));
        component_base.script = Some(include_str!("../javascript/select.html").to_string());
        component_base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text
        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnSelect<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("select_default".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnSelect<'a> {
    pub fn internal_render(&self) -> String {
        let options = {
            let options = self.inner.assets.as_ref().unwrap().get("options").unwrap();
            if let TnAsset::VecString2(options) = options {
                options
                    .iter()
                    .map(|(k, v)| {
                        if let ComponentValue::String(component_value) = self.value() {
                            if *k == *component_value {
                                format!(r#"<option value="{}" selected>{}</option>"#, k, v)
                            } else {
                                format!(r#"<option value="{}">{}</option>"#, k, v)
                            }
                        } else{
                            format!(r#"<option value="{}">{}</option>"#, k, v)
                        } 
                    })
                    .collect::<Vec<String>>()
                    .join("\n")
            } else {
                "".into()
            }
        };

        format!(
            r##"<{} name="{}" id="{}" {} class="flex flex-row p-1 flex-1">{}</{}>"##,
            self.inner.tag,
            self.tron_id(),
            self.tron_id(),
            self.generate_attr_string(),
            options,
            self.inner.tag
        )
    }
}

pub fn get_select_actions(
    comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>,
) -> (TnEvent, Arc<ActionFn>) {
    let comp_guard = comp.blocking_write();
    assert!(comp_guard.get_type() == TnComponentType::Select);
    let evt = TnEvent {
        e_target: comp_guard.tron_id().clone(),
        e_type: "change".into(),
        e_state: "ready".into(),
    };
    (evt, Arc::new(set_select))
}

pub fn set_select(
    context: Arc<RwLock<Context<'static>>>,
    event: TnEvent,
    payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        // println!("paylod value {payload}");
        if let Value::String(selected) = &payload["event_data"]["e_value"] {
            let context_guard = context.read().await;
            let select_id = context_guard.get_component_id(&event.e_target);
            let components_guard = context_guard.components.write().await;
            let mut select = components_guard.get(&select_id).unwrap().write().await;
            select.set_value(ComponentValue::String(selected.clone()));

            select.set_state(ComponentState::Ready);
        };
    };

    Box::pin(f)
}
