use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tracing::debug;

#[derive(Serialize)]
pub struct TnServerSideTriggerData {
    pub target: String,
    pub new_state: String,
}
#[derive(Serialize)]
pub struct TnSseTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
}

pub async fn send_sse_msg_to_client(tx: &Sender<String>, msg: impl Serialize) {
    let json_string = serde_json::to_string(&msg).unwrap();
    if tx.send(json_string).await.is_err() {
        debug!("tx dropped");
    }
}

pub fn html_escape_double_quote(input: &str) -> String {
    let mut output = String::new();
    for c in input.chars() {
        if c == '"' {
            output.push_str("&quot;");
        } else {
            output.push(c)
        }
    }
    output
}
