use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tracing::debug;

/// The `TnServerSideTriggerData` struct in Rust represents data for a server-side trigger with target
/// and new state fields.
/// 
/// Properties:
/// 
/// * `target`: The `target` property in the `TnServerSideTriggerData` struct represents the target of
/// the server-side trigger. It is a string type field where you would specify the target entity or
/// object that the trigger is associated with.
/// * `new_state`: The `new_state` property in the `TnServerSideTriggerData` struct represents the new
/// state that will be assigned to the `target` entity when a server-side trigger is activated. It is a
/// string type field where you can specify the new state value for the target entity.
#[derive(Serialize)]
pub struct TnServerSideTriggerData {
    pub target: String,
    pub new_state: String,
}

/// The `TnSseTriggerMsg` struct contains server-side trigger data for use in Rust programming.
/// 
/// Properties:
/// 
/// * `server_side_trigger_data`: The `TnSseTriggerMsg` struct has a single field named
/// `server_side_trigger_data` of type `TnServerSideTriggerData`. This field represents the server-side
/// trigger data associated with the trigger message.

#[derive(Serialize)]
pub struct TnSseTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
}

/// The function `send_sse_msg_to_client` sends a serialized message to a client using a provided
/// sender.
/// 
/// Arguments:
/// 
/// * `tx`: The `tx` parameter is of type `Sender<String>`, which is a channel sender used to send
/// messages of type `String` to the receiver end of the channel.
/// * `msg`: The `msg` parameter in the `send_sse_msg_to_client` function is of type `impl Serialize`,
/// which means it can accept any type that implements the `Serialize` trait from the `serde` library.
/// This trait allows the object to be serialized into a JSON string using `serde_json
pub async fn send_sse_msg_to_client(tx: &Sender<String>, msg: impl Serialize) {
    let json_string = serde_json::to_string(&msg).unwrap();
    if tx.send(json_string).await.is_err() {
        debug!("tx dropped");
    }
}

/// The function `html_escape_double_quote` takes a string input and replaces any double quotes with the
/// HTML entity `&quot;`.
/// 
/// Arguments:
/// 
/// * `input`: "Hello, "world"!"
/// 
/// Returns:
/// 
/// The `html_escape_double_quote` function returns a new `String` with double quotes (`"`) replaced by
/// the HTML entity `&quot;`.
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
