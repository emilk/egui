use crate::{OrderedViewportIdMap, Theme, ViewportId, ViewportIdMap, emath::Rect};

use super::{DroppedFile, Event, HoveredFile, Modifiers, SafeAreaInsets, ViewportInfo};

/// What the integrations provides to egui at the start of each frame.
///
/// Set the values that make sense, leave the rest at their `Default::default()`.
///
/// You can check if `egui` is using the inputs using
/// [`crate::Context::egui_wants_pointer_input`] and [`crate::Context::egui_wants_keyboard_input`].
///
/// All coordinates are in points (logical pixels) with origin (0, 0) in the top left .corner.
///
/// Ii "points" can be calculated from native physical pixels
/// using `pixels_per_point` = [`crate::Context::zoom_factor`] * `native_pixels_per_point`;
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct RawInput {
    /// The id of the active viewport.
    pub viewport_id: ViewportId,

    /// Information about all egui viewports.
    pub viewports: ViewportIdMap<ViewportInfo>,

    /// The insets used to only render content in a mobile safe area
    ///
    /// `None` will be treated as "same as last frame"
    pub safe_area_insets: Option<SafeAreaInsets>,

    /// Position and size of the area that egui should use, in points.
    /// Usually you would set this to
    ///
    /// `Some(Rect::from_min_size(Default::default(), screen_size_in_points))`.
    ///
    /// but you could also constrain egui to some smaller portion of your window if you like.
    ///
    /// `None` will be treated as "same as last frame", with the default being a very big area.
    pub screen_rect: Option<Rect>,

    /// Maximum size of one side of the font texture.
    ///
    /// Ask your graphics drivers about this. This corresponds to `GL_MAX_TEXTURE_SIZE`.
    ///
    /// The default is a very small (but very portable) 2048.
    pub max_texture_side: Option<usize>,

    /// Monotonically increasing time, in seconds. Relative to whatever. Used for animations.
    /// If `None` is provided, egui will assume a time delta of `predicted_dt` (default 1/60 seconds).
    pub time: Option<f64>,

    /// Should be set to the expected time between frames when painting at vsync speeds.
    /// The default for this is 1/60.
    /// Can safely be left at its default value.
    pub predicted_dt: f32,

    /// Which modifier keys are down at the start of the frame?
    pub modifiers: Modifiers,

    /// In-order events received this frame.
    ///
    /// There is currently no way to know if egui handles a particular event,
    /// but you can check if egui is using the keyboard with [`crate::Context::egui_wants_keyboard_input`]
    /// and/or the pointer (mouse/touch) with [`crate::Context::egui_is_using_pointer`].
    pub events: Vec<Event>,

    /// Dragged files hovering over egui.
    pub hovered_files: Vec<HoveredFile>,

    /// Dragged files dropped into egui.
    ///
    /// Note: when using `eframe` on Windows, this will always be empty if drag-and-drop support has
    /// been disabled in [`crate::viewport::ViewportBuilder`].
    pub dropped_files: Vec<DroppedFile>,

    /// The native window has the keyboard focus (i.e. is receiving key presses).
    ///
    /// False when the user alt-tab away from the application, for instance.
    pub focused: bool,

    /// Does the OS use dark or light mode?
    ///
    /// `None` means "don't know".
    pub system_theme: Option<Theme>,
}

impl Default for RawInput {
    fn default() -> Self {
        Self {
            viewport_id: ViewportId::ROOT,
            viewports: std::iter::once((ViewportId::ROOT, Default::default())).collect(),
            screen_rect: None,
            max_texture_side: None,
            time: None,
            predicted_dt: 1.0 / 60.0,
            modifiers: Modifiers::default(),
            events: vec![],
            hovered_files: Default::default(),
            dropped_files: Default::default(),
            focused: true, // integrations opt into global focus tracking
            system_theme: None,
            safe_area_insets: Default::default(),
        }
    }
}

impl RawInput {
    /// Info about the active viewport
    #[inline]
    pub fn viewport(&self) -> &ViewportInfo {
        self.viewports.get(&self.viewport_id).expect("Failed to find current viewport in egui RawInput. This is the fault of the egui backend")
    }

    /// Helper: move volatile (deltas and events), clone the rest.
    ///
    /// * [`Self::hovered_files`] is cloned.
    /// * [`Self::dropped_files`] is moved.
    pub fn take(&mut self) -> Self {
        Self {
            viewport_id: self.viewport_id,
            viewports: self
                .viewports
                .iter_mut()
                .map(|(id, info)| (*id, info.take()))
                .collect(),
            screen_rect: self.screen_rect.take(),
            safe_area_insets: self.safe_area_insets.take(),
            max_texture_side: self.max_texture_side.take(),
            time: self.time,
            predicted_dt: self.predicted_dt,
            modifiers: self.modifiers,
            events: std::mem::take(&mut self.events),
            hovered_files: self.hovered_files.clone(),
            dropped_files: std::mem::take(&mut self.dropped_files),
            focused: self.focused,
            system_theme: self.system_theme,
        }
    }

    /// Add on new input.
    pub fn append(&mut self, newer: Self) {
        let Self {
            viewport_id: viewport_ids,
            viewports,
            screen_rect,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            mut events,
            mut hovered_files,
            mut dropped_files,
            focused,
            system_theme,
            safe_area_insets: safe_area,
        } = newer;

        self.viewport_id = viewport_ids;
        self.viewports = viewports;
        self.screen_rect = screen_rect.or(self.screen_rect);
        self.max_texture_side = max_texture_side.or(self.max_texture_side);
        self.time = time; // use latest time
        self.predicted_dt = predicted_dt; // use latest dt
        self.modifiers = modifiers; // use latest
        self.events.append(&mut events);
        self.hovered_files.append(&mut hovered_files);
        self.dropped_files.append(&mut dropped_files);
        self.focused = focused;
        self.system_theme = system_theme;
        self.safe_area_insets = safe_area;
    }
}

impl RawInput {
    pub fn ui(&self, ui: &mut crate::Ui) {
        let Self {
            viewport_id,
            viewports,
            screen_rect,
            max_texture_side,
            time,
            predicted_dt,
            modifiers,
            events,
            hovered_files,
            dropped_files,
            focused,
            system_theme,
            safe_area_insets: safe_area,
        } = self;

        ui.label(format!("Active viewport: {viewport_id:?}"));
        let ordered_viewports = viewports
            .iter()
            .map(|(id, value)| (*id, value))
            .collect::<OrderedViewportIdMap<_>>();
        for (id, viewport) in ordered_viewports {
            ui.group(|ui| {
                ui.label(format!("Viewport {id:?}"));
                ui.push_id(id, |ui| {
                    viewport.ui(ui);
                });
            });
        }
        ui.label(format!("screen_rect: {screen_rect:?} points"));

        ui.label(format!("max_texture_side: {max_texture_side:?}"));
        if let Some(time) = time {
            ui.label(format!("time: {time:.3} s"));
        } else {
            ui.label("time: None");
        }
        ui.label(format!("predicted_dt: {:.1} ms", 1e3 * predicted_dt));
        ui.label(format!("modifiers: {modifiers:#?}"));
        ui.label(format!("hovered_files: {}", hovered_files.len()));
        ui.label(format!("dropped_files: {}", dropped_files.len()));
        ui.label(format!("focused: {focused}"));
        ui.label(format!("system_theme: {system_theme:?}"));
        ui.label(format!("safe_area: {safe_area:?}"));
        ui.scope(|ui| {
            ui.set_min_height(150.0);
            ui.label(format!("events: {events:#?}"))
                .on_hover_text("key presses etc");
        });
    }
}
