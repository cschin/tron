use serde::Serialize;
use tokio::sync::mpsc::Sender;
use tracing::debug;

/// Represents data for triggering server-side events in Tron.
///
/// This struct is used to encapsulate data required for triggering server-side events in Tron,
/// including the target component ID and the new state to set.
#[derive(Serialize)]
pub struct TnServerSideTriggerData {
    pub target: String,
    pub new_state: String,
}

/// Represents a message for triggering server-side events in Tron.
///
/// This struct is used to encapsulate a message for triggering server-side events in Tron.
/// It contains a field `server_side_trigger_data` of type `TnServerSideTriggerData`,
/// which holds the data required for triggering the event.

#[derive(Serialize)]
pub struct TnSseTriggerMsg {
    pub server_side_trigger_data: TnServerSideTriggerData,
}

/// Sends a server-sent event (SSE) message to the client.
///
/// This function serializes the provided message using serde_json and sends it to the client
/// via the provided sender channel (`tx`). It returns immediately after sending the message.
///
/// # Arguments
///
/// * `tx` - A reference to the sender channel for sending SSE messages to the client.
/// * `msg` - A message to be sent to the client, which must implement the `Serialize` trait.
///
/// # Examples
///
/// ```
/// // Usage example:
/// let tx = some_sender_channel.clone();
/// let msg = SomeMessage {
///     // Message fields...
/// };
/// send_sse_msg_to_client(&tx, msg).await;
/// ```
pub async fn send_sse_msg_to_client(tx: &Sender<String>, msg: impl Serialize) {
    let json_string = serde_json::to_string(&msg).unwrap();
    if tx.send(json_string).await.is_err() {
        debug!("tx dropped");
    }
}

/// Escapes double quotes in an input string with HTML entity "&quot;".
///
/// This function takes an input string and replaces every occurrence of a double quote character (`"`)
/// with the HTML entity `&quot;`, effectively escaping it for HTML content.
///
/// # Arguments
///
/// * `input` - The input string to be escaped.
///
/// # Returns
///
/// A new string with double quotes escaped using HTML entity "&quot;".
///
/// # Examples
///
/// ```
/// // Usage example:
/// let input = "This is a \"quoted\" string.";
/// let escaped_string = html_escape_double_quote(input);
/// assert_eq!(escaped_string, "This is a &quot;quoted&quot; string.");
/// ```
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
