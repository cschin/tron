use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnTextArea<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnTextArea<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("textarea".into(), id, name, TnComponentType::TextArea);
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
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "textarea",
            },
            self.inner.tag
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

pub async fn append_textarea_value(
    comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>,
    new_str: &str,
    sep: Option<&str>,
) {
    let v;
    {
        let comp = comp.read().await;
        let v0 = match comp.value() {
            ComponentValue::String(s) => s.clone(),
            _ => "".into(),
        };
        v = [v0, new_str.to_string()];
    }
    {
        let mut comp = comp.write().await;
        let sep = sep.unwrap_or("");
        comp.set_value(ComponentValue::String(v.join(sep)));
    }
}

#[derive(ComponentBase)]
pub struct TnStreamTextArea<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnStreamTextArea<'a> {
    pub fn new(id: ComponentId, name: String, value: Vec<String>) -> Self {
        let mut component_base =
            ComponentBase::new("textarea".into(), id, name, TnComponentType::StreamTextArea);
        component_base.set_value(ComponentValue::VecString(value));
        component_base.set_attribute("contenteditable".into(), "false".into());

        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute("type".into(), "text".into());
        component_base.set_attribute(
            "hx-swap".into(),
            "beforeend scroll:bottom focus-scroll:true ".into(),
        );

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnStreamTextArea<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::VecString(vec![]),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnStreamTextArea<'a> {
    pub fn internal_first_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::VecString(s) => s.join(""),
                _ => "".to_string(),
            },
            self.inner.tag
        )
    }

    pub fn internal_render(&self) -> String {
        let empty = "".to_string();
        match self.value() {
            ComponentValue::VecString(s) => s.last().unwrap_or(&empty).clone(),
            _ => "".into(),
        }
    }
}

pub async fn append_stream_textarea_value(
    comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>,
    new_str: &str,
) {
    let mut comp = comp.write().await;
    if let ComponentValue::VecString(v) = comp.get_mut_value() {
        v.push(new_str.to_string());
    }
}

#[derive(ComponentBase)]
pub struct TnTextInput<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnTextInput<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("input".into(), id, name, TnComponentType::TextInput);
        component_base.set_value(ComponentValue::String(value.to_string()));
        component_base.set_attribute("contenteditable".into(), "true".into());

        component_base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        component_base.set_attribute("type".into(), "text".into());
        component_base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        ); //over-ride the default as we need the value of the input text
        component_base.set_attribute("hx-swap".into(), "none".into());

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
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} value="{}">"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => html_escape_double_quote(s),
                _ => "Lorem ipsum dolor sit amet, consectetur adipiscing elit. Nullam eu pellentesque erat, ut sollicitudin nisi.".to_string()
            }
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
