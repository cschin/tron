use super::*;

pub struct TnButton<'a> {
    inner: ComponentBase<'a>
}

impl<'a> TnButton<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("button".to_string(), id, name);
        component_base.value = ComponentValue::String(value);
        component_base.attributes.insert("hx-vals".into(), 
                                         r##"js:{evt_target: event.currentTarget.id, evt_type:event.type}"##.into());
        component_base.attributes.insert("hx-ext".into(), 
                                         "json-enc".into());
        Self {inner: component_base}
    }
}

impl<'a> Default for TnButton<'a> {
    fn default() -> Self {
    let mut component_base =
            ComponentBase::default();
        component_base.value = ComponentValue::String("Button".to_string());
        Self {inner: component_base}
    }
}

impl<'a> ComponentBaseTrait<'a> for TnButton<'a> {
    fn id(&self) -> ComponentId {
        self.inner.id()
    }
    fn tron_id(&self) -> &String {
        self.inner.tron_id()
    }

    fn attributes(&self) -> &ElmAttributes {
        self.inner.attributes()
    }

    fn set_attribute(&mut self, key: String, val: String) {
        self.inner
            .attributes
            .insert(key, val);
    }

    fn generate_attr_string(&self) -> String {
        self.inner.generate_attr_string()
    }

    fn value(&self) -> &ComponentValue {
        &self.inner.value
    }

    fn set_value(&mut self, new_value: ComponentValue) {
        self.inner.value = new_value
    }

    fn state(&self) -> &ComponentState {
        &self.inner.state
    }

    fn set_state(&mut self, new_state: ComponentState) {
        self.inner.state = new_state;
    }

    fn assets(&self) -> &Option<HashMap<String, ComponentAsset>> {
        &None
    }

    fn render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</button>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "button"
            } 
        ))
    }
    fn get_children(&self) -> &Option<Vec<&'a ComponentBase<'a>>> {
        self.inner.get_children()
    }

}

