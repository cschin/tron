use super::*;
use tron_macro::*;

#[derive(ComponentBase)]
pub struct TnAudioRecorder<'a> {
    inner: ComponentBase<'a>,
}

impl<'a> TnAudioRecorder<'a> {
    pub fn new(id: ComponentId, name: String, value: String) -> Self {
        let mut component_base = ComponentBase::new("div".to_string(), id, name);
        component_base.set_value(ComponentValue::String(value));
        component_base.set_attribute("hx-trigger".into(), "streaming, server_side_trigger".into());
        component_base.set_attribute(
            "hx-vals".into(),
            r##"js:{event_data:get_event(event), audio_data: event.detail.audio_data, streaming: event.detail.streaming}"##.into(),
        );
        component_base.assets = Some(HashMap::<String, ComponentAsset>::default());
        component_base.assets.as_mut().unwrap().insert(
            "audio_data".into(),
            ComponentAsset::Bytes(BytesMut::default()),
        );
        Self {
            inner: component_base,
        }
    }
}

impl<'a> Default for TnAudioRecorder<'a> {
    fn default() -> Self {
        Self {
            inner: ComponentBase {
                value: ComponentValue::String("div".to_string()),
                ..Default::default()
            },
        }
    }
}

impl<'a> TnAudioRecorder<'a> {
    pub fn internal_render(&self) -> Html<String> {
        Html::from(format!(
            r##"<{} {}>{}</{}>"##,
            self.inner.tag,
            self.generate_attr_string(),
            match self.value() {
                ComponentValue::String(s) => s,
                _ => "paused",
            },
            self.inner.tag
        ))
    }
}

pub fn append_audio_data(comp: &mut Box<dyn ComponentBaseTrait<'static>>, new_bytes: Bytes) {
    let e = comp
        .get_mut_assets()
        .unwrap()
        .entry("audio_data".into())
        .or_insert(ComponentAsset::Bytes(BytesMut::default()));

    //let e = get_audio_data(comp);
    if let ComponentAsset::Bytes(audio_data) = e {
        (*audio_data).extend_from_slice(&new_bytes);
        println!("new stream data size: {}", (*audio_data).len());
    }
}

pub fn write_audio_data_to_file(comp: &dyn ComponentBaseTrait<'static>) {
    let e = comp
        .get_assets()
        .as_ref()
        .unwrap()
        .get("audio_data")
        .unwrap();

    if let ComponentAsset::Bytes(audio_data) = e {
        let uid = uuid::Uuid::new_v4();
        let filename = std::path::Path::new("output").join(format!("{}.webm", uid));
        let mut file = std::fs::File::create(filename.clone()).unwrap();
        std::io::Write::write_all(&mut file, audio_data).unwrap();
    }
}

pub fn clear_audio_data<'a>(comp: &'a mut Box<dyn ComponentBaseTrait<'a>>) {
    let e = comp
        .get_mut_assets()
        .unwrap()
        .entry("audio_data".into())
        .or_insert(ComponentAsset::Bytes(BytesMut::default()));

    if let ComponentAsset::Bytes(audio_data) = e {
        (*audio_data).clear();
    }
}
