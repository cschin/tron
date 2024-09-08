use super::*;
use serde::Serialize;
use tron_macro::*;

#[derive(Serialize)]
pub struct SseD3PlotTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
    pub d3_plot: String,
}

#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnD3Plot<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a:'static> TnD3PlotBuilder<'a>  {
    pub fn init(mut self, idx: TnComponentIndex, tnid: String, d3_plot_script: String) -> TnD3PlotBuilder<'a> {
        self.base.init("div".into(), idx, tnid, TnComponentType::D3Plot);
        self.base.set_value(TnComponentValue::String(d3_plot_script));
        self.base.set_attribute("type".into(), "d3_simple_scatter_plot".into());

        self.base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());
        self.base.set_attribute("hx-swap".into(), "none".into());

        self.base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_event_with_coordinate(event)}"##.into(),
        );
        self
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

impl TnD3Plot<'static> {
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
            <script>d3_plot("{tron_id}");</script>"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag
        )
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}
