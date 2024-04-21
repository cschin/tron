use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnTextArea<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnTextArea<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("textarea".into(), id, name);
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("contenteditable".into(), "true".into());

        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute("type".into(), "text".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnTextArea<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnTextArea<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "textarea",
            },
            self.inner.tag
        ))
    }
}

pub fn append_textarea_value(
    comp: &mut Box<dyn ComponentBaseTrait<'static>>,
    new_str: &str,
    sep: Option<&str>,
) {
    let v = match comp.value() {
        ComponentValue::String(s) => s.clone(),
        _ => "".into(),
    };
    let v = [v, new_str.to_string()]; 
    let sep = sep.unwrap_or("");
    comp.set_value(ComponentValue::String(v.join(sep)));
}

#[derive(ComponentBase)]
pub struct TnTextInput<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnTextInput<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("input".into(), id, name);
        component_base.set_value(ComponentValue::String(value.to_string()));
        component_base.set_attribute("contenteditable".into(), "true".into());

        component_base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        component_base.set_attribute("type".into(), "text".into());
        component_base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnTextInput<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("input".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnTextInput<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {} value="{}">"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => utils::html_escape_double_quote(s),
                _ => "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam eu pellentesque erat, ut sollicitudin nisi.".to_string()
            }
        ))
    }
}
