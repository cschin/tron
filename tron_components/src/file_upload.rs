use super::*;
use askama::Template;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnFileUpload<'a: 'static> {
    base: TnComponentBase<'a>,
    title: String,
    button_attributes: HashMap<String, String>,
}

#[derive(Template)] // this will generate the code...
#[template(path = "file_upload.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct ScriptTemplate {
    tron_id: String,
}

impl<'a: 'static> TnFileUpload<'a> {
    pub fn new(
        idx: TnComponentIndex,
        tnid: String,
        title: String,
        button_attributes: HashMap<String, String>,
    ) -> Self {
        let mut base =
            TnComponentBase::new("div".into(), idx, tnid.clone(), TnComponentType::FileUpload);
        base.set_value(TnComponentValue::None);
        base.set_attribute("type".into(), "file_upload_widget".into());
        base.set_attribute("id".into(), format!("{}_container", tnid.clone()));
        base.remove_attribute("hx-swap".into());
        base.remove_attribute("hx-target".into());
        base.remove_attribute("hx-post".into());
        base.remove_attribute("hx-vals".into());
        base.remove_attribute("hx-ext".into());
        base.remove_attribute("state".into());
        let script = ScriptTemplate { tron_id: tnid };
        let script = script.render().unwrap();
        base.script = Some(script);

        Self {
            base,
            title,
            button_attributes,
        }
    }
}

impl<'a: 'static> Default for TnFileUpload<'a> {
    /// Returns the default instance of `TnScatterPlot`.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::None,
                ..Default::default()
            },
            title: "File Upload".into(),
            button_attributes: HashMap::default(),
        }
    }
}

impl<'a: 'static> TnFileUpload<'a>
where
    'a: 'static,
{
    /// Renders the `TnScatterPlot` component.
    pub fn internal_render(&self) -> String {
        let tron_id = self.tron_id();
        let button_attributes = self
            .button_attributes
            .iter()
            .map(|(k, v)| {
                if v.is_empty() {
                    k.clone()
                } else {
                    format!(r#"{}="{}""#, k, tron_utils::html_escape_double_quote(v))
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        format!(
            r#"<{} {}><label>{}</label><form id='{}' hx-encoding='multipart/form-data' hx-post='/upload/{}' hx-swap="none" hx-val="js:{{event_data:get_event(event)}}">
                <input type='file' name='{}' multiple>
                <button {}>
                    Upload File
                </button>
                <progress id='progress' value='0' max='100'></progress>
            </form></{}>"#,
            self.base.tag,
            self.generate_attr_string(),
            self.title,
            tron_id,
            self.id(),
            tron_id,
            button_attributes,
            self.base.tag
        )
    }
    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}
