use super::*;
use serde::Serialize;
use tron_macro::*;

#[derive(Serialize)]
pub struct SseD3PlotTriggerMsg {
    pub server_event_data: TnServerEventData,
    pub d3_plot: String,
}

#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnD3Plot<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnD3PlotBuilder<'a> {
    pub fn init(mut self, tnid: String, d3_plot_script: String) -> TnD3PlotBuilder<'a> {
        let component_type = TnComponentType::D3Plot;
        TnComponentType::register_script(
            component_type.clone(),
            include_str!("../javascript/d3_plot.html"),
        );
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), tnid, component_type)
            .set_value(TnComponentValue::String(d3_plot_script))
            .set_attr("type", "d3_simple_scatter_plot")
            .set_attr("hx-trigger", "click, server_event")
            .set_attr("hx-swap", "none")
            .set_attr(
                "hx-vals",
                r##"js:{event_data:get_event_with_coordinate(event)}"##,
            )
            .build();
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

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for TnD3Plot<'a>
where
    'a: 'static,
{
    /// Renders the `TnScatterPlot` component.
    async fn render(&self) -> String {
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
    async fn initial_render(&self) -> String {
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

    async fn pre_render(&mut self, _ctx: &TnContextBase) {}

    async fn post_render(&mut self, _ctx: &TnContextBase) {}
}
