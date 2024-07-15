use super::*;
use serde::Serialize;
use tron_macro::*;

#[derive(Serialize)]
pub struct SseD3PlotTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
    pub d3_plot: String,
}

#[derive(ComponentBase)]
pub struct TnD3Plot<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnD3Plot<'static> {
    /// Creates a new `TnComponent` instance specifically for a D3 simple scatter plot visualization.
    ///
    /// # Arguments
    /// * `idx`: The index of the component in the parent container.
    /// * `tnid`: A unique identifier for the component.
    /// * `d3_plot_script`: A string containing the JavaScript code for a D3 simple scatter plot.
    ///
    /// # Returns
    /// Returns a new instance of the component, initialized with the provided `idx`, `tnid`, and `d3_plot_script`,
    /// and configured with attributes specific for handling a D3 scatter plot, including interactive JavaScript actions.
    ///
    /// The component is also set with attributes to specify its behavior on certain events like `click`,
    /// with specific data handling and swap behavior defined using `hx-trigger`, `hx-swap`, and `hx-vals` attributes.
    ///
    /// An external JavaScript file for the D3 scatter plot is included and linked to the component's script property.
    pub fn new(idx: TnComponentIndex, tnid: String, d3_plot_script: String) -> Self {
        let mut base = TnComponentBase::new("div".into(), idx, tnid, TnComponentType::D3Plot);
        base.set_value(TnComponentValue::String(d3_plot_script));
        base.set_attribute("type".into(), "d3_simple_scatter_plot".into());

        base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());
        base.set_attribute("hx-swap".into(), "none".into());

        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_event_with_coordinate(event)}"##.into(),
        );

        base.script = Some(include_str!("../javascript/d3_plot.html").to_string());

        Self { base }
    }
}

impl Default for TnD3Plot<'static> {
    /// Returns the default instance of `TnScatterPlot`.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::None,
                ..Default::default()
            },
        }
    }
}

impl TnD3Plot<'static>
{
    /// Renders the `TnScatterPlot` component.
    pub fn internal_render(&self) -> String {
        let d3_plot_script = if let TnComponentValue::String(s) = self.value() {
            s.clone()
        } else {
            unreachable!()
        };
        format!(
            r##"<{} {}></{}>{d3_plot_script}"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag
        )
    }
    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        let d3_plot_script = if let TnComponentValue::String(s) = self.value() {
            s.clone()
        } else {
            unreachable!()
        };
        let tron_id = self.tron_id();
        format!(
            r##"<{} {}></{}>{d3_plot_script} 
            <script>document.querySelector('#d3_lib').addEventListener('load', function () {{d3_plot("{tron_id}");}});</script>"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag
        )
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
