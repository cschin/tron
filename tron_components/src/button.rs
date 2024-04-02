use super::*;

pub type SimpleButtonValueType = String;
#[derive(ComponentBase, Default)]
pub struct TnButton {
    pub component_base: ComponentBase<SimpleButtonValueType, ()>,
}



impl TnButton {
    pub fn new(id: ComponentId, name: String, value: SimpleButtonValueType) -> Self {
        let mut component_base =
            ComponentBase::<SimpleButtonValueType, ()>::new("button".to_string(), id, name);
        component_base.value = Some(value);
        Self {
            component_base,
            ..Default::default()
        }
    }
}

impl ComponentRenderTraits for TnButton {
    fn render(&self) -> Html<String> {
        Html::from(format!(
            r##"<button id="{}" {}>{}</button>"##,
            self.component_base.tron_id,
            self.generate_attr_string(),
            self.component_base.value.clone().unwrap()
        ))
    }
}

impl ComponentValueTrait<SimpleButtonValueType> for TnButton {
    fn value(&self) -> &Option<SimpleButtonValueType> {
        self.component_base.value()
    }
    fn set_value(&mut self, value: SimpleButtonValueType) -> &Self {
        self.component_base.value = Some(value);
        self
    }
}
