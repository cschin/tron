pub mod tron_app;

use serde::Serialize;
pub use tron_app::*;

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

pub mod utils {
    use axum::Json;
    use serde::Serialize;
    use serde_json::Value;
    use tokio::sync::mpsc::Sender;
    use tracing::debug;


    pub async fn send_sse_msg_to_client(tx: &Sender<Json<Value>>, data: impl Serialize) {
        let json_value = serde_json::to_value(data).unwrap();
        if tx.send(axum::Json(json_value)).await.is_err() {
            debug!("tx dropped");
        }
    }
}
