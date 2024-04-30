use super::*;
use futures_util::Future;
use serde::Serialize;
use tron_macro::*;
use tron_utils::*;

#[derive(Serialize)]
pub struct SseAudioPlayerTriggerMsg {
    pub server_side_trigger: TriggerData,
    pub audio_player_control: String,
}

#[derive(ComponentBase)]
pub struct TnAudioPlayer<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnAudioPlayer<'a> {
    pub fn new(id: TnComponentId, name: String, value: String) -> Self {
        let mut base = TnComponentBase::new(
            "audio".to_string(),
            id,
            name.clone(),
            TnComponentType::AudioPlayer,
        );
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("src".into(), format!("/tron_streaming/{}", name));
        // component_base.set_attribute("type".into(), "audio/webm".into());
        base.set_attribute("type".into(), "audio/mp3".into());
        base.set_attribute("hx-trigger".into(), "server_side_trigger, ended".into());
        Self { base }
    }
}

impl<'a: 'static> Default for TnAudioPlayer<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("audio".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnAudioPlayer<'a>
where
    'a: 'static,
{
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} controls autoplay>"##,
            self.base.tag,
            self.generate_attr_string(),
        )
    }

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

pub async fn start_audio(comp: TnComponent<'static>, sse_tx: Sender<String>) {
    {
        let mut comp = comp.write().await;
        assert!(comp.get_type() == TnComponentType::AudioPlayer);
        // HX-Reswap was set to "none" when the audio play stop, need to remove it to play audio
        comp.remove_header("HX-Reswap".into()); 
        comp.set_state(TnComponentState::Updating);
    }

    let comp = comp.read().await;
    let msg = SseTriggerMsg {
        server_side_trigger: TriggerData {
            target: comp.tron_id().clone(),
            new_state: "updating".into(),
        },
    };
    send_sse_msg_to_client(&sse_tx, msg).await;
}

pub fn stop_audio_playing_action(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    let f = async move {
        {
            let guard = context.get_component(&event.e_target.clone()).await;
            let mut player = guard.write().await;
            // we don't want to swap the element, or it will replay the audio. the "false" make the header persist until next play event
            player.set_header("HX-Reswap".into(), ("none".into(), false)); 
            player.set_state(TnComponentState::Ready);
        }
        {
            let sse_tx = context.get_sse_tx_with_context().await;
            let msg = SseTriggerMsg {
                server_side_trigger: TriggerData {
                    target: event.e_target.clone(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
    };
    Box::pin(f)
}
