use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnAudioPlayer<'a> {
    inner: ComponentBase<'a>,
}

impl<'a> TnAudioPlayer<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("audio".to_string(), id, name);
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("hx-post".into(), format!("/tron/streaming/{}", id));
        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.assets = Some(HashMap::<String, ComponentAsset>::default());
        component_base.assets.as_mut().unwrap().insert(
            "channels".into(),
            ComponentAsset::Bytes(BytesMut::default()),
        );
        Self {
            inner: component_base,
        }
    }
}

impl<'a> Default for TnAudioPlayer<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("audio".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a> TnAudioPlayer<'a> {
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
