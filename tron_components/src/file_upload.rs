use super::*;
use std::default::Default;
use tron_macro::*;

#[allow(dead_code)]
#[derive(ComponentBase)]
pub struct TnFileUpload<'a: 'static> {
    base: TnComponentBase<'a>,
    title: String,
    button_attributes: HashMap<String, String>,
}

impl TnFileUploadBuilder<'static> {
    pub fn init(
        mut self,
        idx: TnComponentIndex,
        tnid: String,
        _title: String,
        _button_attributes: HashMap<String, String>,
    ) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), idx, tnid.clone(), TnComponentType::FileUpload)
            .set_value(TnComponentValue::None)
            .set_attribute("type".into(), "file_upload".into())
            .set_attribute("id".into(), tnid.clone())
            .set_attribute("hx-swap".into(), "none".into())
            .set_attribute("hx-trigger".into(), "finished".into())
            .set_attribute(
                "hx-vals".into(),
                "js:{event_data:get_event_with_files(event)}".into(),
            )
            .build();
        self
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

impl TnFileUpload<'static> {
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
#[allow(dead_code)]
#[derive(ComponentBase)]
pub struct TnDnDFileUpload<'a: 'static> {
    base: TnComponentBase<'a>,
    title: String,
    button_attributes: HashMap<String, String>,
}

impl TnDnDFileUploadBuilder<'static> {
    pub fn init(
        mut self,
        idx: TnComponentIndex,
        tnid: String,
        _title: String,
        _button_attributes: HashMap<String, String>,
    ) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init(
                "div".into(),
                idx,
                tnid.clone(),
                TnComponentType::DnDFileUpload,
            )
            .set_value(TnComponentValue::None)
            .set_attribute("type".into(), "file_dnd_upload".into())
            .set_attribute("id".into(), tnid.clone())
            .set_attribute("hx-swap".into(), "none".into())
            .set_attribute("hx-trigger".into(), "finished".into())
            .set_attribute(
                "hx-vals".into(),
                "js:{event_data:get_event_with_files_dnd(event)}".into(),
            )
            .build();
        self
    }
}

impl Default for TnDnDFileUpload<'static> {
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

impl TnDnDFileUpload<'static> {
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
            </div>
            <script>
                let theDropzone = new Dropzone('#{tron_id}_form', {{paramName:'{tron_id}_form',  url: "/upload/{}"}});
                window.tron_assets["dropzones"] = {{"{tron_id}_form":theDropzone}};
                theDropzone.on("complete", function(file) {{
                    htmx.trigger("#{tron_id}", "finished", {{}});
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
