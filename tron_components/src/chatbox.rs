use super::*;
use tron_macro::*;
use tracing;

#[derive(ComponentBase)]
pub struct TnChatBox<'a: 'static> {
    inner: ComponentBase<'a>,
}

impl<'a: 'static> TnChatBox<'a> {
    pub fn new(id: ComponentId, name: String, value: Vec<(String, String)>) -> Self {
        let mut component_base =
            ComponentBase::new("div".into(), id, name, TnComponentType::ChatBox);
        component_base.set_value(ComponentValue::VecString2(value));

        component_base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        component_base.set_attribute(
            "hx-swap".into(),
            "beforeend scroll:bottom focus-scroll:true ".into(),
        );
        component_base.set_attribute(
            "class".into(),
            "flex-col".into(),
        );
        component_base.assets = Some(HashMap::default());
        let class = HashMap::from_iter(vec![
            ("user".to_string(), "max-w-fill flex flex-row justify-end p-3 > bg-green-100 rounded-lg p-2 mb-2 text-right".to_string()),
            ("bot".to_string(), "max-w-fill flex flex-row justify-start p-3 > bg-blue-100 rounded-lg p-2 mb-2 text-left".to_string()),
        ]);
        let assets: &mut HashMap<String, TnAsset> = component_base.assets.as_mut().unwrap();
        assets.insert("class".into(), TnAsset::HashMapString(class));
        component_base.script = Some(include_str!("../javascript/chatbox.html").to_string());

        Self {
            inner: component_base,
        }
    }
}

impl<'a: 'static> Default for TnChatBox<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::VecString2(vec![]),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnChatBox<'a> {
    pub fn internal_first_render(&self) -> String {
        tracing::info!(target: "tron_app", "internal_first_render");
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };

        let chat_recorders = if let ComponentValue::VecString2(tag_msg) = self.value() {
            tag_msg
                .iter()
                .map(|(tag, msg)| {
                    let class_str = if let Some(class) = class {
                        class.get(tag)
                    } else {
                        None
                    };
                    if class_str.is_some() {
                        let class_str = class_str.unwrap().clone();
                        let mut class_strs = class_str.split('>');
                        let class_parent = class_strs.next().unwrap(); 
                        let class_str = class_strs.next().unwrap(); 
                        format!(r#"<div class="{class_parent}"><div class="{class_str}">{msg}</div></div>"#)
                    } else {
                        format!(r#"<div><div>{msg}</div></div>"#)
                    }
                })
                .collect::<Vec<String>>()
                .join("")
        } else {
            "".to_string()
        };

        tracing::info!(target: "tron_app", "{}", chat_recorders);

        format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            chat_recorders,
            self.inner.tag
        )
    }

    pub fn internal_render(&self) -> String { 
        tracing::info!(target: "tron_app", "internal_render");
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };
        let last_record = match self.value() {
            ComponentValue::VecString2(s) => s.last(),
            _ => None,
        };
        if let Some((tag, msg)) = last_record {
            let class_str = if let Some(class) = class {
                class.get(tag)
            } else {
                None
            };
            if let Some(class_str) = class_str {
                let class_str = class_str.clone();
                let mut class_strs = class_str.split('>');
                let class_parent = class_strs.next().unwrap(); 
                let class_str = class_strs.next().unwrap(); 
                format!(r#"<div class="{class_parent}"><div class="{class_str}">{msg}</div></div>"#)
            } else {
                format!(r#"<div><div>{msg}</div></div>"#)
            }
        } else {
            "".to_string()
        }

    }
}

pub async fn append_chatbox_value(
    comp: Arc<RwLock<Box<dyn ComponentBaseTrait<'static>>>>,
    tag_msg: (String, String)
) {
    tracing::info!(target: "tron_app", "append_chatbox_value");
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::ChatBox);
    if let ComponentValue::VecString2(v) = comp.get_mut_value() {
        v.push(tag_msg);
    }
}
