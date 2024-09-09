use super::*;
use tron_macro::*;

/// Represents a select component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnSelect<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnSelectBuilder<'static> {
    /// Creates a new select component.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the component.
    /// * `tnid` - The unique ID of the component.
    /// * `value` - The initial value of the select.
    /// * `options` - A vector of tuples representing the options of the select, where each tuple contains the value and label of an option.
    pub fn init(
        mut self,
        tnid: TnComponentId,
        value: String,
        options: Vec<(String, String)>,
    ) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("select".into(), tnid, TnComponentType::Select)
            .set_value(TnComponentValue::String(value))
            .set_attribute("hx-trigger", "change, server_side_trigger")
            .set_attribute("type", "select")
            .set_attribute("hx-swap", "none")
            .set_attribute(
                "hx-vals",
                r##"js:{event_data:get_input_event(event)}"##,
            )
            .create_assets()
            .build(); //over-ride the default as we need the value of the input text
        self.base
            .asset
            .as_mut()
            .unwrap()
            .insert("options".into(), TnAsset::VecString2(options));
        self
    }
}

impl Default for TnSelect<'static> {
    /// Returns a default TnSelect component.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("select_default".into()),
                ..Default::default()
            },
        }
    }
}

impl TnSelect<'static> {
    /// Renders the TnSelect component.
    pub fn internal_render(&self) -> String {
        let options = {
            let options = self.base.asset.as_ref().unwrap().get("options").unwrap();
            if let TnAsset::VecString2(options) = options {
                options
                    .iter()
                    .map(|(k, v)| {
                        let mut selected = "";
                        if let TnComponentValue::String(s) = self.value() {
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
            self.base.tag,
            self.tron_id(),
            self.tron_id(),
            self.generate_attr_string(),
            options,
            self.base.tag
        )
    }

    /// Renders the TnSelect component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
