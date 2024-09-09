use super::*;
use tron_macro::*;

/// Represents a range slider component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnRangeSlider<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnRangeSliderBuilder<'static> {
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
    pub fn init(mut self, tnid: TnComponentId, value: f32, min: f32, max: f32) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("input".into(), tnid, TnComponentType::Slider)
            .set_value(TnComponentValue::String(format!("{}", value)))
            .set_attribute("type", "range")
            .set_attribute("min", &format!("{}", min))
            .set_attribute("max", &format!("{}", max))
            .set_attribute(
                "hx-vals",
                r##"js:{event_data:get_input_event(event)}"##,
            )
            .set_attribute("hx-trigger", "change, server_side_trigger")
            .set_attribute("hx-swap", "none")
            .build();
        self
    }
}

impl Default for TnRangeSlider<'static> {
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

impl TnRangeSlider<'static> {
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

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
