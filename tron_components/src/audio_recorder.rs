use super::*;
use serde::Serialize;
use tron_macro::*;
use tron_utils::TnServerSideTriggerData;

/// Represents a server-sent event (SSE) message for controlling an audio recorder component.
#[derive(Serialize)]
pub struct SseAudioRecorderTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
    pub audio_recorder_control: String,
}

/// Represents an audio recorder component.
#[non_exhaustive]
#[derive(ComponentBase)]
pub struct TnAudioRecorder<'a: 'static> {
    base: TnComponentBase<'a>,
}
impl TnAudioRecorderBuilder<'static> {
    /// Creates a new `TnAudioRecorder` component with the specified index, name, and value.
    ///
    /// # Arguments
    ///
    /// * `idx` - The unique index of the component.
    /// * `tnid` - The name of the component.
    /// * `value` - The initial value of the component.
    ///
    /// # Returns
    ///
    /// A new instance of `TnAudioRecorder`.
    pub fn init(mut self, idx: TnComponentIndex, tnid: String, value: String) -> Self {
        self.base = TnComponentBase::builder(self.base)
            .init("div".to_string(), idx, tnid, TnComponentType::AudioRecorder)
            .set_value(TnComponentValue::String(value))
            .set_attribute("hx-trigger".into(), "streaming, server_side_trigger".into())
            .set_attribute(
                "hx-vals".into(),
                r##"js:{event_data:get_audio_event(event)}"##.into(),
            )
            .build();
        self.base.asset = Some(HashMap::<String, TnAsset>::default());
        self.base
            .asset
            .as_mut()
            .unwrap()
            .insert("audio_data".into(), TnAsset::Bytes(BytesMut::default()));
        self
    }
}

impl Default for TnAudioRecorder<'static> {
    /// Creates a default instance of `TnAudioRecorder`.
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("div".to_string()),
                ..Default::default()
            },
        }
    }
}

impl TnAudioRecorder<'static> {
    /// Generates the internal HTML representation of the audio recorder component.
    pub fn internal_render(&self) -> String {
        format!(
            r##"<{} {}>{}</{}>"##,
            self.base.tag,
            self.generate_attr_string(),
            match self.value() {
                TnComponentValue::String(s) => &s,
                _ => "paused",
            },
            self.base.tag
        )
    }

    /// Generates the initial HTML representation of the audio recorder component.
    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }

    pub fn internal_pre_render(&mut self) {}

    pub fn internal_post_render(&mut self) {}
}

/// Appends new audio data to the specified audio recorder component.
///
/// # Arguments
///
/// * `comp` - A reference to the audio recorder component.
/// * `new_bytes` - The new audio data to be appended.
///
/// # Panics
///
/// Panics if the component is not of type `TnComponentType::AudioRecorder`.
///
pub async fn append_audio_data(comp: TnComponent<'static>, new_bytes: Bytes) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::AudioRecorder);
    let e = comp
        .get_mut_assets()
        .unwrap()
        .entry("audio_data".into())
        .or_insert(TnAsset::Bytes(BytesMut::default()));
    if let TnAsset::Bytes(audio_data) = e {
        (*audio_data).extend_from_slice(&new_bytes);
    }
}

/// Clears the audio data stored in the specified audio recorder component.
///
/// # Arguments
///
/// * `comp` - A reference to the audio recorder component.
///
/// # Panics
///
/// Panics if the component is not of type `TnComponentType::AudioRecorder`.
///
pub async fn clear_audio_data(comp: TnComponent<'static>) {
    let mut comp = comp.write().await;
    assert!(comp.get_type() == TnComponentType::AudioRecorder);
    let e = comp
        .get_mut_assets()
        .unwrap()
        .entry("audio_data".into())
        .or_insert(TnAsset::Bytes(BytesMut::default()));
    if let TnAsset::Bytes(audio_data) = e {
        (*audio_data).clear();
    }
}

/// Writes the audio data stored in the specified audio recorder component to a file.
///
/// The audio data is stored in the component's assets under the key "audio_data".
///
/// # Arguments
///
/// * `comp` - A reference to the audio recorder component.
///
/// # Panics
///
/// Panics if the component is not of type `TnComponentType::AudioRecorder`.
///
pub async fn write_audio_data_to_file(comp: TnComponent<'static>) {
    let comp = comp.read().await;
    assert!(comp.get_type() == TnComponentType::AudioRecorder);
    let e = comp
        .get_assets()
        .as_ref()
        .unwrap()
        .get("audio_data")
        .unwrap();

    if let TnAsset::Bytes(audio_data) = e {
        let uid = uuid::Uuid::new_v4();
        let filename = std::path::Path::new("output").join(format!("{}.webm", uid));
        let mut file = std::fs::File::create(filename.clone()).unwrap();
        std::io::Write::write_all(&mut file, audio_data).unwrap();
    }
}
