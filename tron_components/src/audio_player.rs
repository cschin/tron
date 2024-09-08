use std::str::FromStr;

use super::*;
use axum::http::{HeaderName, HeaderValue};
use futures_util::Future;
use serde::Serialize;
use tron_macro::*;
use tron_utils::*;

/// Represents a message used to trigger audio player actions in Server-Sent Events (SSE).
///
/// This struct contains data necessary to trigger actions related to audio playback
/// in Server-Sent Events. It includes server-side trigger data (`TnServerSideTriggerData`)
/// and an audio player control command.
#[derive(Serialize)]
pub struct SseAudioPlayerTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
    pub audio_player_control: String,
}

/// Represents an audio player component in the application.
///
/// This component provides functionality for playing audio content within the application.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnAudioPlayer<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnAudioPlayerBuilder<'a> {
    /// Creates a new instance of `TnAudioPlayer` with the specified index, ID, and audio source URL.
    ///
    /// # Arguments
    ///
    /// * `idx` - The unique index of the audio player component.
    /// * `tnid` - The ID of the audio player component.
    /// * `value` - The URL of the audio source.
    ///
    /// # Returns
    ///
    /// A new instance of `TnAudioPlayer`.
    pub fn init(mut self, idx: TnComponentIndex, tnid: String, value: String) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init(
                "audio".to_string(),
                idx,
                tnid.clone(),
                TnComponentType::AudioPlayer,
            )
            .set_value(TnComponentValue::String(value))
            .set_attribute("src".into(), format!("/tron_streaming/{}", tnid))
            .set_attribute("type".into(), "audio/mp3".into())
            .set_attribute("hx-trigger".into(), "server_side_trigger, ended".into()).build();
        self
    }
}

impl<'a: 'static> Default for TnAudioPlayer<'a> {
    /// Returns the default instance of `TnAudioPlayer`.
    ///
    /// The default instance has the component type set to "audio" and an empty value.
    ///
    /// # Returns
    ///
    /// The default instance of `TnAudioPlayer`.
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
    /// Generates the HTML string representation of the audio player component with its attributes.
    ///
    /// This method returns an HTML string representing the audio player component
    /// with its attributes. The generated string includes the opening audio tag
    /// along with any attributes set for the component.
    ///
    /// # Returns
    ///
    /// A string containing the HTML representation of the audio player component.

    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {} controls autoplay>"##,
            self.base.tag,
            self.generate_attr_string(),
        )
    }

    /// Generates the initial HTML string representation of the audio player component.
    ///
    /// This method behaves the same as `internal_render`, generating the HTML string
    /// representation of the audio player component with its attributes. It is provided
    /// as a convenience method with the same behavior as `internal_render`.
    ///
    /// # Returns
    ///
    /// A string containing the initial HTML representation of the audio player component.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Asynchronously starts playing audio associated with a given component and sends an SSE message.
///
/// This function asynchronously starts playing audio associated with the provided component.
/// It removes the "HX-Reswap" header to enable audio playback and sets the component's state
/// to "updating". Then, it sends an SSE message to notify clients about the state change.
///
/// # Arguments
///
/// * `comp` - A reference to the component to start playing audio.
/// * `sse_tx` - A sender channel for sending SSE messages.
///
/// # Panics
///
/// This function will panic if the component's type is not `TnComponentType::AudioPlayer`.
///
pub async fn start_audio(comp: TnComponent<'static>, sse_tx: Sender<String>) {
    {
        let mut comp = comp.write().await;
        assert!(comp.get_type() == TnComponentType::AudioPlayer);
        // HX-Reswap was set to "none" when the audio play stop, need to remove it to play audio
        comp.remove_header("HX-Reswap".into());
        comp.set_state(TnComponentState::Updating);
    }

    let comp = comp.read().await;
    let msg = TnSseTriggerMsg {
        server_side_trigger_data: TnServerSideTriggerData {
            target: comp.tron_id().clone(),
            new_state: "updating".into(),
        },
    };
    send_sse_msg_to_client(&sse_tx, msg).await;
}

/// Defines an action to stop audio playback when the "ended" event is triggered.
///
/// This function defines an action to stop audio playback when the "ended" event is triggered.
/// It checks if the event type is "ended" and the event state is "updating".
/// If the conditions are met, it removes the "HX-Reswap" header to prevent the audio from replaying
/// and sets the component's state to "ready". Then, it sends an SSE message to notify clients
/// about the state change and returns HTML content and headers.
///
/// # Arguments
///
/// * `context` - The context containing the component and SSE sender.
/// * `event` - The event triggering the action.
/// * `_payload` - Additional payload data (unused in this function).
///
/// # Returns
///
/// A future that resolves to a tuple containing optional headers and HTML content.
///
pub fn stop_audio_playing_action(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let f = async move {
        if event.e_type != "ended" || event.e_state != "updating" {
            return None;
        }
        {
            let guard = context.get_component(&event.e_trigger.clone()).await;
            let mut player = guard.write().await;
            // we don't want to swap the element, or it will replay the audio. the "false" make the header persist until next play event
            player.set_header("HX-Reswap".into(), ("none".into(), false));
            player.set_state(TnComponentState::Ready);
        }
        {
            let sse_tx = context.get_sse_tx().await;
            let msg = TnSseTriggerMsg {
                server_side_trigger_data: TnServerSideTriggerData {
                    target: event.e_trigger.clone(),
                    new_state: "ready".into(),
                },
            };
            send_sse_msg_to_client(&sse_tx, msg).await;
        }
        let html = context.render_component(&event.h_target.unwrap()).await;
        let mut header = HeaderMap::new();
        header.insert(
            HeaderName::from_str("HX-Reswap").unwrap(),
            HeaderValue::from_str("none").unwrap(),
        );
        Some((header, Html::from(html)))
    };
    Box::pin(f)
}

pub async fn stop_audio(comp: TnComponent<'static>, sse_tx: Sender<String>) {
    {
        let mut comp = comp.write().await;
        assert!(comp.get_type() == TnComponentType::AudioPlayer);
        // HX-Reswap was set to "none" when the audio play stop, need to remove it to play audio
        comp.remove_header("HX-Reswap".into());
        comp.set_state(TnComponentState::Ready);
    }

    let comp = comp.read().await;
    let msg = SseAudioPlayerTriggerMsg {
        server_side_trigger_data: TnServerSideTriggerData {
            target: comp.tron_id().clone(),
            new_state: "ready".into(),
        },
        audio_player_control: "stop".into(),
    };
    send_sse_msg_to_client(&sse_tx, msg).await;
}
