use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnRangeSlider<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnRangeSlider<'a> {
    pub fn new(id: TnComponentIndex, name: String, value: f32, min: f32, max: f32) -> Self {
        let mut base = TnComponentBase::new("input".into(), id, name, TnComponentType::Button);
        base.set_value(TnComponentValue::String(format!("{}",value)));
        base.set_attribute("type".into(), "range".into());
        base.set_attribute("min".into(), format!("{}", min));
        base.set_attribute("max".into(), format!("{}", max));
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        );
        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());

        Self { base }
    }
}

impl<'a: 'static> Default for TnRangeSlider<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("range".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnRangeSlider<'a>
where
    'a: 'static,
{
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} type="range" {} value="{}"/>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(v) => v,
                _ => "0.0",
            },
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
