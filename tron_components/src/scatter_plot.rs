use super::*;
use tron_macro::*;

/// Represents a range slider component.
#[derive(ComponentBase)]
pub struct TnSimpleScatterPlot<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnSimpleScatterPlot<'a> {
    /// Creates a new instance of `TnSimpleScatterPlot`.
    ///
    /// # Arguments
    ///
    /// * `idx` - The unique identifier for the range slider component.
    /// * `tnid` - The name of the range slider component.
    /// * `value` - The initial value of the range slider.
    ///
    /// # Returns
    ///
    /// A new instance of `TnRangeSlider`.
    pub fn new(
        idx: TnComponentIndex,
        tnid: String,
        xy: Vec<(f32, f32)>,
        c: Vec<(f32, f32, f32, f32)>,
        s: Vec<f32>,
    ) -> Self {
        let mut base = TnComponentBase::new("svg".into(), idx, tnid, TnComponentType::SimpleScatterPlot);
        base.set_value(TnComponentValue::None);
        base.set_attribute("type".into(), "simple_scatter_plot".into());
        base.create_assets();
        assert!(xy.len() == c.len() && c.len() == s.len());
        let mut xmin = f32::MAX;
        let mut xmax = f32::MIN;
        let mut ymin = f32::MAX;
        let mut ymax = f32::MIN;
        if xy.is_empty() {
            (xmin, xmax, ymin, ymax) = (0.0, 100.0, 0.0, 100.0); //unit in pixel
        };
        xy.iter().for_each(|&(x, y)| {
            if x < xmin {
                xmin = x;
            }
            if x > xmax {
                xmax = x;
            }
            if y < ymin {
                ymin = y;
            }
            if y > ymax {
                ymax = y;
            }
        });
        let assets = base.get_mut_assets().unwrap();
        assets.insert("xy".into(), TnAsset::VecF32_2(xy));
        assets.insert("c".into(), TnAsset::VecF32_4(c));
        assets.insert("s".into(), TnAsset::VecF32(s));

        let xr = xmax - xmin;
        let yr = ymax - ymin;
        let xmin = xmin - xr * 0.1;
        //let xmax = xmax + xr * 0.05;
        let ymin = ymin - yr * 0.1;
        //let ymax = ymax + yr * 0.05;
        let xr = xr * 1.2;
        let yr = yr * 1.2;
        base.set_attribute("viewBox".into(), format!("{xmin} {ymin} {xr} {yr}"));
        base.set_attribute( "weight".into(), format!("{xr}"));
        base.set_attribute( "height".into(), format!("{yr}"));

        base.set_attribute("hx-trigger".into(), "click, server_side_trigger".into());
        base.set_attribute("hx-swap".into(), "none".into());

        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_event_with_mouse_coordinate(event)}"##.into(),
        );

        base.script = Some(include_str!("../javascript/scatter_plot.html").to_string());

        Self {
            base,
        }
    }
}

impl<'a: 'static> Default for TnSimpleScatterPlot<'a> {
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

impl<'a: 'static> TnSimpleScatterPlot<'a>
where
    'a: 'static,
{
    /// Renders the `TnScatterPlot` component.
    pub fn internal_render(&self) -> String {
        let xy = if let TnAsset::VecF32_2(xy) = self.get_assets().unwrap().get("xy").unwrap() {
            xy
        } else {
            unreachable!()
        };

        let c = if let TnAsset::VecF32_4(c) = self.get_assets().unwrap().get("c").unwrap() {
            c
        } else {
            unreachable!()
        };

        let s = if let TnAsset::VecF32(s) = self.get_assets().unwrap().get("s").unwrap() {
            s
        } else {
            unreachable!()
        };

        let svg_string = (0..xy.len()).map( move |i| {
            let x = xy[i].0;
            let y = xy[i].1;
            let s = s[i];
            let (r, g, b, a) = c[i];
            let r = r*256.0;
            let g = g*256.0;
            let b = b*256.0;
            format!(r##"<circle cx="{x}" cy="{y}" r="{s}" fill="rgb({r}, {g}, {b})" opacity="{a}" />"##)
        }).collect::<Vec<_>>().join("");

        format!(
            r##"<{} {}>{svg_string}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag
        )
    }
    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
