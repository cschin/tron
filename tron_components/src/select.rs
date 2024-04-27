use super::*;
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
                        let mut selected = "";
                        if let ComponentValue::String(s) = self.value() {
                            if *s == *k {
                                selected = "selected"
                            }
                        } 
                        format!(r#"<option value="{}" {}>{}</option>"#, k, selected, v)
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

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
