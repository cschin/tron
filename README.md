# Tron - A Rust Backend Framework for Interactive Web Apps

`Tron` is a modern Rust backend framework designed to simplify building interactive Single Page Applications (SPAs). It seamlessly integrates the Rust asynchronous runtime with [HTMX](https://htmx.org), allowing developers to write backend Rust code that defines UI components, handles events, and manages interaction between components. The framework minimizes the need for frontend JavaScript programming, offering a streamlined development experience for Rust enthusiasts.

## Key Features

- **Component-Based Development**: Define UI components in Rust and handle their events efficiently.
- **Event Processing**: Process HTMX-generated events using Rust functions.
- **Asynchronous Runtime**: Utilize the Rust asynchronous runtime for services, streams, and third-party API interactions.
- **Single Page Application (SPA)**: Create interactive SPAs with minimal JavaScript.

## Getting Started

### Prerequisites

- Rust (latest stable version)
- Cargo (Rust package manager)

### Installation

1. clone the github repository

```
git clone https://github.com/cschin/tron.git
```

2. Add `tron_app` to your `Cargo.toml` along with other common dependencies. For example:

```toml
[dependencies]
tron_app = [{path = "path/to/tron/tron_app"}]
axum = "0.7.5"
serde = { version = "1.0.197", features = ["derive"] }
tokio = { version = "1.37.0", features = ["full"] }
tracing = "0.1.40"
serde_json = "1.0.115"
futures-util = "0.3.30"
askama = "0.12.1"

```

### Examples

See the [examples](https://github.com/cschin/tron/tree/main/examples) directory for more examples.

### Basic Application Template

Here's a template to get you started with `Tron`. This example showcases how to define the main structure of your Tron application and includes placeholders for building the application context, layout, and actions.

```rust
#![allow(dead_code)]
#![allow(unused_imports)]

use askama::Template;
use futures_util::Future;
use axum::extract::Json;
use tokio::sync::{mpsc::Sender, RwLock};
use serde_json::Value;
use tracing::debug;
use tron_components::{
    text::TnTextInput, TnButton, TnComponentBaseTrait, TnComponentState, TnComponentValue,
    TnContext, TnContextBase, TnEvent, TnEventActions, TnTextArea,
};
use std::{collections::HashMap, pin::Pin, sync::Arc};

// The main entry point of the application
#[tokio::main]
async fn main() {
    let app_config = tron_app::AppConfigure::default();
    // Set up shared application state
    let app_share_data = tron_app::AppData {
        context: RwLock::new(HashMap::default()),
        session_expiry: RwLock::new(HashMap::default()), 
        event_actions: RwLock::new(TnEventActions::default()),
        build_context: Arc::new(Box::new(build_context)),
        build_actions: Arc::new(Box::new(build_actions)),
        build_layout: Arc::new(Box::new(layout)),
    };
    tron_app::run(app_share_data, app_config).await
}

// Functions for building the application context, layout, and event actions

fn build_context() -> TnContext {
    let context = Arc::new(RwLock::new(TnContextBase::default()));
    TnContext { base: context }
}

fn layout(context: TnContext) -> String {
    // Define your application layout and components
    "This is a template, please fill in the components and how to layout them.".into()
}

fn build_actions(context: TnContext) -> TnEventActions {
    let actions = TnEventActions::default();
    // Add your event actions here
    actions
}

// Example placeholder for handling events
fn test_event_action(
    context: TnContext,
    tx: Sender<Json<Value>>,
    event: TnEvent,
) -> Pin<Box<dyn Future<Output = ()> + Send + Sync>> {
    todo!()
}
```

See the [01_widgets](https://github.com/cschin/tron/tree/main/examples/01_widgets) directory for adding components and set up the action and layout.

If you clone the repo, you can run the examples using 
```
cargo run --bin tron_widgets # show a number of widgets
cargo run --bin conversational_bot # a conversational bot example
cargo run --bin cognito_login # a cognito login example
```

### License

This project is licensed under the MIT License - see the [LICENSE](https://github.com/example/tron/blob/main/LICENSE) file for details.
