use accesskit::Vec2;

pub enum AKEvent {
    ActionRequest(accesskit::ActionRequest),
    Simulated(SimulatedEvent),
}

pub enum SimulatedEvent {
    Click { position: Vec2 },
    Type { text: String },
}
