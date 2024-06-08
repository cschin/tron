use super::*;
use tron_macro::*;

/// Represents a range slider component.
#[derive(ComponentBase)]
pub struct TnRangeSlider<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnRangeSlider<'a> {
    /// Creates a new instance of `TnRangeSlider`.
    ///
    /// # Arguments
    ///
    /// * `idx` - The unique identifier for the range slider component.
    /// * `tnid` - The name of the range slider component.
    /// * `value` - The initial value of the range slider.
    /// * `min` - The minimum value of the range slider.
    /// * `max` - The maximum value of the range slider.
    ///
    /// # Returns
    ///
    /// A new instance of `TnRangeSlider`.
    pub fn new(idx: TnComponentIndex, tnid: String, value: f32, min: f32, max: f32) -> Self {
        let mut base = TnComponentBase::new("input".into(), idx, tnid, TnComponentType::Slider);
        base.set_value(TnComponentValue::String(format!("{}",value)));
        base.set_attribute("type".into(), "range".into());
        base.set_attribute("min".into(), format!("{}", min));
        base.set_attribute("max".into(), format!("{}", max));
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_input_event(event)}"##.into(),
        );
        base.set_attribute("hx-trigger".into(), "change, server_side_trigger".into());
        base.set_attribute("hx-swap".into(), "none".into());


        Self { base }
    }
}

impl<'a: 'static> Default for TnRangeSlider<'a> {
    /// Returns the default instance of `TnRangeSlider`.
    ///
    /// The default value is set to "range", representing a default range value.
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
    /// Renders the `TnRangeSlider` component.
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
    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self)  {
    }

    pub fn internal_post_render(&mut self)  {
    }
}
