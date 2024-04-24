use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnCheckList<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnCheckList<'a> {
    pub fn new(id: ComponentId, name: String, value: HashMap<String, bool>) -> Self {
        let mut component_base = ComponentBase::new("div".into(), id, name, TnComponentType::CheckList);
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
        let childred_render_results = self.get_children().iter().map(|c: &Arc<RwLock<Box<dyn ComponentBaseTrait<'a>>>>| {
            c.blocking_read().render()
        }).collect::<Vec<String>>().join(" ");
        format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            childred_render_results,
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
        let mut component_base = ComponentBase::new("input".into(), id, name, TnComponentType::CheckBox);
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
            r##"<div><{} {} type="checkbox" /><label>&nbsp;{}</label></div>"##,
            self.inner.tag,
            self.generate_attr_string(),
            self.tron_id()
        )
    }
}