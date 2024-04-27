
use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tracing::debug;


#[derive(Serialize)]
pub struct TriggerData {
    pub target: String,
    pub new_state: String,
}
#[derive(Serialize)]
pub struct SseTriggerMsg {
    pub server_side_trigger: TriggerData,
}

#[derive(Serialize)]
pub struct SseAudioRecorderTriggerMsg {
    pub server_side_trigger: TriggerData,
    pub audio_recorder_control: String,
}

#[derive(Serialize)]
pub struct SseAudioPlayerTriggerMsg {
    pub server_side_trigger: TriggerData,
    pub audio_player_control: String,
}

pub async fn send_sse_msg_to_client(tx: &Sender<String>, data: impl Serialize) {
    let json_string = serde_json::to_string(&data).unwrap();
    if tx.send(json_string).await.is_err() {
        debug!("tx dropped");
    }
}
