//! egui theme (spacing, colors, etc).

#![allow(clippy::if_same_then_else)]

use std::collections::BTreeMap;

use epaint::{Rounding, Shadow, Stroke};

use crate::{
    ecolor::*, emath::*, ComboBox, CursorIcon, FontFamily, FontId, Grid, Margin, Response,
    RichText, WidgetText,
};

// ----------------------------------------------------------------------------

/// Alias for a [`FontId`] (font of a certain size).
///
/// The font is found via look-up in [`Style::text_styles`].
/// You can use [`TextStyle::resolve`] to do this lookup.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum TextStyle {
    /// Used when small text is needed.
    Small,

    /// Normal labels. Easily readable, doesn't take up too much space.
    Body,

    /// Same size as [`Self::Body`], but used when monospace is important (for code snippets, aligning numbers, etc).
    Monospace,

    /// Buttons. Maybe slightly bigger than [`Self::Body`].
    ///
    /// Signifies that he item can be interacted with.
    Button,

    /// Heading. Probably larger than [`Self::Body`].
    Heading,

    /// A user-chosen style, found in [`Style::text_styles`].
    /// ```
    /// egui::TextStyle::Name("footing".into());
    /// ````
    Name(std::sync::Arc<str>),
}

impl std::fmt::Display for TextStyle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Small => "Small".fmt(f),
            Self::Body => "Body".fmt(f),
            Self::Monospace => "Monospace".fmt(f),
            Self::Button => "Button".fmt(f),
            Self::Heading => "Heading".fmt(f),
            Self::Name(name) => (*name).fmt(f),
        }
    }
}

impl TextStyle {
    /// Look up this [`TextStyle`] in [`Style::text_styles`].
    pub fn resolve(&self, style: &Style) -> FontId {
        style.text_styles.get(self).cloned().unwrap_or_else(|| {
            panic!(
                "Failed to find {:?} in Style::text_styles. Available styles:\n{:#?}",
                self,
                style.text_styles()
            )
        })
    }
}

// ----------------------------------------------------------------------------

/// A way to select [`FontId`], either by picking one directly or by using a [`TextStyle`].
pub enum FontSelection {
    /// Default text style - will use [`TextStyle::Body`], unless
    /// [`Style::override_font_id`] or [`Style::override_text_style`] is set.
    Default,

    /// Directly select size and font family
    FontId(FontId),

    /// Use a [`TextStyle`] to look up the [`FontId`] in [`Style::text_styles`].
    Style(TextStyle),
}

impl Default for FontSelection {
    #[inline]
    fn default() -> Self {
        Self::Default
    }
}

impl FontSelection {
    pub fn resolve(self, style: &Style) -> FontId {
        match self {
            Self::Default => {
                if let Some(override_font_id) = &style.override_font_id {
                    override_font_id.clone()
                } else if let Some(text_style) = &style.override_text_style {
                    text_style.resolve(style)
                } else {
                    TextStyle::Body.resolve(style)
                }
            }
            Self::FontId(font_id) => font_id,
            Self::Style(text_style) => text_style.resolve(style),
        }
    }
}

impl From<FontId> for FontSelection {
    #[inline(always)]
    fn from(font_id: FontId) -> Self {
        Self::FontId(font_id)
    }
}

impl From<TextStyle> for FontSelection {
    #[inline(always)]
    fn from(text_style: TextStyle) -> Self {
        Self::Style(text_style)
    }
}

// ----------------------------------------------------------------------------

/// Specifies the look and feel of egui.
///
/// You can change the visuals of a [`Ui`] with [`Ui::style_mut`]
/// and of everything with [`crate::Context::set_style`].
///
/// If you want to change fonts, use [`crate::Context::set_fonts`] instead.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Style {
    /// If set this will change the default [`TextStyle`] for all widgets.
    ///
    /// On most widgets you can also set an explicit text style,
    /// which will take precedence over this.
    pub override_text_style: Option<TextStyle>,

    /// If set this will change the font family and size for all widgets.
    ///
    /// On most widgets you can also set an explicit text style,
    /// which will take precedence over this.
    pub override_font_id: Option<FontId>,

    /// The [`FontFamily`] and size you want to use for a specific [`TextStyle`].
    ///
    /// The most convenient way to look something up in this is to use [`TextStyle::resolve`].
    ///
    /// If you would like to overwrite app text_styles
    ///
    /// ```
    /// # let mut ctx = egui::Context::default();
    /// use egui::FontFamily::Proportional;
    /// use egui::FontId;
    /// use egui::TextStyle::*;
    ///
    /// // Get current context style
    /// let mut style = (*ctx.style()).clone();
    ///
    /// // Redefine text_styles
    /// style.text_styles = [
    ///   (Heading, FontId::new(30.0, Proportional)),
    ///   (Name("Heading2".into()), FontId::new(25.0, Proportional)),
    ///   (Name("Context".into()), FontId::new(23.0, Proportional)),
    ///   (Body, FontId::new(18.0, Proportional)),
    ///   (Monospace, FontId::new(14.0, Proportional)),
    ///   (Button, FontId::new(14.0, Proportional)),
    ///   (Small, FontId::new(10.0, Proportional)),
    /// ].into();
    ///
    /// // Mutate global style with above changes
    /// ctx.set_style(style);
    /// ```
    pub text_styles: BTreeMap<TextStyle, FontId>,

    /// The style to use for [`DragValue`] text.
    pub drag_value_text_style: TextStyle,

    /// If set, labels buttons wtc will use this to determine whether or not
    /// to wrap the text at the right edge of the [`Ui`] they are in.
    /// By default this is `None`.
    ///
    /// * `None`: follow layout
    /// * `Some(true)`: default on
    /// * `Some(false)`: default off
    pub wrap: Option<bool>,

    /// Sizes and distances between widgets
    pub spacing: Spacing,

    /// How and when interaction happens.
    pub interaction: Interaction,

    /// Colors etc.
    pub visuals: Visuals,

    /// How many seconds a typical animation should last.
    pub animation_time: f32,

    /// Options to help debug why egui behaves strangely.
    ///
    /// Only available in debug builds.
    #[cfg(debug_assertions)]
    pub debug: DebugOptions,

    /// Show tooltips explaining [`DragValue`]:s etc when hovered.
    ///
    /// This only affects a few egui widgets.
    pub explanation_tooltips: bool,

    /// Show the URL of hyperlinks in a tooltip when hovered.
    pub url_in_tooltip: bool,

    /// If true and scrolling is enabled for only one direction, allow horizontal scrolling without pressing shift
    pub always_scroll_the_only_direction: bool,
}

impl Style {
    // TODO(emilk): rename style.interact() to maybe... `style.interactive` ?
    /// Use this style for interactive things.
    /// Note that you must already have a response,
    /// i.e. you must allocate space and interact BEFORE painting the widget!
    pub fn interact(&self, response: &Response) -> &WidgetVisuals {
        self.visuals.widgets.style(response)
    }

    pub fn interact_selectable(&self, response: &Response, selected: bool) -> WidgetVisuals {
        let mut visuals = *self.visuals.widgets.style(response);
        if selected {
            visuals.weak_bg_fill = self.visuals.selection.bg_fill;
            visuals.bg_fill = self.visuals.selection.bg_fill;
            // visuals.bg_stroke = self.visuals.selection.stroke;
            visuals.fg_stroke = self.visuals.selection.stroke;
        }
        visuals
    }

    /// Style to use for non-interactive widgets.
    pub fn noninteractive(&self) -> &WidgetVisuals {
        &self.visuals.widgets.noninteractive
    }

    /// All known text styles.
    pub fn text_styles(&self) -> Vec<TextStyle> {
        self.text_styles.keys().cloned().collect()
    }
}

/// Controls the sizes and distances between widgets.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Spacing {
    /// Horizontal and vertical spacing between widgets.
    ///
    /// To add extra space between widgets, use [`Ui::add_space`].
    ///
    /// `item_spacing` is inserted _after_ adding a widget, so to increase the spacing between
    /// widgets `A` and `B` you need to change `item_spacing` before adding `A`.
    pub item_spacing: Vec2,

    /// Horizontal and vertical margins within a window frame.
    pub window_margin: Margin,

    /// Button size is text size plus this on each side
    pub button_padding: Vec2,

    /// Horizontal and vertical margins within a menu frame.
    pub menu_margin: Margin,

    /// Indent collapsing regions etc by this much.
    pub indent: f32,

    /// Minimum size of a [`DragValue`], color picker button, and other small widgets.
    /// `interact_size.y` is the default height of button, slider, etc.
    /// Anything clickable should be (at least) this size.
    pub interact_size: Vec2, // TODO(emilk): rename min_interact_size ?

    /// Default width of a [`Slider`].
    pub slider_width: f32,

    /// Default rail height of a [`Slider`].
    pub slider_rail_height: f32,

    /// Default (minimum) width of a [`ComboBox`](crate::ComboBox).
    pub combo_width: f32,

    /// Default width of a [`TextEdit`].
    pub text_edit_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the width/height of the outer part of this icon (e.g. the BOX of the checkbox).
    pub icon_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the width/height of the inner part of this icon (e.g. the check of the checkbox).
    pub icon_width_inner: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the spacing between the icon and the text
    pub icon_spacing: f32,

    /// Width of a tooltip (`on_hover_ui`, `on_hover_text` etc).
    pub tooltip_width: f32,

    /// The default width of a menu.
    pub menu_width: f32,

    /// Horizontal distance between a menu and a submenu.
    pub menu_spacing: f32,

    /// End indented regions with a horizontal line
    pub indent_ends_with_horizontal_line: bool,

    /// Height of a combo-box before showing scroll bars.
    pub combo_height: f32,

    /// Controls the spacing of a [`crate::ScrollArea`].
    pub scroll: ScrollStyle,
}

impl Spacing {
    /// Returns small icon rectangle and big icon rectangle
    pub fn icon_rectangles(&self, rect: Rect) -> (Rect, Rect) {
        let icon_width = self.icon_width;
        let big_icon_rect = Rect::from_center_size(
            pos2(rect.left() + icon_width / 2.0, rect.center().y),
            vec2(icon_width, icon_width),
        );

        let small_icon_rect =
            Rect::from_center_size(big_icon_rect.center(), Vec2::splat(self.icon_width_inner));

        (small_icon_rect, big_icon_rect)
    }
}

// ----------------------------------------------------------------------------

/// Controls the spacing and visuals of a [`crate::ScrollArea`].
///
/// There are three presets to chose from:
/// * [`Self::solid`]
/// * [`Self::thin`]
/// * [`Self::floating`]
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct ScrollStyle {
    /// If `true`, scroll bars float above the content, partially covering it.
    ///
    /// If `false`, the scroll bars allocate space, shrinking the area
    /// available to the contents.
    ///
    /// This also changes the colors of the scroll-handle to make
    /// it more promiment.
    pub floating: bool,

    /// The width of the scroll bars at it largest.
    pub bar_width: f32,

    /// Make sure the scroll handle is at least this big
    pub handle_min_length: f32,

    /// Margin between contents and scroll bar.
    pub bar_inner_margin: f32,

    /// Margin between scroll bar and the outer container (e.g. right of a vertical scroll bar).
    /// Only makes sense for non-floating scroll bars.
    pub bar_outer_margin: f32,

    /// The thin width of floating scroll bars that the user is NOT hovering.
    ///
    /// When the user hovers the scroll bars they expand to [`Self::bar_width`].
    pub floating_width: f32,

    /// How much space i allocated for a floating scroll bar?
    ///
    /// Normally this is zero, but you could set this to something small
    /// like 4.0 and set [`Self::dormant_handle_opacity`] and
    /// [`Self::dormant_background_opacity`] to e.g. 0.5
    /// so as to always show a thin scroll bar.
    pub floating_allocated_width: f32,

    /// If true, use colors with more contrast. Good for floating scroll bars.
    pub foreground_color: bool,

    /// The opaqueness of the background when the user is neither scrolling
    /// nor hovering the scroll area.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub dormant_background_opacity: f32,

    /// The opaqueness of the background when the user is hovering
    /// the scroll area, but not the scroll bar.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub active_background_opacity: f32,

    /// The opaqueness of the background when the user is hovering
    /// over the scroll bars.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub interact_background_opacity: f32,

    /// The opaqueness of the handle when the user is neither scrolling
    /// nor hovering the scroll area.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub dormant_handle_opacity: f32,

    /// The opaqueness of the handle when the user is hovering
    /// the scroll area, but not the scroll bar.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub active_handle_opacity: f32,

    /// The opaqueness of the handle when the user is hovering
    /// over the scroll bars.
    ///
    /// This is only for floating scroll bars.
    /// Solid scroll bars are always opaque.
    pub interact_handle_opacity: f32,
}

impl Default for ScrollStyle {
    fn default() -> Self {
        Self::floating()
    }
}

impl ScrollStyle {
    /// Solid scroll bars that always use up space
    pub fn solid() -> Self {
        Self {
            floating: false,
            bar_width: 6.0,
            handle_min_length: 12.0,
            bar_inner_margin: 4.0,
            bar_outer_margin: 0.0,
            floating_width: 2.0,
            floating_allocated_width: 0.0,

            foreground_color: false,

            dormant_background_opacity: 0.0,
            active_background_opacity: 0.4,
            interact_background_opacity: 0.7,

            dormant_handle_opacity: 0.0,
            active_handle_opacity: 0.6,
            interact_handle_opacity: 1.0,
        }
    }

    /// Thin scroll bars that expand on hover
    pub fn thin() -> Self {
        Self {
            floating: true,
            bar_width: 10.0,
            floating_allocated_width: 6.0,
            foreground_color: false,

            dormant_background_opacity: 1.0,
            dormant_handle_opacity: 1.0,

            active_background_opacity: 1.0,
            active_handle_opacity: 1.0,

            // Be tranlucent when expanded so we can see the content
            interact_background_opacity: 0.6,
            interact_handle_opacity: 0.6,

            ..Self::solid()
        }
    }

    /// No scroll bars until you hover the scroll area,
    /// at which time they appear faintly, and then expand
    /// when you hover the scroll bars.
    pub fn floating() -> Self {
        Self {
            floating: true,
            bar_width: 10.0,
            foreground_color: true,
            floating_allocated_width: 0.0,
            dormant_background_opacity: 0.0,
            dormant_handle_opacity: 0.0,
            ..Self::solid()
        }
    }

    /// Width of a solid vertical scrollbar, or height of a horizontal scroll bar, when it is at its widest.
    pub fn allocated_width(&self) -> f32 {
        if self.floating {
            self.floating_allocated_width
        } else {
            self.bar_inner_margin + self.bar_width + self.bar_outer_margin
        }
    }

    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Presets:");
            ui.selectable_value(self, Self::solid(), "Solid");
            ui.selectable_value(self, Self::thin(), "Thin");
            ui.selectable_value(self, Self::floating(), "Floating");
        });

        ui.collapsing("Details", |ui| {
            self.details_ui(ui);
        });
    }

    pub fn details_ui(&mut self, ui: &mut Ui) {
        let Self {
            floating,
            bar_width,
            handle_min_length,
            bar_inner_margin,
            bar_outer_margin,
            floating_width,
            floating_allocated_width,

            foreground_color,

            dormant_background_opacity,
            active_background_opacity,
            interact_background_opacity,
            dormant_handle_opacity,
            active_handle_opacity,
            interact_handle_opacity,
        } = self;

        ui.horizontal(|ui| {
            ui.label("Type:");
            ui.selectable_value(floating, false, "Solid");
            ui.selectable_value(floating, true, "Floating");
        });

        ui.horizontal(|ui| {
            ui.add(DragValue::new(bar_width).clamp_range(0.0..=32.0));
            ui.label("Full bar width");
        });
        if *floating {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(floating_width).clamp_range(0.0..=32.0));
                ui.label("Thin bar width");
            });
            ui.horizontal(|ui| {
                ui.add(DragValue::new(floating_allocated_width).clamp_range(0.0..=32.0));
                ui.label("Allocated width");
            });
        }

        ui.horizontal(|ui| {
            ui.add(DragValue::new(handle_min_length).clamp_range(0.0..=32.0));
            ui.label("Minimum handle length");
        });
        ui.horizontal(|ui| {
            ui.add(DragValue::new(bar_outer_margin).clamp_range(0.0..=32.0));
            ui.label("Outer margin");
        });

        ui.horizontal(|ui| {
            ui.label("Color:");
            ui.selectable_value(foreground_color, false, "Background");
            ui.selectable_value(foreground_color, true, "Foreground");
        });

        if *floating {
            crate::Grid::new("opacity").show(ui, |ui| {
                fn opacity_ui(ui: &mut Ui, opacity: &mut f32) {
                    ui.add(DragValue::new(opacity).speed(0.01).clamp_range(0.0..=1.0));
                }

                ui.label("Opacity");
                ui.label("Dormant");
                ui.label("Active");
                ui.label("Interacting");
                ui.end_row();

                ui.label("Background:");
                opacity_ui(ui, dormant_background_opacity);
                opacity_ui(ui, active_background_opacity);
                opacity_ui(ui, interact_background_opacity);
                ui.end_row();

                ui.label("Handle:");
                opacity_ui(ui, dormant_handle_opacity);
                opacity_ui(ui, active_handle_opacity);
                opacity_ui(ui, interact_handle_opacity);
                ui.end_row();
            });
        } else {
            ui.horizontal(|ui| {
                ui.add(DragValue::new(bar_inner_margin).clamp_range(0.0..=32.0));
                ui.label("Inner margin");
            });
        }
    }
}

// ----------------------------------------------------------------------------

/// How and when interaction happens.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Interaction {
    /// How close a widget must be to the mouse to have a chance to register as a click or drag.
    ///
    /// If this is larger than zero, it gets easier to hit widgets,
    /// which is important for e.g. touch screens.
    pub interact_radius: f32,

    /// Radius of the interactive area of the side of a window during drag-to-resize.
    pub resize_grab_radius_side: f32,

    /// Radius of the interactive area of the corner of a window during drag-to-resize.
    pub resize_grab_radius_corner: f32,

    /// If `false`, tooltips will show up anytime you hover anything, even is mouse is still moving
    pub show_tooltips_only_when_still: bool,

    /// Delay in seconds before showing tooltips after the mouse stops moving
    pub tooltip_delay: f32,

    /// Can you select the text on a [`crate::Label`] by default?
    pub selectable_labels: bool,

    /// Can the user select text that span multiple labels?
    ///
    /// The default is `true`, but text seelction can be slightly glitchy,
    /// so you may want to disable it.
    pub multi_widget_text_select: bool,
}

/// Look and feel of the text cursor.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct TextCursorStyle {
    /// The color and width of the text cursor
    pub stroke: Stroke,

    /// Show where the text cursor would be if you clicked?
    pub preview: bool,

    /// Should the cursor blink?
    pub blink: bool,

    /// When blinking, this is how long the cursor is visible.
    pub on_duration: f32,

    /// When blinking, this is how long the cursor is invisible.
    pub off_duration: f32,
}

impl Default for TextCursorStyle {
    fn default() -> Self {
        Self {
            stroke: Stroke::new(2.0, Color32::from_rgb(192, 222, 255)), // Dark mode
            preview: false,
            blink: true,
            on_duration: 0.5,
            off_duration: 0.5,
        }
    }
}

/// Controls the visual style (colors etc) of egui.
///
/// You can change the visuals of a [`Ui`] with [`Ui::visuals_mut`]
/// and of everything with [`crate::Context::set_visuals`].
///
/// If you want to change fonts, use [`crate::Context::set_fonts`] instead.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Visuals {
    /// If true, the visuals are overall dark with light text.
    /// If false, the visuals are overall light with dark text.
    ///
    /// NOTE: setting this does very little by itself,
    /// this is more to provide a convenient summary of the rest of the settings.
    pub dark_mode: bool,

    /// Override default text color for all text.
    ///
    /// This is great for setting the color of text for any widget.
    ///
    /// If `text_color` is `None` (default), then the text color will be the same as the
    /// foreground stroke color (`WidgetVisuals::fg_stroke`)
    /// and will depend on whether or not the widget is being interacted with.
    ///
    /// In the future we may instead modulate
    /// the `text_color` based on whether or not it is interacted with
    /// so that `visuals.text_color` is always used,
    /// but its alpha may be different based on whether or not
    /// it is disabled, non-interactive, hovered etc.
    pub override_text_color: Option<Color32>,

    /// Visual styles of widgets
    pub widgets: Widgets,

    pub selection: Selection,

    /// The color used for [`Hyperlink`],
    pub hyperlink_color: Color32,

    /// Something just barely different from the background color.
    /// Used for [`crate::Grid::striped`].
    pub faint_bg_color: Color32,

    /// Very dark or light color (for corresponding theme).
    /// Used as the background of text edits, scroll bars and others things
    /// that needs to look different from other interactive stuff.
    pub extreme_bg_color: Color32,

    /// Background color behind code-styled monospaced labels.
    pub code_bg_color: Color32,

    /// A good color for warning text (e.g. orange).
    pub warn_fg_color: Color32,

    /// A good color for error text (e.g. red).
    pub error_fg_color: Color32,

    pub window_rounding: Rounding,
    pub window_shadow: Shadow,
    pub window_fill: Color32,
    pub window_stroke: Stroke,

    /// Highlight the topmost window.
    pub window_highlight_topmost: bool,

    pub menu_rounding: Rounding,

    /// Panel background color
    pub panel_fill: Color32,

    pub popup_shadow: Shadow,

    pub resize_corner_size: f32,

    /// How the text cursor acts.
    pub text_cursor: TextCursorStyle,

    /// Allow child widgets to be just on the border and still have a stroke with some thickness
    pub clip_rect_margin: f32,

    /// Show a background behind buttons.
    pub button_frame: bool,

    /// Show a background behind collapsing headers.
    pub collapsing_header_frame: bool,

    /// Draw a vertical lien left of indented region, in e.g. [`crate::CollapsingHeader`].
    pub indent_has_left_vline: bool,

    /// Whether or not Grids and Tables should be striped by default
    /// (have alternating rows differently colored).
    pub striped: bool,

    /// Show trailing color behind the circle of a [`Slider`]. Default is OFF.
    ///
    /// Enabling this will affect ALL sliders, and can be enabled/disabled per slider with [`Slider::trailing_fill`].
    pub slider_trailing_fill: bool,

    /// Shape of the handle for sliders and similar widgets.
    ///
    /// Changing this will affect ALL sliders, and can be enabled/disabled per slider with [`Slider::handle_shape`].
    pub handle_shape: HandleShape,

    /// Should the cursor change when the user hovers over an interactive/clickable item?
    ///
    /// This is consistent with a lot of browser-based applications (vscode, github
    /// all turn your cursor into [`CursorIcon::PointingHand`] when a button is
    /// hovered) but it is inconsistent with native UI toolkits.
    pub interact_cursor: Option<CursorIcon>,

    /// Show a spinner when loading an image.
    pub image_loading_spinners: bool,

    /// How to display numeric color values.
    pub numeric_color_space: NumericColorSpace,
}

impl Visuals {
    #[inline(always)]
    pub fn noninteractive(&self) -> &WidgetVisuals {
        &self.widgets.noninteractive
    }

    // Non-interactive text color.
    pub fn text_color(&self) -> Color32 {
        self.override_text_color
            .unwrap_or_else(|| self.widgets.noninteractive.text_color())
    }

    pub fn weak_text_color(&self) -> Color32 {
        self.gray_out(self.text_color())
    }

    #[inline(always)]
    pub fn strong_text_color(&self) -> Color32 {
        self.widgets.active.text_color()
    }

    /// Window background color.
    #[inline(always)]
    pub fn window_fill(&self) -> Color32 {
        self.window_fill
    }

    #[inline(always)]
    pub fn window_stroke(&self) -> Stroke {
        self.window_stroke
    }

    /// When fading out things, we fade the colors towards this.
    // TODO(emilk): replace with an alpha
    #[inline(always)]
    pub fn fade_out_to_color(&self) -> Color32 {
        self.widgets.noninteractive.weak_bg_fill
    }

    /// Returned a "grayed out" version of the given color.
    #[inline(always)]
    pub fn gray_out(&self, color: Color32) -> Color32 {
        crate::ecolor::tint_color_towards(color, self.fade_out_to_color())
    }
}

/// Selected text, selected elements etc
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Selection {
    pub bg_fill: Color32,
    pub stroke: Stroke,
}

/// Shape of the handle for sliders and similar widgets.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum HandleShape {
    /// Circular handle
    Circle,

    /// Rectangular handle
    Rect {
        /// Aspect ratio of the rectangle. Set to < 1.0 to make it narrower.
        aspect_ratio: f32,
    },
}

/// The visuals of widgets for different states of interaction.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Widgets {
    /// The style of a widget that you cannot interact with.
    /// * `noninteractive.bg_stroke` is the outline of windows.
    /// * `noninteractive.bg_fill` is the background color of windows.
    /// * `noninteractive.fg_stroke` is the normal text color.
    pub noninteractive: WidgetVisuals,

    /// The style of an interactive widget, such as a button, at rest.
    pub inactive: WidgetVisuals,

    /// The style of an interactive widget while you hover it, or when it is highlighted.
    ///
    /// See [`Response::hovered`], [`Response::highlighted`] and [`Response::highlight`].
    pub hovered: WidgetVisuals,

    /// The style of an interactive widget as you are clicking or dragging it.
    pub active: WidgetVisuals,

    /// The style of a button that has an open menu beneath it (e.g. a combo-box)
    pub open: WidgetVisuals,
}

impl Widgets {
    pub fn style(&self, response: &Response) -> &WidgetVisuals {
        if !response.sense.interactive() {
            &self.noninteractive
        } else if response.is_pointer_button_down_on() || response.has_focus() || response.clicked()
        {
            &self.active
        } else if response.hovered() || response.highlighted() {
            &self.hovered
        } else {
            &self.inactive
        }
    }
}

/// bg = background, fg = foreground.
#[derive(Clone, Copy, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct WidgetVisuals {
    /// Background color of widgets that must have a background fill,
    /// such as the slider background, a checkbox background, or a radio button background.
    ///
    /// Must never be [`Color32::TRANSPARENT`].
    pub bg_fill: Color32,

    /// Background color of widgets that can _optionally_ have a background fill, such as buttons.
    ///
    /// May be [`Color32::TRANSPARENT`].
    pub weak_bg_fill: Color32,

    /// For surrounding rectangle of things that need it,
    /// like buttons, the box of the checkbox, etc.
    /// Should maybe be called `frame_stroke`.
    pub bg_stroke: Stroke,

    /// Button frames etc.
    pub rounding: Rounding,

    /// Stroke and text color of the interactive part of a component (button text, slider grab, check-mark, …).
    pub fg_stroke: Stroke,

    /// Make the frame this much larger.
    pub expansion: f32,
}

impl WidgetVisuals {
    #[inline(always)]
    pub fn text_color(&self) -> Color32 {
        self.fg_stroke.color
    }
}

/// Options for help debug egui by adding extra visualization
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg(debug_assertions)]
pub struct DebugOptions {
    /// Always show callstack to ui on hover.
    ///
    /// Useful for figuring out where in the code some UI is being created.
    ///
    /// Only works in debug builds.
    /// Requires the `callstack` feature.
    /// Does not work on web.
    #[cfg(debug_assertions)]
    pub debug_on_hover: bool,

    /// Show callstack for the current widget on hover if all modifier keys are pressed down.
    ///
    /// Useful for figuring out where in the code some UI is being created.
    ///
    /// Only works in debug builds.
    /// Requires the `callstack` feature.
    /// Does not work on web.
    ///
    /// Default is `true` in debug builds, on native, if the `callstack` feature is enabled.
    #[cfg(debug_assertions)]
    pub debug_on_hover_with_all_modifiers: bool,

    /// If we show the hover ui, include where the next widget is placed.
    #[cfg(debug_assertions)]
    pub hover_shows_next: bool,

    /// Show which widgets make their parent wider
    pub show_expand_width: bool,

    /// Show which widgets make their parent higher
    pub show_expand_height: bool,

    pub show_resize: bool,

    /// Show an overlay on all interactive widgets.
    pub show_interactive_widgets: bool,

    /// Show interesting widgets under the mouse cursor.
    pub show_widget_hits: bool,
}

#[cfg(debug_assertions)]
impl Default for DebugOptions {
    fn default() -> Self {
        Self {
            debug_on_hover: false,
            debug_on_hover_with_all_modifiers: cfg!(feature = "callstack")
                && !cfg!(target_arch = "wasm32"),
            hover_shows_next: false,
            show_expand_width: false,
            show_expand_height: false,
            show_resize: false,
            show_interactive_widgets: false,
            show_widget_hits: false,
        }
    }
}

// ----------------------------------------------------------------------------

/// The default text styles of the default egui theme.
pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    use FontFamily::{Monospace, Proportional};

    [
        (TextStyle::Small, FontId::new(9.0, Proportional)),
        (TextStyle::Body, FontId::new(12.5, Proportional)),
        (TextStyle::Button, FontId::new(12.5, Proportional)),
        (TextStyle::Heading, FontId::new(18.0, Proportional)),
        (TextStyle::Monospace, FontId::new(12.0, Monospace)),
    ]
    .into()
}

impl Default for Style {
    fn default() -> Self {
        Self {
            override_font_id: None,
            override_text_style: None,
            text_styles: default_text_styles(),
            drag_value_text_style: TextStyle::Button,
            wrap: None,
            spacing: Spacing::default(),
            interaction: Interaction::default(),
            visuals: Visuals::default(),
            animation_time: 1.0 / 12.0,
            #[cfg(debug_assertions)]
            debug: Default::default(),
            explanation_tooltips: false,
            url_in_tooltip: false,
            always_scroll_the_only_direction: false,
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            item_spacing: vec2(8.0, 3.0),
            window_margin: Margin::same(6.0),
            menu_margin: Margin::same(6.0),
            button_padding: vec2(4.0, 1.0),
            indent: 18.0, // match checkbox/radio-button with `button_padding.x + icon_width + icon_spacing`
            interact_size: vec2(40.0, 18.0),
            slider_width: 100.0,
            slider_rail_height: 8.0,
            combo_width: 100.0,
            text_edit_width: 280.0,
            icon_width: 14.0,
            icon_width_inner: 8.0,
            icon_spacing: 4.0,
            tooltip_width: 600.0,
            menu_width: 150.0,
            menu_spacing: 2.0,
            combo_height: 200.0,
            scroll: Default::default(),
            indent_ends_with_horizontal_line: false,
        }
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            resize_grab_radius_side: 5.0,
            resize_grab_radius_corner: 10.0,
            interact_radius: 5.0,
            show_tooltips_only_when_still: true,
            tooltip_delay: 0.3,
            selectable_labels: true,
            multi_widget_text_select: true,
        }
    }
}

impl Visuals {
    /// Default dark theme.
    pub fn dark() -> Self {
        Self {
            dark_mode: true,
            override_text_color: None,
            widgets: Widgets::default(),
            selection: Selection::default(),
            hyperlink_color: Color32::from_rgb(90, 170, 255),
            faint_bg_color: Color32::from_additive_luminance(5), // visible, but barely so
            extreme_bg_color: Color32::from_gray(10),            // e.g. TextEdit background
            code_bg_color: Color32::from_gray(64),
            warn_fg_color: Color32::from_rgb(255, 143, 0), // orange
            error_fg_color: Color32::from_rgb(255, 0, 0),  // red

            window_rounding: Rounding::same(6.0),
            window_shadow: Shadow {
                offset: vec2(10.0, 20.0),
                blur: 15.0,
                spread: 0.0,
                color: Color32::from_black_alpha(96),
            },
            window_fill: Color32::from_gray(27),
            window_stroke: Stroke::new(1.0, Color32::from_gray(60)),
            window_highlight_topmost: true,

            menu_rounding: Rounding::same(6.0),

            panel_fill: Color32::from_gray(27),

            popup_shadow: Shadow {
                offset: vec2(6.0, 10.0),
                blur: 8.0,
                spread: 0.0,
                color: Color32::from_black_alpha(96),
            },

            resize_corner_size: 12.0,

            text_cursor: Default::default(),

            clip_rect_margin: 3.0, // should be at least half the size of the widest frame stroke + max WidgetVisuals::expansion
            button_frame: true,
            collapsing_header_frame: false,
            indent_has_left_vline: true,

            striped: false,

            slider_trailing_fill: false,
            handle_shape: HandleShape::Circle,

            interact_cursor: None,

            image_loading_spinners: true,

            numeric_color_space: NumericColorSpace::GammaByte,
        }
    }

    /// Default light theme.
    pub fn light() -> Self {
        Self {
            dark_mode: false,
            widgets: Widgets::light(),
            selection: Selection::light(),
            hyperlink_color: Color32::from_rgb(0, 155, 255),
            faint_bg_color: Color32::from_additive_luminance(5), // visible, but barely so
            extreme_bg_color: Color32::from_gray(255),           // e.g. TextEdit background
            code_bg_color: Color32::from_gray(230),
            warn_fg_color: Color32::from_rgb(255, 100, 0), // slightly orange red. it's difficult to find a warning color that pops on bright background.
            error_fg_color: Color32::from_rgb(255, 0, 0),  // red

            window_shadow: Shadow {
                offset: vec2(10.0, 20.0),
                blur: 15.0,
                spread: 0.0,
                color: Color32::from_black_alpha(25),
            },
            window_fill: Color32::from_gray(248),
            window_stroke: Stroke::new(1.0, Color32::from_gray(190)),

            panel_fill: Color32::from_gray(248),

            popup_shadow: Shadow {
                offset: vec2(6.0, 10.0),
                blur: 8.0,
                spread: 0.0,
                color: Color32::from_black_alpha(25),
            },

            text_cursor: TextCursorStyle {
                stroke: Stroke::new(2.0, Color32::from_rgb(0, 83, 125)),
                ..Default::default()
            },

            ..Self::dark()
        }
    }
}

impl Default for Visuals {
    fn default() -> Self {
        Self::dark()
    }
}

impl Selection {
    fn dark() -> Self {
        Self {
            bg_fill: Color32::from_rgb(0, 92, 128),
            stroke: Stroke::new(1.0, Color32::from_rgb(192, 222, 255)),
        }
    }

    fn light() -> Self {
        Self {
            bg_fill: Color32::from_rgb(144, 209, 255),
            stroke: Stroke::new(1.0, Color32::from_rgb(0, 83, 125)),
        }
    }
}

impl Default for Selection {
    fn default() -> Self {
        Self::dark()
    }
}

impl Widgets {
    pub fn dark() -> Self {
        Self {
            noninteractive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(27),
                bg_fill: Color32::from_gray(27),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines
                fg_stroke: Stroke::new(1.0, Color32::from_gray(140)), // normal text color
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(60), // button background
                bg_fill: Color32::from_gray(60),      // checkbox background
                bg_stroke: Default::default(),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(180)), // button text
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(70),
                bg_fill: Color32::from_gray(70),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                rounding: Rounding::same(3.0),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(55),
                bg_fill: Color32::from_gray(55),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                rounding: Rounding::same(2.0),
                expansion: 1.0,
            },
            open: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(45),
                bg_fill: Color32::from_gray(27),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(210)),
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
        }
    }

    pub fn light() -> Self {
        Self {
            noninteractive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(248),
                bg_fill: Color32::from_gray(248),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines
                fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(230), // button background
                bg_fill: Color32::from_gray(230),      // checkbox background
                bg_stroke: Default::default(),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(220),
                bg_fill: Color32::from_gray(220),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                fg_stroke: Stroke::new(1.5, Color32::BLACK),
                rounding: Rounding::same(3.0),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(165),
                bg_fill: Color32::from_gray(165),
                bg_stroke: Stroke::new(1.0, Color32::BLACK),
                fg_stroke: Stroke::new(2.0, Color32::BLACK),
                rounding: Rounding::same(2.0),
                expansion: 1.0,
            },
            open: WidgetVisuals {
                weak_bg_fill: Color32::from_gray(220),
                bg_fill: Color32::from_gray(220),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(160)),
                fg_stroke: Stroke::new(1.0, Color32::BLACK),
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
        }
    }
}

impl Default for Widgets {
    fn default() -> Self {
        Self::dark()
    }
}

// ----------------------------------------------------------------------------

use crate::{widgets::*, Ui};

impl Style {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            override_font_id,
            override_text_style,
            text_styles,
            drag_value_text_style,
            wrap: _,
            spacing,
            interaction,
            visuals,
            animation_time,
            #[cfg(debug_assertions)]
            debug,
            explanation_tooltips,
            url_in_tooltip,
            always_scroll_the_only_direction,
        } = self;

        visuals.light_dark_radio_buttons(ui);

        crate::Grid::new("_options").show(ui, |ui| {
            ui.label("Override font id");
            ui.vertical(|ui| {
                ui.horizontal(|ui| {
                    ui.radio_value(override_font_id, None, "None");
                    if ui.radio(override_font_id.is_some(), "override").clicked() {
                        *override_font_id = Some(FontId::default());
                    }
                });
                if let Some(override_font_id) = override_font_id {
                    crate::introspection::font_id_ui(ui, override_font_id);
                }
            });
            ui.end_row();

            ui.label("Override text style");
            crate::ComboBox::from_id_source("Override text style")
                .selected_text(match override_text_style {
                    None => "None".to_owned(),
                    Some(override_text_style) => override_text_style.to_string(),
                })
                .show_ui(ui, |ui| {
                    ui.selectable_value(override_text_style, None, "None");
                    let all_text_styles = ui.style().text_styles();
                    for style in all_text_styles {
                        let text =
                            crate::RichText::new(style.to_string()).text_style(style.clone());
                        ui.selectable_value(override_text_style, Some(style), text);
                    }
                });
            ui.end_row();

            ui.label("Text style of DragValue");
            crate::ComboBox::from_id_source("drag_value_text_style")
                .selected_text(drag_value_text_style.to_string())
                .show_ui(ui, |ui| {
                    let all_text_styles = ui.style().text_styles();
                    for style in all_text_styles {
                        let text =
                            crate::RichText::new(style.to_string()).text_style(style.clone());
                        ui.selectable_value(drag_value_text_style, style, text);
                    }
                });
            ui.end_row();

            ui.label("Animation duration");
            ui.add(
                DragValue::new(animation_time)
                    .clamp_range(0.0..=1.0)
                    .speed(0.02)
                    .suffix(" s"),
            );
            ui.end_row();
        });

        ui.collapsing("🔠 Text Styles", |ui| text_styles_ui(ui, text_styles));
        ui.collapsing("📏 Spacing", |ui| spacing.ui(ui));
        ui.collapsing("☝ Interaction", |ui| interaction.ui(ui));
        ui.collapsing("🎨 Visuals", |ui| visuals.ui(ui));

        #[cfg(debug_assertions)]
        ui.collapsing("🐛 Debug", |ui| debug.ui(ui));

        ui.checkbox(explanation_tooltips, "Explanation tooltips")
            .on_hover_text(
                "Show explanatory text when hovering DragValue:s and other egui widgets",
            );

        ui.checkbox(url_in_tooltip, "Show url when hovering links");

        ui.checkbox(always_scroll_the_only_direction, "Always scroll the only enabled direction")
            .on_hover_text(
                "If scrolling is enabled for only one direction, allow horizontal scrolling without pressing shift",
            );

        ui.vertical_centered(|ui| reset_button(ui, self, "Reset style"));
    }
}

fn text_styles_ui(ui: &mut Ui, text_styles: &mut BTreeMap<TextStyle, FontId>) -> Response {
    ui.vertical(|ui| {
        crate::Grid::new("text_styles").show(ui, |ui| {
            for (text_style, font_id) in &mut *text_styles {
                ui.label(RichText::new(text_style.to_string()).font(font_id.clone()));
                crate::introspection::font_id_ui(ui, font_id);
                ui.end_row();
            }
        });
        crate::reset_button_with(ui, text_styles, "Reset text styles", default_text_styles());
    })
    .response
}

impl Spacing {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            item_spacing,
            window_margin,
            menu_margin,
            button_padding,
            indent,
            interact_size,
            slider_width,
            slider_rail_height,
            combo_width,
            text_edit_width,
            icon_width,
            icon_width_inner,
            icon_spacing,
            tooltip_width,
            menu_width,
            menu_spacing,
            indent_ends_with_horizontal_line,
            combo_height,
            scroll,
        } = self;

        Grid::new("spacing")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Item spacing");
                ui.add(two_drag_values(item_spacing, 0.0..=20.0));
                ui.end_row();

                ui.label("Window margin");
                ui.add(window_margin);
                ui.end_row();

                ui.label("Menu margin");
                ui.add(menu_margin);
                ui.end_row();

                ui.label("Button padding");
                ui.add(two_drag_values(button_padding, 0.0..=20.0));
                ui.end_row();

                ui.label("Interact size")
                    .on_hover_text("Minimum size of an interactive widget");
                ui.add(two_drag_values(interact_size, 4.0..=60.0));
                ui.end_row();

                ui.label("Indent");
                ui.add(DragValue::new(indent).clamp_range(0.0..=100.0));
                ui.end_row();

                ui.label("Slider width");
                ui.add(DragValue::new(slider_width).clamp_range(0.0..=1000.0));
                ui.end_row();

                ui.label("Slider rail height");
                ui.add(DragValue::new(slider_rail_height).clamp_range(0.0..=50.0));
                ui.end_row();

                ui.label("ComboBox width");
                ui.add(DragValue::new(combo_width).clamp_range(0.0..=1000.0));
                ui.end_row();

                ui.label("TextEdit width");
                ui.add(DragValue::new(text_edit_width).clamp_range(0.0..=1000.0));
                ui.end_row();

                ui.label("Tooltip wrap width");
                ui.add(DragValue::new(tooltip_width).clamp_range(0.0..=1000.0));
                ui.end_row();

                ui.label("Default menu width");
                ui.add(DragValue::new(menu_width).clamp_range(0.0..=1000.0));
                ui.end_row();

                ui.label("Menu spacing")
                    .on_hover_text("Horizontal spacing between menus");
                ui.add(DragValue::new(menu_spacing).clamp_range(0.0..=10.0));
                ui.end_row();

                ui.label("Checkboxes etc");
                ui.vertical(|ui| {
                    ui.add(
                        DragValue::new(icon_width)
                            .prefix("outer icon width:")
                            .clamp_range(0.0..=60.0),
                    );
                    ui.add(
                        DragValue::new(icon_width_inner)
                            .prefix("inner icon width:")
                            .clamp_range(0.0..=60.0),
                    );
                    ui.add(
                        DragValue::new(icon_spacing)
                            .prefix("spacing:")
                            .clamp_range(0.0..=10.0),
                    );
                });
                ui.end_row();
            });

        ui.checkbox(
            indent_ends_with_horizontal_line,
            "End indented regions with a horizontal separator",
        );

        ui.horizontal(|ui| {
            ui.label("Max height of a combo box");
            ui.add(DragValue::new(combo_height).clamp_range(0.0..=1000.0));
        });

        ui.collapsing("Scroll Area", |ui| {
            scroll.ui(ui);
        });

        ui.vertical_centered(|ui| reset_button(ui, self, "Reset spacing"));
    }
}

impl Interaction {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            interact_radius,
            resize_grab_radius_side,
            resize_grab_radius_corner,
            show_tooltips_only_when_still,
            tooltip_delay,
            selectable_labels,
            multi_widget_text_select,
        } = self;

        ui.spacing_mut().item_spacing = vec2(12.0, 8.0);

        Grid::new("interaction")
            .num_columns(2)
            .striped(true)
            .show(ui, |ui| {
                ui.label("interact_radius")
                    .on_hover_text("Interact with the closest widget within this radius.");
                ui.add(DragValue::new(interact_radius).clamp_range(0.0..=20.0));
                ui.end_row();

                ui.label("resize_grab_radius_side").on_hover_text("Radius of the interactive area of the side of a window during drag-to-resize");
                ui.add(DragValue::new(resize_grab_radius_side).clamp_range(0.0..=20.0));
                ui.end_row();

                ui.label("resize_grab_radius_corner").on_hover_text("Radius of the interactive area of the corner of a window during drag-to-resize.");
                ui.add(DragValue::new(resize_grab_radius_corner).clamp_range(0.0..=20.0));
                ui.end_row();

                ui.label("Tooltip delay").on_hover_text(
                    "Delay in seconds before showing tooltips after the mouse stops moving",
                );
                ui.add(
                    DragValue::new(tooltip_delay)
                        .clamp_range(0.0..=1.0)
                        .speed(0.05)
                        .suffix(" s"),
                );
                ui.end_row();
            });

        ui.checkbox(
            show_tooltips_only_when_still,
            "Only show tooltips if mouse is still",
        );

        ui.horizontal(|ui| {
            ui.checkbox(selectable_labels, "Selectable text in labels");
            if *selectable_labels {
                ui.checkbox(multi_widget_text_select, "Across multiple labels");
            }
        });

        ui.vertical_centered(|ui| reset_button(ui, self, "Reset interaction settings"));
    }
}

impl Widgets {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            active,
            hovered,
            inactive,
            noninteractive,
            open,
        } = self;

        ui.collapsing("Noninteractive", |ui| {
            ui.label(
                "The style of a widget that you cannot interact with, e.g. labels and separators.",
            );
            noninteractive.ui(ui);
        });
        ui.collapsing("Interactive but inactive", |ui| {
            ui.label("The style of an interactive widget, such as a button, at rest.");
            inactive.ui(ui);
        });
        ui.collapsing("Interactive and hovered", |ui| {
            ui.label("The style of an interactive widget while you hover it.");
            hovered.ui(ui);
        });
        ui.collapsing("Interactive and active", |ui| {
            ui.label("The style of an interactive widget as you are clicking or dragging it.");
            active.ui(ui);
        });
        ui.collapsing("Open menu", |ui| {
            ui.label("The style of an open combo-box or menu button");
            open.ui(ui);
        });

        // ui.vertical_centered(|ui| reset_button(ui, self));
    }
}

impl Selection {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self { bg_fill, stroke } = self;
        ui.label("Selectable labels");

        Grid::new("selectiom").num_columns(2).show(ui, |ui| {
            ui.label("Background fill");
            ui.color_edit_button_srgba(bg_fill);
            ui.end_row();

            ui.label("Stroke");
            ui.add(stroke);
            ui.end_row();
        });
    }
}

impl WidgetVisuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            weak_bg_fill,
            bg_fill: mandatory_bg_fill,
            bg_stroke,
            rounding,
            fg_stroke,
            expansion,
        } = self;

        Grid::new("widget")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Optional background fill")
                    .on_hover_text("For buttons, combo-boxes, etc");
                ui.color_edit_button_srgba(weak_bg_fill);
                ui.end_row();

                ui.label("Mandatory background fill")
                    .on_hover_text("For checkboxes, sliders, etc");
                ui.color_edit_button_srgba(mandatory_bg_fill);
                ui.end_row();

                ui.label("Background stroke");
                ui.add(bg_stroke);
                ui.end_row();

                ui.label("Rounding");
                ui.add(rounding);
                ui.end_row();

                ui.label("Foreground stroke (text)");
                ui.add(fg_stroke);
                ui.end_row();

                ui.label("Expansion")
                    .on_hover_text("make shapes this much larger");
                ui.add(DragValue::new(expansion).speed(0.1));
                ui.end_row();
            });
    }
}

impl Visuals {
    /// Show radio-buttons to switch between light and dark mode.
    pub fn light_dark_radio_buttons(&mut self, ui: &mut crate::Ui) {
        ui.horizontal(|ui| {
            ui.selectable_value(self, Self::light(), "☀ Light");
            ui.selectable_value(self, Self::dark(), "🌙 Dark");
        });
    }

    /// Show small toggle-button for light and dark mode.
    #[must_use]
    pub fn light_dark_small_toggle_button(&self, ui: &mut crate::Ui) -> Option<Self> {
        #![allow(clippy::collapsible_else_if)]
        if self.dark_mode {
            if ui
                .add(Button::new("☀").frame(false))
                .on_hover_text("Switch to light mode")
                .clicked()
            {
                return Some(Self::light());
            }
        } else {
            if ui
                .add(Button::new("🌙").frame(false))
                .on_hover_text("Switch to dark mode")
                .clicked()
            {
                return Some(Self::dark());
            }
        }
        None
    }

    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            dark_mode: _,
            override_text_color: _,
            widgets,
            selection,
            hyperlink_color,
            faint_bg_color,
            extreme_bg_color,
            code_bg_color,
            warn_fg_color,
            error_fg_color,

            window_rounding,
            window_shadow,
            window_fill,
            window_stroke,
            window_highlight_topmost,

            menu_rounding,

            panel_fill,

            popup_shadow,

            resize_corner_size,

            text_cursor,

            clip_rect_margin,
            button_frame,
            collapsing_header_frame,
            indent_has_left_vline,

            striped,

            slider_trailing_fill,
            handle_shape,
            interact_cursor,

            image_loading_spinners,

            numeric_color_space,
        } = self;

        ui.collapsing("Background Colors", |ui| {
            ui_color(ui, &mut widgets.inactive.weak_bg_fill, "Buttons");
            ui_color(ui, window_fill, "Windows");
            ui_color(ui, panel_fill, "Panels");
            ui_color(ui, faint_bg_color, "Faint accent").on_hover_text(
                "Used for faint accentuation of interactive things, like striped grids.",
            );
            ui_color(ui, extreme_bg_color, "Extreme")
                .on_hover_text("Background of plots and paintings");
        });

        ui.collapsing("Text color", |ui| {
            ui_text_color(ui, &mut widgets.noninteractive.fg_stroke.color, "Label");
            ui_text_color(
                ui,
                &mut widgets.inactive.fg_stroke.color,
                "Unhovered button",
            );
            ui_text_color(ui, &mut widgets.hovered.fg_stroke.color, "Hovered button");
            ui_text_color(ui, &mut widgets.active.fg_stroke.color, "Clicked button");

            ui_text_color(ui, warn_fg_color, RichText::new("Warnings"));
            ui_text_color(ui, error_fg_color, RichText::new("Errors"));

            ui_text_color(ui, hyperlink_color, "hyperlink_color");

            ui_color(ui, code_bg_color, RichText::new("Code background").code()).on_hover_ui(
                |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 0.0;
                        ui.label("For monospaced inlined text ");
                        ui.code("like this");
                        ui.label(".");
                    });
                },
            );
        });

        ui.collapsing("Text cursor", |ui| {
            text_cursor.ui(ui);
        });

        ui.collapsing("Window", |ui| {
            Grid::new("window")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Fill");
                    ui.color_edit_button_srgba(window_fill);
                    ui.end_row();

                    ui.label("Stroke");
                    ui.add(window_stroke);
                    ui.end_row();

                    ui.label("Rounding");
                    ui.add(window_rounding);
                    ui.end_row();

                    ui.label("Shadow");
                    ui.add(window_shadow);
                    ui.end_row();
                });

            ui.checkbox(window_highlight_topmost, "Highlight topmost Window");
        });

        ui.collapsing("Menus and popups", |ui| {
            Grid::new("menus_and_popups")
                .num_columns(2)
                .spacing([12.0, 8.0])
                .striped(true)
                .show(ui, |ui| {
                    ui.label("Rounding");
                    ui.add(menu_rounding);
                    ui.end_row();

                    ui.label("Shadow");
                    ui.add(popup_shadow);
                    ui.end_row();
                });
        });

        ui.collapsing("Widgets", |ui| widgets.ui(ui));
        ui.collapsing("Selection", |ui| selection.ui(ui));

        ui.collapsing("Misc", |ui| {
            ui.add(Slider::new(resize_corner_size, 0.0..=20.0).text("resize_corner_size"));
            ui.add(Slider::new(clip_rect_margin, 0.0..=20.0).text("clip_rect_margin"));

            ui.checkbox(button_frame, "Button has a frame");
            ui.checkbox(collapsing_header_frame, "Collapsing header has a frame");
            ui.checkbox(
                indent_has_left_vline,
                "Paint a vertical line to the left of indented regions",
            );

            ui.checkbox(striped, "Default stripes on grids and tables");

            ui.checkbox(slider_trailing_fill, "Add trailing color to sliders");

            handle_shape.ui(ui);

            ComboBox::from_label("Interact cursor")
                .selected_text(
                    interact_cursor.map_or_else(|| "-".to_owned(), |cursor| format!("{cursor:?}")),
                )
                .show_ui(ui, |ui| {
                    ui.selectable_value(interact_cursor, None, "-");

                    for cursor in CursorIcon::ALL {
                        ui.selectable_value(interact_cursor, Some(cursor), format!("{cursor:?}"))
                            .on_hover_cursor(cursor);
                    }
                })
                .response
                .on_hover_text("Use this cursor when hovering buttons etc");

            ui.checkbox(image_loading_spinners, "Image loading spinners")
                .on_hover_text("Show a spinner when an Image is loading");

            ui.horizontal(|ui| {
                ui.label("Color picker type");
                numeric_color_space.toggle_button_ui(ui);
            });
        });

        ui.vertical_centered(|ui| reset_button(ui, self, "Reset visuals"));
    }
}

impl TextCursorStyle {
    fn ui(&mut self, ui: &mut Ui) {
        let Self {
            stroke,
            preview,
            blink,
            on_duration,
            off_duration,
        } = self;

        ui.horizontal(|ui| {
            ui.label("Stroke");
            ui.add(stroke);
        });

        ui.checkbox(preview, "Preview text cursor on hover");

        ui.checkbox(blink, "Blink");

        if *blink {
            Grid::new("cursor_blink").show(ui, |ui| {
                ui.label("On time");
                ui.add(
                    DragValue::new(on_duration)
                        .speed(0.1)
                        .clamp_range(0.0..=2.0)
                        .suffix(" s"),
                );
                ui.end_row();

                ui.label("Off time");
                ui.add(
                    DragValue::new(off_duration)
                        .speed(0.1)
                        .clamp_range(0.0..=2.0)
                        .suffix(" s"),
                );
                ui.end_row();
            });
        }
    }
}

#[cfg(debug_assertions)]
impl DebugOptions {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            debug_on_hover,
            debug_on_hover_with_all_modifiers,
            hover_shows_next,
            show_expand_width,
            show_expand_height,
            show_resize,
            show_interactive_widgets,
            show_widget_hits,
        } = self;

        {
            ui.checkbox(debug_on_hover, "Show widget info on hover.");
            ui.checkbox(
                debug_on_hover_with_all_modifiers,
                "Show widget info on hover if holding all modifier keys",
            );

            ui.checkbox(hover_shows_next, "Show next widget placement on hover");
        }

        ui.checkbox(
            show_expand_width,
            "Show which widgets make their parent wider",
        );
        ui.checkbox(
            show_expand_height,
            "Show which widgets make their parent higher",
        );
        ui.checkbox(show_resize, "Debug Resize");

        ui.checkbox(
            show_interactive_widgets,
            "Show an overlay on all interactive widgets",
        );

        ui.checkbox(show_widget_hits, "Show widgets under mouse pointer");

        ui.vertical_centered(|ui| reset_button(ui, self, "Reset debug options"));
    }
}

// TODO(emilk): improve and standardize
fn two_drag_values(value: &mut Vec2, range: std::ops::RangeInclusive<f32>) -> impl Widget + '_ {
    move |ui: &mut crate::Ui| {
        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut value.x)
                    .clamp_range(range.clone())
                    .prefix("x: "),
            );
            ui.add(
                DragValue::new(&mut value.y)
                    .clamp_range(range.clone())
                    .prefix("y: "),
            );
        })
        .response
    }
}

fn ui_color(ui: &mut Ui, color: &mut Color32, label: impl Into<WidgetText>) -> Response {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba(color);
        ui.label(label);
    })
    .response
}

fn ui_text_color(ui: &mut Ui, color: &mut Color32, label: impl Into<RichText>) -> Response {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba(color);
        ui.label(label.into().color(*color));
    })
    .response
}

impl HandleShape {
    pub fn ui(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Slider handle");
            ui.radio_value(self, Self::Circle, "Circle");
            if ui
                .radio(matches!(self, Self::Rect { .. }), "Rectangle")
                .clicked()
            {
                *self = Self::Rect { aspect_ratio: 0.5 };
            }
            if let Self::Rect { aspect_ratio } = self {
                ui.add(Slider::new(aspect_ratio, 0.1..=3.0).text("Aspect ratio"));
            }
        });
    }
}

/// How to display numeric color values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub enum NumericColorSpace {
    /// RGB is 0-255 in gamma space.
    ///
    /// Alpha is 0-255 in linear space .
    GammaByte,

    /// 0-1 in linear space.
    Linear,
    // TODO(emilk): add Hex as an option
}

impl NumericColorSpace {
    pub fn toggle_button_ui(&mut self, ui: &mut Ui) -> crate::Response {
        let tooltip = match self {
            Self::GammaByte => "Showing color values in 0-255 gamma space",
            Self::Linear => "Showing color values in 0-1 linear space",
        };

        let mut response = ui.button(self.to_string()).on_hover_text(tooltip);
        if response.clicked() {
            *self = match self {
                Self::GammaByte => Self::Linear,
                Self::Linear => Self::GammaByte,
            };
            response.mark_changed();
        }
        response
    }
}

impl std::fmt::Display for NumericColorSpace {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::GammaByte => write!(f, "U8"),
            Self::Linear => write!(f, "F"),
        }
    }
}

impl Widget for &mut Margin {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut same = self.is_same();

        let response = if same {
            ui.horizontal(|ui| {
                ui.checkbox(&mut same, "same");

                let mut value = self.left;
                ui.add(DragValue::new(&mut value));
                *self = Margin::same(value);
            })
            .response
        } else {
            ui.vertical(|ui| {
                ui.checkbox(&mut same, "same");

                crate::Grid::new("margin").num_columns(2).show(ui, |ui| {
                    ui.label("Left");
                    ui.add(DragValue::new(&mut self.left));
                    ui.end_row();

                    ui.label("Right");
                    ui.add(DragValue::new(&mut self.right));
                    ui.end_row();

                    ui.label("Top");
                    ui.add(DragValue::new(&mut self.top));
                    ui.end_row();

                    ui.label("Bottom");
                    ui.add(DragValue::new(&mut self.bottom));
                    ui.end_row();
                });
            })
            .response
        };

        // Apply the checkbox:
        if same {
            *self = Margin::same((self.left + self.right + self.top + self.bottom) / 4.0);
        } else if self.is_same() {
            self.right *= 1.00001; // prevent collapsing into sameness
        }

        response
    }
}

impl Widget for &mut Rounding {
    fn ui(self, ui: &mut Ui) -> Response {
        let mut same = self.is_same();

        let response = if same {
            ui.horizontal(|ui| {
                ui.checkbox(&mut same, "same");

                let mut cr = self.nw;
                ui.add(DragValue::new(&mut cr).clamp_range(0.0..=f32::INFINITY));
                *self = Rounding::same(cr);
            })
            .response
        } else {
            ui.vertical(|ui| {
                ui.checkbox(&mut same, "same");

                crate::Grid::new("rounding").num_columns(2).show(ui, |ui| {
                    ui.label("NW");
                    ui.add(DragValue::new(&mut self.nw).clamp_range(0.0..=f32::INFINITY));
                    ui.end_row();

                    ui.label("NE");
                    ui.add(DragValue::new(&mut self.ne).clamp_range(0.0..=f32::INFINITY));
                    ui.end_row();

                    ui.label("SW");
                    ui.add(DragValue::new(&mut self.sw).clamp_range(0.0..=f32::INFINITY));
                    ui.end_row();

                    ui.label("SE");
                    ui.add(DragValue::new(&mut self.se).clamp_range(0.0..=f32::INFINITY));
                    ui.end_row();
                });
            })
            .response
        };

        // Apply the checkbox:
        if same {
            *self = Rounding::same((self.nw + self.ne + self.sw + self.se) / 4.0);
        } else if self.is_same() {
            self.se *= 1.00001; // prevent collapsing into sameness
        }

        response
    }
}

impl Widget for &mut Shadow {
    fn ui(self, ui: &mut Ui) -> Response {
        let epaint::Shadow {
            offset,
            blur,
            spread,
            color,
        } = self;

        ui.vertical(|ui| {
            crate::Grid::new("shadow_ui").show(ui, |ui| {
                ui.add(
                    DragValue::new(&mut offset.x)
                        .speed(1.0)
                        .clamp_range(-100.0..=100.0)
                        .prefix("x: "),
                );
                ui.add(
                    DragValue::new(&mut offset.y)
                        .speed(1.0)
                        .clamp_range(-100.0..=100.0)
                        .prefix("y: "),
                );
                ui.end_row();

                ui.add(
                    DragValue::new(blur)
                        .speed(1.0)
                        .clamp_range(0.0..=100.0)
                        .prefix("blur: "),
                );

                ui.add(
                    DragValue::new(spread)
                        .speed(1.0)
                        .clamp_range(0.0..=100.0)
                        .prefix("spread: "),
                );
            });
            ui.color_edit_button_srgba(color);
        })
        .response
    }
}

impl Widget for &mut Stroke {
    fn ui(self, ui: &mut Ui) -> Response {
        let Stroke { width, color } = self;

        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(width)
                    .speed(0.1)
                    .clamp_range(0.0..=f32::INFINITY),
            )
            .on_hover_text("Width");
            ui.color_edit_button_srgba(color);

            // stroke preview:
            let (_id, stroke_rect) = ui.allocate_space(ui.spacing().interact_size);
            let left = stroke_rect.left_center();
            let right = stroke_rect.right_center();
            ui.painter().line_segment([left, right], (*width, *color));
        })
        .response
    }
}

impl Widget for &mut crate::Frame {
    fn ui(self, ui: &mut Ui) -> Response {
        let crate::Frame {
            inner_margin,
            outer_margin,
            rounding,
            shadow,
            fill,
            stroke,
        } = self;

        crate::Grid::new("frame")
            .num_columns(2)
            .spacing([12.0, 8.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label("Inner margin");
                ui.add(inner_margin);
                ui.end_row();

                ui.label("Outer margin");
                ui.add(outer_margin);
                ui.end_row();

                ui.label("Rounding");
                ui.add(rounding);
                ui.end_row();

                ui.label("Shadow");
                ui.add(shadow);
                ui.end_row();

                ui.label("Fill");
                ui.color_edit_button_srgba(fill);
                ui.end_row();

                ui.label("Stroke");
                ui.add(stroke);
                ui.end_row();
            })
            .response
    }
}
