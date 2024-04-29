use super::*;
use serde::Serialize;
use tron_macro::*;
use tron_utils::TriggerData;
#[derive(Serialize)]
pub struct SseAudioRecorderTriggerMsg {
    pub server_side_trigger: TriggerData,
    pub audio_recorder_control: String,
}

#[derive(ComponentBase)]
pub struct TnAudioRecorder<'a: 'static> {
    base: TnComponentBase<'a>,
}

impl<'a: 'static> TnAudioRecorder<'a> {
    pub fn new(id: TnComponentId, name: String, value: String) -> Self {
        let mut base =
            TnComponentBase::new("div".to_string(), id, name, TnComponentType::AudioRecorder);
        base.set_value(TnComponentValue::String(value));
        base.set_attribute("hx-trigger".into(), "streaming, server_side_trigger".into());
        base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_event(event), audio_data: event.detail.audio_data, streaming: event.detail.streaming}"##.into(),
        );
        base.asset = Some(HashMap::<String, TnAsset>::default());
        base.asset
            .as_mut()
            .unwrap()
            .insert("audio_data".into(), TnAsset::Bytes(BytesMut::default()));
        base.script = Some(include_str!("../javascript/audio_recorder.html").to_string());
        Self { base }
    }
}

impl<'a: 'static> Default for TnAudioRecorder<'a> {
    fn default() -> Self {
        Self {
            base: TnComponentBase {
                value: TnComponentValue::String("div".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a: 'static> TnAudioRecorder<'a>
where
    'a: 'static,
{
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

    pub fn internal_first_render(&self) -> String {
        self.internal_render()
    }
}

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
