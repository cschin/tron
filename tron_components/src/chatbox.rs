use super::*;
use tron_macro::*;
use tron_utils::{send_sse_msg_to_client, TnServerEventData, TnSseTriggerMsg};

/// A component representing a chat box.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnChatBox<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl TnChatBoxBuilder<'static> {
    /// Creates a new instance of `TnChatBox`.
    ///
    /// # Arguments
    ///
    /// * `idx` - The unique identifier of the chat box.
    /// * `tnid` - The name of the chat box.
    /// * `value` - Initial messages to be displayed in the chat box, each message consists of a tuple containing the sender and the message content.
    ///
    /// # Returns
    ///
    /// A new instance of `TnChatBox`.
    pub fn init(mut self,  tnid: String, value: Vec<(String, String)>) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("div".into(), tnid, TnComponentType::ChatBox)
            .set_value(TnComponentValue::VecString2(value))
            .set_attribute("hx-trigger", "server_event")
            .set_attribute(
                "hx-swap",
                "beforeend scroll:bottom focus-scroll:true ",
            )
            .set_attribute("class", "flex-col")
            .create_assets()
            .build();
        let class = HashMap::from_iter(vec![
            // ("user".to_string(), "max-w-fill flex flex-row justify-end p-1 > bg-green-100 rounded-lg p-2 mb-1 text-right".to_string()),
            // ("bot".to_string(), "max-w-fill flex flex-row justify-start p-1 > bg-blue-100 rounded-lg p-2 mb-1 text-left".to_string()),
            (
                "user".to_string(),
                "chat chat-end > bg-green-900 chat-bubble".to_string(),
            ),
            (
                "bot".to_string(),
                "chat chat-start > bg-blue-900 chat-bubble".to_string(),
            ),
        ]);
        let assets: &mut HashMap<String, TnAsset> = self.base.asset.as_mut().unwrap();
        assets.insert("class".into(), TnAsset::HashMapString(class));

        self
    }
}


impl Default for TnChatBox<'static> {
    /// Creates a default instance of `TnChatBox` with an empty message list.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::VecString2(vec![]),
                ..Default::default()
            },
        }
    }
}

#[async_trait]
impl<'a> TnComponentRenderTrait<'a> for TnChatBox<'a>
where
    'a: 'static,
{
    /// Renders the chat box component for the first time.
    async fn first_render(&self) -> String {
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };

        // Extract tag-message pairs from the component's value and generate HTML for chat records
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

        // Format the complete HTML for the chat box
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            chat_recorders,
            self.base.tag
        )
    }

    /// Renders the chat box component.
    async fn render(&self) -> String {
        // Retrieve the class attribute from the component's assets
        let class = if let TnAsset::HashMapString(class) =
            self.get_assets().unwrap().get("class").unwrap()
        {
            Some(class)
        } else {
            None
        };

        // Extract the last record from the component's value and generate HTML for it
        let last_record = match self.value() {
            TnComponentValue::VecString2(s) => s.last(),
            _ => None,
        };

        // Generate HTML for the last chat record
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

    async fn pre_render(&mut self) {}

    async fn post_render(&mut self) {}
}

/// Appends a new tag-message pair to the chat box component's value.
pub async fn append_chatbox_value(
    comp: Arc<RwLock<Box<dyn TnComponentBaseRenderTrait<'static>>>>,
    tag_msg: (String, String),
) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::ChatBox);
    if let TnComponentValue::VecString2(v) = comp.get_mut_value() {
        v.push(tag_msg);
    }
}

/// Cleans the chat box component's transcript and triggers an update.
///
/// This function removes the transcript content of the chat box component identified by `tron_id`
/// within the given `context`. It then triggers a server-side update by setting the component's state
/// to `Ready` and sending a server-side trigger message via Server-Sent Events (SSE).
/// Once the update is triggered, the content of the chat box will be empty.
///
/// # Arguments
///
/// * `context` - The context containing the chat box component.
/// * `tron_id` - The unique identifier of the chat box component to be cleaned.
///
/// # Panics
///
/// This function will panic if:
/// - The chat box component's type is not `ChatBox`.
/// - Unable to acquire a write lock for the chat box component.
///
pub async fn clean_chatbox_with_context(context: &TnContext, tron_id: &str) {
    let sse_tx = context.get_sse_tx().await;
    {
        // remove the transcript in the chatbox component, and sent the hx-reswap to innerHTML
        // once the server side trigger for an update, the content will be empty
        // the hx-reswap will be removed when there is new text in append_chatbox_value()
        assert!(
            context.get_component(tron_id).await.read().await.get_type()
                == TnComponentType::ChatBox
        );

        context
            .set_value_for_component(tron_id, TnComponentValue::VecString2(vec![]))
            .await;
        let comp = context.get_component(tron_id).await;
        assert!(comp.read().await.get_type() == TnComponentType::ChatBox);
        {
            let mut guard = comp.write().await;
            guard.set_state(TnComponentState::Ready);
            guard.set_header("hx-reswap", ("innerHTML".into(), true));

            let msg = TnSseTriggerMsg {
                server_event_data: TnServerEventData {
                    target: tron_id.into(),
                    new_state: "ready".into(),
                },
            };

            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    }
}
