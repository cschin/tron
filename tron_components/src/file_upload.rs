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

impl TnFileUpload<'static> {
    pub fn new(
        idx: TnComponentIndex,
        tnid: String,
        title: String,
        button_attributes: HashMap<String, String>,
    ) -> Self {
        let mut base =
            TnComponentBase::new("div".into(), idx, tnid.clone(), TnComponentType::FileUpload);
        base.set_value(TnComponentValue::None);
        base.set_attribute("type".into(), "file_upload".into());
        base.set_attribute("id".into(), tnid.clone());
        base.set_attribute("hx-swap".into(), "none".into());
        base.set_attribute("hx-trigger".into(), "finished".into());
        base.set_attribute(
            "hx-vals".into(),
            "js:{event_data:get_event_with_files(event)}".into(),
        );
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

impl Default for TnFileUpload<'static> {
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

impl TnFileUpload<'static>
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
        let container_class = if self.attributes().contains_key("class") {
            format!(
                r#"class="{}"#,
                self.attributes().get("class").unwrap().clone()
            )
        } else {
            "".into()
        };
        format!(
            r##"<div {container_class}><{} {}></{}><label>{}</label><form id='{tron_id}_form' hx-encoding='multipart/form-data' hx-post='/upload/{}' hx-swap="none" hx-val="js:{{event_data:get_event(event)}}">
                <input id="{tron_id}_input" type='file' name='{tron_id}_form' multiple>
                <button {button_attributes}>
                    Click to Upload
                </button>
                <progress id='{tron_id}_progress' value='0' max='100'></progress>
            </form></div>
            <script>
            htmx.on('#{tron_id}_form', 'htmx:xhr:progress', function(evt) {{
                htmx.find('#{tron_id}_progress').setAttribute('value', evt.detail.loaded/evt.detail.total * 100)
            }});

            htmx.on('#{tron_id}_form', 'htmx:afterRequest', function(evt) {{
            if (evt.detail.successful) {{
                htmx.trigger("#{tron_id}", "finished", {{}});
            }}
            }});</script>"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag,
            self.title,
            self.id()
        )
    }

    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

// Drag and Drop 

#[derive(ComponentBase)]
pub struct TnDnDFileUpload<'a: 'static> {
    base: TnComponentBase<'a>,
    title: String,
    button_attributes: HashMap<String, String>, 
}

#[derive(Template)] // this will generate the code...
#[template(path = "dnd_file_upload.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct DnDScriptTemplate {
    tron_id: String,
}

impl TnDnDFileUpload<'static> {
    pub fn new(
        idx: TnComponentIndex,
        tnid: String,
        title: String,
        button_attributes: HashMap<String, String>,
    ) -> Self {
        let mut base =
            TnComponentBase::new("div".into(), idx, tnid.clone(), TnComponentType::FileUpload);
        base.set_value(TnComponentValue::None);
        base.set_attribute("type".into(), "file_dnd_upload".into());
        base.set_attribute("id".into(), tnid.clone());
        base.set_attribute("hx-swap".into(), "none".into());
        base.set_attribute("hx-trigger".into(), "finished".into());
        base.set_attribute(
            "hx-vals".into(),
            "js:{event_data:get_event_with_files_dnd(event)}".into(),
        );
        let script = DnDScriptTemplate { tron_id: tnid };
        let script = script.render().unwrap();
        base.script = Some(script);

        Self {
            base,
            title,
            button_attributes
        }
    }
}



impl TnDnDFileUpload<'static>
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
        let container_class = if self.attributes().contains_key("class") {
            format!(
                r#"class="{}"#,
                self.attributes().get("class").unwrap().clone()
            )
        } else {
            "".into()
        };
        format!(
            r##"<div {container_class}><{} {}></{}><label>{}</label>
            <form class="dropzone"
                  id="{tron_id}_form"
                  hx-encoding='multipart/form-data' hx-post='/upload/{}'>
            </form>
            <button {button_attributes} id="{tron_id}_clear_btn">
                    Clear
            </button>
            <script>
                htmx.on('#{tron_id}_form', 'htmx:afterRequest', function(evt) {{
                    if (evt.detail.successful) {{
                        htmx.trigger("#{tron_id}", "finished", {{}});
                    }};
                }});
                document.querySelector('#dropzone_lib').addEventListener('load', 
                    function () {{
                        let theDropzone = new Dropzone('#{tron_id}_form', {{paramName:'{tron_id}_form',  url: "/upload/{}"}});
                        window.tron_assets["dropzones"] = {{"{tron_id}_form":theDropzone}};
                }});
                document.querySelector('#{tron_id}_clear_btn').addEventListener('click', 
                    function () {{ 
                        window.tron_assets["dropzones"]["{tron_id}_form"].removeAllFiles(); 
                }});
            </script>"##,
            self.base.tag,
            self.generate_attr_string(),
            self.base.tag,
            self.title,
            self.id(),
            self.id()
        )
    }

    /// Renders the `TnRangeSlider` component for the first time.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}