use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnChatBox<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnChatBox<'a> {
    pub fn new(id: TnComponentId, name: String, value: Vec<(String, String)>) -> Self {
        let mut base =
            TnComponentBase::new("div".into(), id, name, TnComponentType::ChatBox);
        base.set_value(TnComponentValue::VecString2(value));

        base.set_attribute("hx-trigger".into(), "server_side_trigger".into());
        base.set_attribute(
            "hx-swap".into(),
            "beforeend scroll:bottom focus-scroll:true ".into(),
        );
        base.set_attribute(
            "class".into(),
            "flex-col".into(),
        );
        base.asset = Some(HashMap::default());
        let class = HashMap::from_iter(vec![
            // ("user".to_string(), "max-w-fill flex flex-row justify-end p-1 > bg-green-100 rounded-lg p-2 mb-1 text-right".to_string()),
            // ("bot".to_string(), "max-w-fill flex flex-row justify-start p-1 > bg-blue-100 rounded-lg p-2 mb-1 text-left".to_string()),
            ("user".to_string(), "chat chat-end > bg-green-900 chat-bubble".to_string()),
            ("bot".to_string(), "chat chat-start > bg-blue-900 chat-bubble".to_string()),
        ]);
        let assets: &mut HashMap<String, TnAsset> = base.asset.as_mut().unwrap();
        assets.insert("class".into(), TnAsset::HashMapString(class));
        base.script = Some(include_str!("../javascript/chatbox.html").to_string());

        Self {
            base,
        }
    }
}

impl<'a: 'static> Default for TnChatBox<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::VecString2(vec![]),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnChatBox<'a> {
    pub fn internal_first_render(&self) -> String {
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };

        let chat_recorders = if let TnComponentValue::VecString2(tag_msg) = self.value() {
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

        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            chat_recorders,
            self.base.tag
        )
    }

    pub fn internal_render(&self) -> String { 
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };
        let last_record = match self.value() {
            TnComponentValue::VecString2(s) => s.last(),
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
    comp: Arc<RwLock<Box<dyn TnComponentBaseTrait<'static>>>>,
    tag_msg: (String, String)
) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::ChatBox);
    comp.remove_header("hx-reswap".into()); // after reset this is set, remove it for appending the text
    if let TnComponentValue::VecString2(v) = comp.get_mut_value() {
        v.push(tag_msg);
    }
}
