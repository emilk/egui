use crate::math::*;

/// What the integration gives to the gui.
/// All coordinates in emigui is in point/logical coordinates.
#[derive(Clone, Debug, Default, Deserialize)]
#[serde(default)]
pub struct RawInput {
    /// Is the button currently down?
    pub mouse_down: bool,

    /// Current position of the mouse in points.
    pub mouse_pos: Option<Pos2>,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Files has been dropped into the window.
    pub dropped_files: Vec<std::path::PathBuf>,

    /// Someone is threatening to drop these on us.
    pub hovered_files: Vec<std::path::PathBuf>,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

/// What emigui maintains
#[derive(Clone, Debug, Default)]
pub struct GuiInput {
    // TODO: mouse: Mouse as separate
    //
    /// Is the button currently down?
    /// true the frame when it is pressed,
    /// false the frame it is released.
    pub mouse_down: bool,

    /// The mouse went from !down to down
    pub mouse_pressed: bool,

    /// The mouse went from down to !down
    pub mouse_released: bool,

    /// Current position of the mouse in points.
    /// None for touch screens when finger is not down.
    pub mouse_pos: Option<Pos2>,

    /// How much the mouse moved compared to last frame, in points.
    pub mouse_move: Vec2,

    /// Current velocity of mouse cursor, if any.
    pub mouse_velocity: Vec2,

    /// How many pixels the user scrolled
    pub scroll_delta: Vec2,

    /// Size of the screen in points.
    pub screen_size: Vec2,

    /// Also known as device pixel ratio, > 1 for HDPI screens.
    pub pixels_per_point: f32,

    /// Time in seconds. Relative to whatever. Used for animation.
    pub time: f64,

    /// Time since last frame, in seconds.
    pub dt: f32,

    /// Files has been dropped into the window.
    pub dropped_files: Vec<std::path::PathBuf>,

    /// Someone is threatening to drop these on us.
    pub hovered_files: Vec<std::path::PathBuf>,

    /// In-order events received this frame
    pub events: Vec<Event>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Event {
    Copy,
    Cut,
    /// Text input, e.g. via keyboard or paste action
    Text(String),
    Key {
        key: Key,
        pressed: bool,
    },
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Key {
    Alt,
    Backspace,
    Control,
    Delete,
    Down,
    End,
    Escape,
    Home,
    Insert,
    Left,
    /// Windows key or Mac Command key
    Logo,
    PageDown,
    PageUp,
    Return,
    Right,
    Shift,
    // Space,
    Tab,
    Up,
}

impl GuiInput {
    pub fn from_last_and_new(last: &RawInput, new: &RawInput) -> GuiInput {
        let mouse_move = new
            .mouse_pos
            .and_then(|new| last.mouse_pos.map(|last| new - last))
            .unwrap_or_default();
        let dt = (new.time - last.time) as f32;
        let mut mouse_velocity = mouse_move / dt;
        if !mouse_velocity.is_finite() {
            mouse_velocity = Vec2::zero();
        }
        GuiInput {
            mouse_down: new.mouse_down && new.mouse_pos.is_some(),
            mouse_pressed: !last.mouse_down && new.mouse_down,
            mouse_released: last.mouse_down && !new.mouse_down,
            mouse_pos: new.mouse_pos,
            mouse_move,
            mouse_velocity,
            scroll_delta: new.scroll_delta,
            screen_size: new.screen_size,
            pixels_per_point: new.pixels_per_point,
            time: new.time,
            dt,
            dropped_files: new.dropped_files.clone(),
            hovered_files: new.hovered_files.clone(),
            events: new.events.clone(),
        }
    }
}

impl RawInput {
    pub fn ui(&self, region: &mut crate::Region) {
        use crate::label;
        // TODO: simpler way to show values, e.g. `region.value("Mouse Pos:", self.mouse_pos);
        // TODO: easily change default font!
        region.add(label!("mouse_down: {}", self.mouse_down));
        region.add(label!("mouse_pos: {:.1?}", self.mouse_pos));
        region.add(label!("scroll_delta: {:?}", self.scroll_delta));
        region.add(label!("screen_size: {:?}", self.screen_size));
        region.add(label!("pixels_per_point: {}", self.pixels_per_point));
        region.add(label!("time: {:.3} s", self.time));
        region.add(label!("events: {:?}", self.events));
        region.add(label!("dropped_files: {:?}", self.dropped_files));
        region.add(label!("hovered_files: {:?}", self.hovered_files));
    }
}

impl GuiInput {
    pub fn ui(&self, region: &mut crate::Region) {
        use crate::label;
        region.add(label!("mouse_down: {}", self.mouse_down));
        region.add(label!("mouse_pressed: {}", self.mouse_pressed));
        region.add(label!("mouse_released: {}", self.mouse_released));
        region.add(label!("mouse_pos: {:?}", self.mouse_pos));
        region.add(label!("mouse_move: {:?}", self.mouse_move));
        region.add(label!(
            "mouse_velocity: [{:3.0} {:3.0}] points/sec",
            self.mouse_velocity.x,
            self.mouse_velocity.y
        ));
        region.add(label!("scroll_delta: {:?}", self.scroll_delta));
        region.add(label!("screen_size: {:?}", self.screen_size));
        region.add(label!("pixels_per_point: {}", self.pixels_per_point));
        region.add(label!("time: {}", self.time));
        region.add(label!("events: {:?}", self.events));
        region.add(label!("dropped_files: {:?}", self.dropped_files));
        region.add(label!("hovered_files: {:?}", self.hovered_files));
    }
}
