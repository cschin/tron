use super::*;
use tron_macro::*;

/// Represents a select component.
#[derive(ComponentBase)]
pub struct TnSelect<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnSelect<'a> {
    /// Creates a new select component.
    ///
    /// # Arguments
    ///
    /// * `idx` - The index of the component.
    /// * `tnid` - The unique ID of the component.
    /// * `value` - The initial value of the select.
    /// * `options` - A vector of tuples representing the options of the select, where each tuple contains the value and label of an option.
    pub fn new(
        idx: TnComponentIndex,
        tnid: String,
        value: String,
        options: Vec<(String, String)>,
    ) -> Self {
        let mut base = TnComponentBase::new("select".into(), idx, tnid, TnComponentType::Select);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("type".into(), "select".into());
        base.set_attribute("hx-swap".into(), "none".into());
        base.asset = Some(HashMap::default());
        base.asset
            .as_mut()
            .unwrap()
            .insert("options".into(), TnAsset::VecString2(options));
        base.script = Some(include_str!("../javascript/select.html").to_string());
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text

        Self { base }
    }
}

impl<'a: 'static> Default for TnSelect<'a> {
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

impl<'a: 'static> TnSelect<'a> {
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
}
