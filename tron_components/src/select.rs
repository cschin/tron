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
            .set_attr("hx-trigger", "change, server_event")
            .set_attr("type", "select")
            .set_attr("hx-swap", "none")
            .set_attr("hx-vals", r##"js:{event_data:get_input_event(event)}"##)
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

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for TnSelect<'a>
where
    'a: 'static,
{
    /// Renders the TnSelect component.
    async fn render(&self) -> String {
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
    async fn initial_render(&self) -> String {
        self.render().await
    }
    async fn pre_render(&mut self, _ctx: &TnContextBase) {}

    async fn post_render(&mut self, _ctx: &TnContextBase) {}
}
