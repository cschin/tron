use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnCheckList<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnCheckList<'a> {
    pub fn new(id: ComponentId, name: String, value: HashMap<String, bool>) -> Self {
        let mut component_base = ComponentBase::new("div".into(), id, name, "checklist".into());
        component_base.set_value(ComponentValue::CheckItems(value));
        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute("type".into(), "checklist".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnCheckList<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnCheckList<'a> {
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
}

#[derive(ComponentBase)]
pub struct TnCheckBox<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnCheckBox<'a> {
    pub fn new(id: ComponentId, name: String, value: bool) -> Self {
        let mut component_base = ComponentBase::new("div".into(), id, name, "checklist".into());
        component_base.set_value(ComponentValue::CheckItem(value));
        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute("type".into(), "checkbox".into());

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnCheckBox<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("".into()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnCheckBox<'a> {
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
}