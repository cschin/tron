use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnAudioPlayer<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnAudioPlayer<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new(
            "audio".to_string(),
            id,
            name.clone(),
            TnComponentType::AudioPlayer,
        );
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("src".into(), format!("/tron_streaming/{}", name));
        // component_base.set_attribute("type".into(), "audio/webm".into());
        component_base.set_attribute("type".into(), "audio/mp3".into());
        component_base.set_attribute("hx-trigger".into(), "server_side_trigger, ended".into());
        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnAudioPlayer<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("audio".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnAudioPlayer<'a>
where
    'a: 'static,
{
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} controls autoplay>"##,
            self.inner.tag,
            self.generate_attr_string(),
        )
    }
}
