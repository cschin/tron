use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnAudioRecorder<'a> {
    inner: ComponentBase<'a>,
}

impl<'a> TnAudioRecorder<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("div".to_string(), id, name);
        component_base.set_value(ComponentValue::String(value));
        component_base
            .set_attribute("hx-trigger".into(), "streaming, server_side_trigger".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a> Default for TnAudioRecorder<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("div".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a> TnAudioRecorder<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "paused",
            },
            self.inner.tag
        ))
    }
}
