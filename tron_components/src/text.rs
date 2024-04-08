use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnText<'a> {
    inner: ComponentBase<'a>
}

impl<'a> TnText<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("div".to_string(), id, name);
        component_base.value = ComponentValue::String(value);
        component_base.attributes.insert("hx-vals".into(), 
                                         r##"js:{evt_target: event.currentTarget.id, evt_type:event.type}"##.into());
        component_base.attributes.insert("hx-ext".into(), 
                                         "json-enc".into());
        component_base.attributes.insert("contenteditable".into(), 
                                         "true".into());
        Self {inner: component_base}
    }
}

impl<'a> Default for TnText<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("div".to_string()),
                .. Default::default() 
            }
        }
    }
}

impl<'a> TnText<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam eu pellentesque erat, ut sollicitudin nisi."
            },
            self.inner.tag 
        ))

    }
}