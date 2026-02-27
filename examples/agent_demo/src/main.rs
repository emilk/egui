//! A simple demo application showcasing eframe agent capabilities.
use std::sync::Arc;

mod conversation_view;
mod counter_view;

use conversation_view::ConversationView;
use counter_view::CounterView;
use eframe::AppCreator;
use eframe_agent::agent_ws;
use eframe_agent::{AgentApp, AgentRuntime, AgentViewRegistry, AutomationBridge, ToolLogView};

#[cfg(all(not(target_arch = "wasm32"), feature = "mcp_sse"))]
use eframe_agent::McpSseServer;
#[cfg(not(target_arch = "wasm32"))]
use eframe_agent::{AgentBridge, AgentWsServer, SimpleAgentRuntime};

fn main() -> eframe::Result<()> {
    #[cfg(not(target_arch = "wasm32"))]
    env_logger::init();

    #[cfg(target_arch = "wasm32")]
    wasm_bindgen_futures::spawn_local(run_wasm());

    #[cfg(not(target_arch = "wasm32"))]
    {
        if std::env::var("AGENT_WS_URL").is_ok() {
            let runtime = agent_ws::build_runtime();
            let native_options = eframe::NativeOptions::default();
            return eframe::run_native(
                "Agent demo",
                native_options,
                boxed_app_creator(runtime, None),
            );
        }

        let agent_runtime: Arc<dyn AgentRuntime> = Arc::new(SimpleAgentRuntime::new());
        let bridge = Arc::new(AgentBridge::new(Arc::clone(&agent_runtime)));
        let ws = AgentWsServer::spawn_with_bridge("127.0.0.1:9001", Arc::clone(&bridge))
            .map_err(eframe::Error::AppCreation)?;
        #[cfg(feature = "mcp_sse")]
        let automation = Some(AutomationBridge::new());
        #[cfg(not(feature = "mcp_sse"))]
        let automation: Option<AutomationBridge> = None;
        #[cfg(feature = "mcp_sse")]
        let _sse = McpSseServer::spawn_default_with_automation(
            Arc::clone(&bridge),
            automation.clone().expect("automation configured"),
        )
        .map_err(eframe::Error::AppCreation)?;
        let ui_runtime: Arc<dyn AgentRuntime> =
            Arc::new(agent_ws::AgentWsRuntime::connect(ws.url()));
        let native_options = eframe::NativeOptions::default();
        eframe::run_native(
            "Agent demo",
            native_options,
            boxed_app_creator(ui_runtime, automation),
        )
    }

    #[cfg(target_arch = "wasm32")]
    {
        Ok(())
    }
}

#[cfg(target_arch = "wasm32")]
async fn run_wasm() {
    eframe::WebLogger::init(log::LevelFilter::Debug).ok();

    let runtime = agent_ws::build_runtime();
    let web_options = eframe::WebOptions::default();

    let runner = eframe::WebRunner::new();
    let canvas_id = "the_canvas_id";

    runner
        .start_with_canvas_id(web_options, canvas_id, boxed_app_creator(runtime, None))
        .await
        .ok();
}

fn boxed_app_creator(
    runtime: Arc<dyn AgentRuntime>,
    automation: Option<AutomationBridge>,
) -> AppCreator<'static> {
    Box::new(move |cc| {
        let views = AgentViewRegistry::new()
            .with_view(CounterView::default())
            .with_view(ConversationView)
            .with_view(ToolLogView::default());
        let mut builder = AgentApp::builder(Arc::clone(&runtime))
            .with_creation_context(cc)
            .with_views(views);
        if let Some(automation) = automation.clone() {
            builder = builder.with_automation_bridge(automation);
        }
        let app = builder.build();
        Ok(Box::new(app))
    })
}
