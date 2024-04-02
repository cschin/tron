use super::*;

#[derive(Default)]
pub struct TnButton {
    inner: ComponentBase
}

impl TnButton {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base =
            ComponentBase::new("button".to_string(), id, name);
        component_base.value = ComponentValue::String(value);
        Self {inner: component_base}
    }
}

impl ComponentBaseTrait for TnButton {
    fn id(&self) -> ComponentId {
        self.inner.id()
    }
    fn tron_id(&self) -> &String {
        self.inner.tron_id()
    }

    fn attributes(&self) -> &ElmAttributes {
        self.inner.attributes()
    }
    fn children_targets(&self) -> &Option<Vec<u32>> {
        self.inner.children_targets()
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

    fn set_value(&mut self, value: ComponentValue) {
        self.inner.value = value
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
                _ => "the_value_is_not_a_straing"
            } 
        ))
    }

}

