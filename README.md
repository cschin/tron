# Tron - A Rust Backend Framework for Interactive Web Apps

`Tron` is an experimental Rust backend framework designed to simplify building interactive Single Page Applications (SPAs). It seamlessly integrates the Rust asynchronous runtime with [HTMX](https://htmx.org), allowing developers to write backend Rust code that defines UI components, handles events, and manages interaction between components. The framework minimizes the need for frontend JavaScript programming, offering a streamlined development experience for Rust enthusiasts.

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

This project is licensed under the MIT License - see the [LICENSE](https://github.com/cschin/tron/blob/main/LICENSE) file for details.



## An experiment using Rust + HTMX for SPA (Single Page Application)

### Introduction

A while ago, I helped a friend build a conversational bot incorporating Automatic Speech Recognition (ASR), a large language model, and Text to Speech (TTS) for a prototype and demonstration for some startup ideas. Although I grew up during the [NCSA browser era](https://en.wikipedia.org/wiki/Mosaic_(web_browser)) and developed a research data analytics web platform for DNA sequencing R&D, I later shifted most of my focus to algorithms and scientific applications in bioinformatics and machine learning. I had missed how the web technology evolved into multimedia/streaming platforms. Fortunately, after studying [MDN documentation and some examples](https://developer.mozilla.org/en-US/docs/Web/API/Media_Capture_and_Streams_API), I managed to handle media input/output from a webpage using just a little JavaScript.

Then I built an "MVP" (Minimum Viable Prototype) using the [Gradio](https://www.gradio.app) library from Hugging Face in just a few days. Gradio’s key feature is its integration of frontend and server-side code within the library. Developers simply write straightforward Python code to define the inputs and outputs of a UI component and program responses to UI events using Python functions. This simplicity is ideal for ML developers to showcase their work interactively. Although I encountered challenges with "real-time" applications, like streaming audio and detecting microphone inactivity, some clever hacks helped me overcome these issues. The elegant UI of Gradio greatly enhanced the MVP, impressing many. I am thankful to the Gradio development team for their excellent work.

Meanwhile, I was thinking “what if I can just write Rust code for an SPA?” 

I have experimented with several Rust GUI frameworks such as [Dioxus](https://dioxuslabs.com), [Leptos](https://www.leptos.dev), and [Egui](https://github.com/emilk/egui). While each is impressive for its intended purpose, they also require adopting new paradigms of UI programming. Meanwhile, creating UI interfaces in Gradio is way simpler than coding each component from scratch. I am curious about how closely I can mimic Gradio's design using Rust with somewhat minimal effort. Note that Gradio, developed over several years, is feature-rich, and replicating its capabilities in Rust could be a lengthy process. Gradio uses Servlets for UI components and FastAPI for backend HTTP request processing. To emulate my experience with Gradio in Rust, I tried using [HTMX](https://htmx.org) on the frontend. It is simple and easy to learn. With HTMX, I can perform server-side rendering of UI components using Rust code and a template library to generate HTML directly in response to HTMX’s "old school XMLHttpRequest." 😊 We can serve the UI components and handle interactions between them using Rust with the Axum web framework to manage the HTTP requests from HTMX.

I spent two weeks writing the ["Tron" framework](https://github.com/cschin/tron). While it is not "production-ready", it is a great experiment for me to learn something new. By constructing it, I became more familiar with web programming using Rust, and better understood the Tokio runtime and asynchronous programming. However, the code in Rust isn't as concise as the Python code for Gradio due to Rust's static type system. I also didn't merely replicate the input/output model from Gradio. Currently, an "action" function (a future in the Tokio runtime) can access the full context of an app, interact with data belonging to all components, and manage other additional assets or states of the app. This flexibility allows for more dynamic designs, though developers must be cautious not to misuse such broad exposure. One advantage of a Rust backend is that it can easily integrate with any backend service written in Rust within the codebase. The strong typing and borrow checker aid in achieving "fearless concurrency."

### An simple example

A Tron App typically need to set a couple of logical code blocks and chain them together.

- import the dependencies 
- define the main function to configure the app
- define a function for specifying the components in the SPA
- define a function to generate the initial HTML layout
- define a function for setting up the actions 
- define a set of function of the actions and the services

The `main` function is the entry point of the app. It is responsible for configuring the app and setting up the components. This is boilerplate code for setting up the app. It looks like this:

```
// This is the main entry point of the application
// It sets up the application configuration and state
// and then starts the application by calling tron_app::run
#[tokio::main]
async fn main() {
    let app_config = tron_app::AppConfigure {
        http_only: true,
        ..Default::default()
    };
    // set app state
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
```

It set up the `context` member which contains the app state and the `event_actions` member that tells the tron_app runtime how to process the events. Then we need to pass three functions `build_context`, `build_actions` and `layout` in the `app_share_data`;

#### The build_context function

A developer needs to define the `fn build_context()` to create the `context` which contains a set of components first. For example, the following code creates a `button` component and return a context with it.

```
static BUTTON: &str = "button";

// These functions are used to build the application context,
// layout, and event actions respectively
fn build_context() -> TnContext {
    let mut context = TnContextBase::default();

    let component_index = 0;
    let mut btn = TnButton::new(component_index, BUTTON.into(), "click me".into());
    btn.set_attribute(
        "class".to_string(),
        "btn btn-sm btn-outline btn-primary flex-1".to_string(),
    );

    btn.set_attribute("hx-target".to_string(), "#count".to_string());
    btn.set_attribute("hx-swap".to_string(), "innerHTML".to_string());
    context.asset.blocking_write().insert("count".into(), TnAsset::U32(0));

    context.add_component(btn);

    TnContext {
        base: Arc::new(RwLock::new(context)),
    }
}
```

Note that we set a number of attributes to the `button` component. The `hx-target` attribute is used to specify the target element to be updated. The `hx-swap` attribute is used to specify how the target element should be updated. It is default to `outerHTML`. However, in this case, we want to update the innerHTML of the target element.

We also create a component level asset. This asset is used to store the number of times the button has been clicked.

#### Layout 

The `layout` function should generate a `String` that represents the initial HTML layout of the app, sent to the browser to render when the page loads. In this example, we use the [`askama`](https://github.com/djc/askama) template library to assist in generating the HTML layout. Unlike Gradio, developers need to know HTML to create layouts with components defined in the `build_context` function. This is certainly more tedious than Gradio but offers more flexibility and control over the layout. Here's a simple example of a `layout` function for a button and a `div` element that displays a counter for how many times the button is clicked:

```
#[derive(Template)] // this will generate the code...
#[template(path = "app_page.html", escape = "none")] // using the template in this path, relative                                    // to the `templates` dir in the crate root
struct AppPageTemplate {
    button: String,
}

fn layout(context: TnContext) -> String {
    let context_guard = context.blocking_read();
    let button =  context_guard.render_to_string(BUTTON);
    let html = AppPageTemplate {
        button,
    };
    html.render().unwrap()
}
```

The `app_page.html` file is a template file which contains HTML code. It looks like this defined where to render the `button` component and defined the `div` to show the counter and the CSS class for the style and layout:

```
<div class="container mx-auto px-4">
    <div class="flex flex-row p-1">
        <div class="flex flex-row p-1 basic-2">
            {{button}}
            <div id="count" class="flex p-1">0</div>
        </div>
    </div>
</div>
```

#### Action Function

Our simple goal here is for the counter to increase by one each time a user clicks the button. We can achieve this by creating an action, which is a function called upon the button's click. Here's what a simple action function looks like: it returns a "pinned future" so the Tron app can use it to respond to the HTTP request. The "future" takes `context`, `event`, and `payload` (additional configurable data, e.g., client-side states, values) and generates an `Option` of `TnHtmlResponse`. The `TnHtmlResponse` encapsulates the HTTP response header and the HTML body, allowing a developer to use customized headers to control the [HTMX behavior](https://htmx.org/reference/#response_headers) after receiving the response.


```

fn button_clicked(
    context: TnContext,
    event: TnEvent,
    _payload: Value,
) -> Pin<Box<dyn Future<Output = TnHtmlResponse> + Send + Sync>> {
    let action = async move {
        tracing::info!(target: "tron_app", "{:?}", event);
        if event.e_trigger != BUTTON {
            None
        } else {
            let asset_ref = context.get_asset_ref().await;
            let mut asset_guard = asset_ref.write().await;
            let count = asset_guard.get_mut("count").unwrap();
            let new_count = if let TnAsset::U32(count) = count {
                *count += 1;
                *count
            } else {
                0
            };
            Some((HeaderMap::new(), Html::from(format!("count: {new_count}"))))
        }
    };
    Box::pin(action)
}
```

In the code above, we take the `count` stored in the context asset, update it, and then return the new value in the HTTP response. This will update the counter in the browser.

#### Build actions


After defining some actions, we need to connect them to the related component. The `fn button_clicked()` 
should respond when a click event is triggered on the button in the web frontend. We need to inform the Tron
 App about this. This is achieved by defining a `build_action` function, which connects the component to the 
 action.

Our `build_action` is defined as follows:

```
fn build_actions(context: TnContext) -> TnEventActions {
    let mut actions = TnEventActions::default();
    let index = context.blocking_read().get_component_index(BUTTON);
    actions.insert(
        index,
        (TnActionExecutionMethod::Await, Arc::new(button_clicked)),
    );
    actions
}
```

It simply inserts the index of the `BUTTON` component and the associated action into a `TnEventActions` struct and returns it. `TnActionExecutionMethod::Await` means that the action will be awaited until it finishes execution before returning to the main event loop. Alternatively, one can use `TnActionExecutionMethod::Spawn` to spawn a new task for the action. In the `Spawn` scenario, the code returns with default rendering results and ignores the results from the actions. However, the Tron framework features server-side code that can trigger events on client-side components. In this simple example, we do not need the server-side trigger. Nevertheless, it proves very useful when the client-side needs to respond after longer server-side computations or API calls are completed.

#### Put it together

The complete code for this example can be found in the [`examples/04_simple_button_counter`](https://github.com/cschin/tron/tree/main/examples/04_simple_button_counter) directory.

One can run the example under a shell from the root of the repo(assuming the Rust environment is set up):

```
cargo run --example 04_simple_button_counter
```

Then you can point your browser to [http://localhost:3001](http://localhost:3001) to see the result.

![Simple Button Counter](https://github.com/cschin/tron/blob/main/misc/images/simple_button_counter.png?raw=true)


## Other Features and More Examples

To make a conversational bot with other AI services, a backend framework like Tron App library needs additional features beyond basic examples. Here are some enhancements I've implemented:

- **Data Streaming**: Enables media playback in the frontend.
- **Server-Side Triggers through Event Streams**: Allows server-side components to update client-side components without user input, synchronizing client-side rendering with server status.
- **Background Services**: Utilizes server-side rendering to leverage server computing resources. APIs that are unsuitable for client-side calls or require processing can now be handled in Rust, providing services to the app. These services can be spawned and managed through Tokio's channels for intercommunication.

You can try to run the `tron_widgets` example to see a number of currently implemented widgets. Check [the code](https://github.com/cschin/tron/blob/main/examples/01_widgets/src/main.rs) to see how to programmatically create components and handle interactions between components beyond the simple button counter example.

![Some tron widgets](https://github.com/cschin/tron/blob/main/misc/images/widgets.png?raw=true)


Some of the more advanced features are shown in how I created a [conversational bot](https://github.com/cschin/tron/tree/main/examples/02_conversational_bot).

![A conversational bot](https://github.com/cschin/tron/blob/main/misc/images/conversational_bot.png?raw=true)

## What's Next

Looking ahead, the focus will be on enhancing the use of Rust for server-side rendering, aiming to simplify the development process for those who prefer not to invest heavily in JavaScript but still desire an interactive frontend. The Tron App's approach is especially promising for single page or machine learning demonstration where developer expertise may lean more towards backend logic than frontend programming.

By leveraging Rust's performance and security features, developers can efficiently handle server-side computations and state management while minimizing client-side code. This not only reduces the complexity and increases the reliability of web applications but also opens up new possibilities for integrating advanced machine learning models directly into web interfaces without the usual overhead.

Although Rust is more verbose and has a steep learning curve, it may still be worth investing in for its scalability to more complex systems. The strong static typing system and the borrow checker, along with excellent asynchronous runtime for performance, help scale up projects and avoid common programming errors, making it a robust choice for serious development work for a larger system.

I plan to continue to develop this especially if I have a need for a single-page application. If any readers find this useful or want to explore similar ideas for a better framework, I would be eager to connect with more friends who are also interested in this field. Your feedback and collaboration could greatly enrich this project and the community.

May 18, 2024