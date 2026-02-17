use eframe_agent::{
    input::AgentInputAdapter,
    runtime::{AgentCommand, AgentRuntime, AgentUpdate, MessageRole, SimpleAgentRuntime},
    state::{AgentState, TaskStatus},
};
use egui::{Event, Key, KeyboardShortcut, Modifiers, RawInput};

#[test]
fn state_tracks_messages_and_tasks() {
    let mut state = AgentState::default();
    state.record_update(AgentUpdate::user("hello"));
    state.record_update(AgentUpdate::TaskStarted {
        id: 1,
        label: "demo".into(),
    });
    state.record_update(AgentUpdate::TaskFinished {
        id: 1,
        label: "demo".into(),
        success: true,
    });

    assert_eq!(state.messages.len(), 1);
    assert_eq!(state.messages[0].role, MessageRole::User);
    assert_eq!(state.tasks.len(), 1);
    assert_eq!(state.tasks[0].status, TaskStatus::Completed);
}

#[test]
fn input_adapter_emits_actions() {
    let mut adapter = AgentInputAdapter::new(
        KeyboardShortcut::new(Modifiers::COMMAND, Key::K),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::L),
        KeyboardShortcut::new(Modifiers::COMMAND, Key::Period),
    );

    let mut raw_input = RawInput::default();
    raw_input.events.push(Event::Key {
        key: Key::K,
        physical_key: None,
        pressed: true,
        repeat: false,
        modifiers: command_mods(),
    });

    adapter.process(&mut raw_input);
    let mut actions = Vec::new();
    adapter.drain_actions(&mut actions);
    assert_eq!(actions.len(), 1);
}

#[test]
fn runtime_emits_updates() {
    let runtime = SimpleAgentRuntime::new();
    runtime.submit_command(AgentCommand::SubmitPrompt("hi".into()));

    let mut updates = Vec::new();
    // wait for background thread to process
    for _ in 0..10 {
        runtime.poll_updates(&mut updates);
        if !updates.is_empty() {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }

    assert!(
        updates
            .iter()
            .any(|update| matches!(update, AgentUpdate::Message { .. })),
        "expected at least one message update"
    );
    runtime.shutdown();
}

fn command_mods() -> Modifiers {
    Modifiers {
        command: true,
        mac_cmd: true,
        ..Default::default()
    }
}
