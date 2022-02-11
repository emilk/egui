//! egui theme (spacing, colors, etc).

#![allow(clippy::if_same_then_else)]

use crate::{color::*, emath::*, FontFamily, FontId, Response, RichText, WidgetText};
use epaint::{mutex::Arc, Rounding, Shadow, Stroke};
use std::collections::BTreeMap;

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

    /// Same size as [`Self::Body]`, but used when monospace is important (for aligning number, code snippets, etc).
    Monospace,

    /// Buttons. Maybe slightly bigger than [`Self::Body]`.
    /// Signifies that he item is interactive.
    Button,

    /// Heading. Probably larger than [`Self::Body]`.
    Heading,

    /// A user-chosen style, found in [`Style::text_styles`].
    /// ```
    /// egui::TextStyle::Name("footing".into());
    /// ````
    Name(Arc<str>),
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
    pub text_styles: BTreeMap<TextStyle, FontId>,

    /// If set, labels buttons wtc will use this to determine whether or not
    /// to wrap the text at the right edge of the `Ui` they are in.
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
    pub debug: DebugOptions,

    /// Show tooltips explaining `DragValue`:s etc when hovered.
    ///
    /// This only affects a few egui widgets.
    pub explanation_tooltips: bool,
}

impl Style {
    // TODO: rename style.interact() to maybe... `style.interactive` ?
    /// Use this style for interactive things.
    /// Note that you must already have a response,
    /// i.e. you must allocate space and interact BEFORE painting the widget!
    pub fn interact(&self, response: &Response) -> &WidgetVisuals {
        self.visuals.widgets.style(response)
    }

    pub fn interact_selectable(&self, response: &Response, selected: bool) -> WidgetVisuals {
        let mut visuals = *self.visuals.widgets.style(response);
        if selected {
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

    /// Indent collapsing regions etc by this much.
    pub indent: f32,

    /// Minimum size of a `DragValue`, color picker button, and other small widgets.
    /// `interact_size.y` is the default height of button, slider, etc.
    /// Anything clickable should be (at least) this size.
    pub interact_size: Vec2, // TODO: rename min_interact_size ?

    /// Default width of a `Slider` and `ComboBox`.
    pub slider_width: f32, // TODO: rename big_interact_size ?

    /// Default width of a `TextEdit`.
    pub text_edit_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the width/height of this icon.
    pub icon_width: f32,

    /// Checkboxes, radio button and collapsing headers have an icon at the start.
    /// This is the spacing between the icon and the text
    pub icon_spacing: f32,

    /// Width of a tooltip (`on_hover_ui`, `on_hover_text` etc).
    pub tooltip_width: f32,

    /// End indented regions with a horizontal line
    pub indent_ends_with_horizontal_line: bool,

    /// Height of a combo-box before showing scroll bars.
    pub combo_height: f32,

    pub scroll_bar_width: f32,
}

impl Spacing {
    /// Returns small icon rectangle and big icon rectangle
    pub fn icon_rectangles(&self, rect: Rect) -> (Rect, Rect) {
        let box_side = self.icon_width;
        let big_icon_rect = Rect::from_center_size(
            pos2(rect.left() + box_side / 2.0, rect.center().y),
            vec2(box_side, box_side),
        );

        let small_rect_side = 8.0; // TODO: make a parameter
        let small_icon_rect =
            Rect::from_center_size(big_icon_rect.center(), Vec2::splat(small_rect_side));

        (small_icon_rect, big_icon_rect)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct Margin {
    pub left: f32,
    pub right: f32,
    pub top: f32,
    pub bottom: f32,
}

impl Margin {
    #[inline]
    pub fn same(margin: f32) -> Self {
        Self {
            left: margin,
            right: margin,
            top: margin,
            bottom: margin,
        }
    }

    /// Margins with the same size on opposing sides
    #[inline]
    pub fn symmetric(x: f32, y: f32) -> Self {
        Self {
            left: x,
            right: x,
            top: y,
            bottom: y,
        }
    }

    /// Total margins on both sides
    pub fn sum(&self) -> Vec2 {
        Vec2::new(self.left + self.right, self.top + self.bottom)
    }
}

impl From<Vec2> for Margin {
    fn from(v: Vec2) -> Self {
        Self::symmetric(v.x, v.y)
    }
}

/// How and when interaction happens.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
#[cfg_attr(feature = "serde", serde(default))]
pub struct Interaction {
    /// Mouse must be this close to the side of a window to resize
    pub resize_grab_radius_side: f32,

    /// Mouse must be this close to the corner of a window to resize
    pub resize_grab_radius_corner: f32,

    /// If `false`, tooltips will show up anytime you hover anything, even is mouse is still moving
    pub show_tooltips_only_when_still: bool,
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

    /// The color used for `Hyperlink`,
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

    pub window_rounding: Rounding,
    pub window_shadow: Shadow,

    pub popup_shadow: Shadow,

    pub resize_corner_size: f32,

    pub text_cursor_width: f32,
    /// show where the text cursor would be if you clicked
    pub text_cursor_preview: bool,

    /// Allow child widgets to be just on the border and still have a stroke with some thickness
    pub clip_rect_margin: f32,

    /// Show a background behind buttons.
    pub button_frame: bool,

    /// Show a background behind collapsing headers.
    pub collapsing_header_frame: bool,
}

impl Visuals {
    #[inline(always)]
    pub fn noninteractive(&self) -> &WidgetVisuals {
        &self.widgets.noninteractive
    }

    pub fn text_color(&self) -> Color32 {
        self.override_text_color
            .unwrap_or_else(|| self.widgets.noninteractive.text_color())
    }

    pub fn weak_text_color(&self) -> Color32 {
        crate::color::tint_color_towards(self.text_color(), self.window_fill())
    }

    #[inline(always)]
    pub fn strong_text_color(&self) -> Color32 {
        self.widgets.active.text_color()
    }

    /// Window background color.
    #[inline(always)]
    pub fn window_fill(&self) -> Color32 {
        self.widgets.noninteractive.bg_fill
    }

    #[inline(always)]
    pub fn window_stroke(&self) -> Stroke {
        self.widgets.noninteractive.bg_stroke
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
    /// The style of an interactive widget while you hover it.
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
        } else if response.is_pointer_button_down_on() || response.has_focus() {
            &self.active
        } else if response.hovered() {
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
    /// Background color of widget.
    pub bg_fill: Color32,

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
#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Deserialize, serde::Serialize))]
pub struct DebugOptions {
    /// However over widgets to see their rectangles
    pub debug_on_hover: bool,
    /// Show which widgets make their parent wider
    pub show_expand_width: bool,
    /// Show which widgets make their parent higher
    pub show_expand_height: bool,
    pub show_resize: bool,
}

// ----------------------------------------------------------------------------

/// The default text styles of the default egui theme.
pub fn default_text_styles() -> BTreeMap<TextStyle, FontId> {
    let mut text_styles = BTreeMap::new();
    text_styles.insert(
        TextStyle::Small,
        FontId::new(10.0, FontFamily::Proportional),
    );
    text_styles.insert(TextStyle::Body, FontId::new(14.0, FontFamily::Proportional));
    text_styles.insert(
        TextStyle::Button,
        FontId::new(14.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Heading,
        FontId::new(20.0, FontFamily::Proportional),
    );
    text_styles.insert(
        TextStyle::Monospace,
        FontId::new(14.0, FontFamily::Monospace),
    );
    text_styles
}

impl Default for Style {
    fn default() -> Self {
        Self {
            override_font_id: None,
            override_text_style: None,
            text_styles: default_text_styles(),
            wrap: None,
            spacing: Spacing::default(),
            interaction: Interaction::default(),
            visuals: Visuals::default(),
            animation_time: 1.0 / 12.0,
            debug: Default::default(),
            explanation_tooltips: false,
        }
    }
}

impl Default for Spacing {
    fn default() -> Self {
        Self {
            item_spacing: vec2(8.0, 3.0),
            window_margin: Margin::same(6.0),
            button_padding: vec2(4.0, 1.0),
            indent: 18.0, // match checkbox/radio-button with `button_padding.x + icon_width + icon_spacing`
            interact_size: vec2(40.0, 18.0),
            slider_width: 100.0,
            text_edit_width: 280.0,
            icon_width: 14.0,
            icon_spacing: 0.0,
            tooltip_width: 600.0,
            combo_height: 200.0,
            scroll_bar_width: 8.0,
            indent_ends_with_horizontal_line: false,
        }
    }
}

impl Default for Interaction {
    fn default() -> Self {
        Self {
            resize_grab_radius_side: 5.0,
            resize_grab_radius_corner: 10.0,
            show_tooltips_only_when_still: false,
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
            faint_bg_color: Color32::from_gray(24),
            extreme_bg_color: Color32::from_gray(10), // e.g. TextEdit background
            code_bg_color: Color32::from_gray(64),
            window_rounding: Rounding::same(6.0),
            window_shadow: Shadow::big_dark(),
            popup_shadow: Shadow::small_dark(),
            resize_corner_size: 12.0,
            text_cursor_width: 2.0,
            text_cursor_preview: false,
            clip_rect_margin: 3.0, // should be at least half the size of the widest frame stroke + max WidgetVisuals::expansion
            button_frame: true,
            collapsing_header_frame: false,
        }
    }

    /// Default light theme.
    pub fn light() -> Self {
        Self {
            dark_mode: false,
            widgets: Widgets::light(),
            selection: Selection::light(),
            hyperlink_color: Color32::from_rgb(0, 155, 255),
            faint_bg_color: Color32::from_gray(245),
            extreme_bg_color: Color32::from_gray(255), // e.g. TextEdit background
            code_bg_color: Color32::from_gray(230),
            window_shadow: Shadow::big_light(),
            popup_shadow: Shadow::small_light(),
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
                bg_fill: Color32::from_gray(27), // window background
                bg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // separators, indentation lines, windows outlines
                fg_stroke: Stroke::new(1.0, Color32::from_gray(140)), // normal text color
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: Color32::from_gray(60), // button background
                bg_stroke: Default::default(),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(180)), // button text
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from_gray(70),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(150)), // e.g. hover over window edge or button
                fg_stroke: Stroke::new(1.5, Color32::from_gray(240)),
                rounding: Rounding::same(3.0),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from_gray(55),
                bg_stroke: Stroke::new(1.0, Color32::WHITE),
                fg_stroke: Stroke::new(2.0, Color32::WHITE),
                rounding: Rounding::same(2.0),
                expansion: 1.0,
            },
            open: WidgetVisuals {
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
                bg_fill: Color32::from_gray(248), // window background - should be distinct from TextEdit background
                bg_stroke: Stroke::new(1.0, Color32::from_gray(190)), // separators, indentation lines, windows outlines
                fg_stroke: Stroke::new(1.0, Color32::from_gray(80)),  // normal text color
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            inactive: WidgetVisuals {
                bg_fill: Color32::from_gray(230), // button background
                bg_stroke: Default::default(),
                fg_stroke: Stroke::new(1.0, Color32::from_gray(60)), // button text
                rounding: Rounding::same(2.0),
                expansion: 0.0,
            },
            hovered: WidgetVisuals {
                bg_fill: Color32::from_gray(220),
                bg_stroke: Stroke::new(1.0, Color32::from_gray(105)), // e.g. hover over window edge or button
                fg_stroke: Stroke::new(1.5, Color32::BLACK),
                rounding: Rounding::same(3.0),
                expansion: 1.0,
            },
            active: WidgetVisuals {
                bg_fill: Color32::from_gray(165),
                bg_stroke: Stroke::new(1.0, Color32::BLACK),
                fg_stroke: Stroke::new(2.0, Color32::BLACK),
                rounding: Rounding::same(2.0),
                expansion: 1.0,
            },
            open: WidgetVisuals {
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
            wrap: _,
            spacing,
            interaction,
            visuals,
            animation_time,
            debug,
            explanation_tooltips,
        } = self;

        visuals.light_dark_radio_buttons(ui);

        crate::Grid::new("_options").show(ui, |ui| {
            ui.label("Override font id:");
            ui.horizontal(|ui| {
                ui.radio_value(override_font_id, None, "None");
                if ui.radio(override_font_id.is_some(), "override").clicked() {
                    *override_font_id = Some(FontId::default());
                }
                if let Some(override_font_id) = override_font_id {
                    crate::introspection::font_id_ui(ui, override_font_id);
                }
            });
            ui.end_row();

            ui.label("Override text style:");
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

            ui.label("Animation duration:");
            ui.add(
                Slider::new(animation_time, 0.0..=1.0)
                    .clamp_to_range(true)
                    .suffix(" s"),
            );
            ui.end_row();
        });

        ui.collapsing("🔠 Text Styles", |ui| text_styles_ui(ui, text_styles));
        ui.collapsing("📏 Spacing", |ui| spacing.ui(ui));
        ui.collapsing("☝ Interaction", |ui| interaction.ui(ui));
        ui.collapsing("🎨 Visuals", |ui| visuals.ui(ui));
        ui.collapsing("🐛 Debug", |ui| debug.ui(ui));

        ui.checkbox(explanation_tooltips, "Explanation tooltips")
            .on_hover_text(
                "Show explanatory text when hovering DragValue:s and other egui widgets",
            );

        ui.vertical_centered(|ui| reset_button(ui, self));
    }
}

fn text_styles_ui(ui: &mut Ui, text_styles: &mut BTreeMap<TextStyle, FontId>) -> Response {
    ui.vertical(|ui| {
        crate::Grid::new("text_styles").show(ui, |ui| {
            for (text_style, font_id) in text_styles.iter_mut() {
                ui.label(RichText::new(text_style.to_string()).font(font_id.clone()));
                crate::introspection::font_id_ui(ui, font_id);
                ui.end_row();
            }
        });
        crate::reset_button_with(ui, text_styles, default_text_styles());
    })
    .response
}

impl Spacing {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            item_spacing,
            window_margin,
            button_padding,
            indent,
            interact_size,
            slider_width,
            text_edit_width,
            icon_width,
            icon_spacing,
            tooltip_width,
            indent_ends_with_horizontal_line,
            combo_height,
            scroll_bar_width,
        } = self;

        ui.add(slider_vec2(item_spacing, 0.0..=20.0, "Item spacing"));

        let margin_range = 0.0..=20.0;
        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut window_margin.left)
                    .clamp_range(margin_range.clone())
                    .prefix("left: "),
            );
            ui.add(
                DragValue::new(&mut window_margin.right)
                    .clamp_range(margin_range.clone())
                    .prefix("right: "),
            );

            ui.label("Window margins x");
        });

        ui.horizontal(|ui| {
            ui.add(
                DragValue::new(&mut window_margin.top)
                    .clamp_range(margin_range.clone())
                    .prefix("top: "),
            );
            ui.add(
                DragValue::new(&mut window_margin.bottom)
                    .clamp_range(margin_range)
                    .prefix("bottom: "),
            );
            ui.label("Window margins y");
        });

        ui.add(slider_vec2(button_padding, 0.0..=20.0, "Button padding"));
        ui.add(slider_vec2(interact_size, 4.0..=60.0, "Interact size"))
            .on_hover_text("Minimum size of an interactive widget");
        ui.horizontal(|ui| {
            ui.add(DragValue::new(indent).clamp_range(0.0..=100.0));
            ui.label("Indent");
        });
        ui.horizontal(|ui| {
            ui.add(DragValue::new(slider_width).clamp_range(0.0..=1000.0));
            ui.label("Slider width");
        });
        ui.horizontal(|ui| {
            ui.add(DragValue::new(text_edit_width).clamp_range(0.0..=1000.0));
            ui.label("TextEdit width");
        });
        ui.horizontal(|ui| {
            ui.add(DragValue::new(scroll_bar_width).clamp_range(0.0..=32.0));
            ui.label("Scroll-bar width width");
        });

        ui.horizontal(|ui| {
            ui.label("Checkboxes etc:");
            ui.add(
                DragValue::new(icon_width)
                    .prefix("width:")
                    .clamp_range(0.0..=60.0),
            );
            ui.add(
                DragValue::new(icon_spacing)
                    .prefix("spacing:")
                    .clamp_range(0.0..=10.0),
            );
        });

        ui.horizontal(|ui| {
            ui.add(DragValue::new(tooltip_width).clamp_range(0.0..=1000.0));
            ui.label("Tooltip wrap width");
        });

        ui.checkbox(
            indent_ends_with_horizontal_line,
            "End indented regions with a horizontal separator",
        );

        ui.horizontal(|ui| {
            ui.label("Max height of a combo box");
            ui.add(DragValue::new(combo_height).clamp_range(0.0..=1000.0));
        });

        ui.vertical_centered(|ui| reset_button(ui, self));
    }
}

impl Interaction {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            resize_grab_radius_side,
            resize_grab_radius_corner,
            show_tooltips_only_when_still,
        } = self;
        ui.add(Slider::new(resize_grab_radius_side, 0.0..=20.0).text("resize_grab_radius_side"));
        ui.add(
            Slider::new(resize_grab_radius_corner, 0.0..=20.0).text("resize_grab_radius_corner"),
        );
        ui.checkbox(
            show_tooltips_only_when_still,
            "Only show tooltips if mouse is still",
        );

        ui.vertical_centered(|ui| reset_button(ui, self));
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
        ui_color(ui, bg_fill, "background fill");
        stroke_ui(ui, stroke, "stroke");
    }
}

impl WidgetVisuals {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            bg_fill,
            bg_stroke,
            rounding,
            fg_stroke,
            expansion,
        } = self;
        ui_color(ui, bg_fill, "background fill");
        stroke_ui(ui, bg_stroke, "background stroke");

        rounding_ui(ui, rounding);

        stroke_ui(ui, fg_stroke, "foreground stroke (text)");
        ui.add(Slider::new(expansion, -5.0..=5.0).text("expansion"))
            .on_hover_text("make shapes this much larger");
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
            window_rounding,
            window_shadow,
            popup_shadow,
            resize_corner_size,
            text_cursor_width,
            text_cursor_preview,
            clip_rect_margin,
            button_frame,
            collapsing_header_frame,
        } = self;

        ui.collapsing("Background Colors", |ui| {
            ui_color(ui, &mut widgets.inactive.bg_fill, "Buttons");
            ui_color(ui, &mut widgets.noninteractive.bg_fill, "Windows");
            ui_color(ui, faint_bg_color, "Faint accent").on_hover_text(
                "Used for faint accentuation of interactive things, like striped grids.",
            );
            ui_color(ui, extreme_bg_color, "Extreme")
                .on_hover_text("Background of plots and paintings");
        });

        ui.collapsing("Window", |ui| {
            // Common shortcuts
            ui_color(ui, &mut widgets.noninteractive.bg_fill, "Fill");
            stroke_ui(ui, &mut widgets.noninteractive.bg_stroke, "Outline");

            rounding_ui(ui, window_rounding);

            shadow_ui(ui, window_shadow, "Shadow");
            shadow_ui(ui, popup_shadow, "Shadow (small menus and popups)");
        });

        ui.collapsing("Widgets", |ui| widgets.ui(ui));
        ui.collapsing("Selection", |ui| selection.ui(ui));

        ui_color(
            ui,
            &mut widgets.noninteractive.fg_stroke.color,
            "Text color",
        );
        ui_color(ui, code_bg_color, RichText::new("Code background").code()).on_hover_ui(|ui| {
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = 0.0;
                ui.label("For monospaced inlined text ");
                ui.code("like this");
                ui.label(".");
            });
        });

        ui_color(ui, hyperlink_color, "hyperlink_color");
        ui.add(Slider::new(resize_corner_size, 0.0..=20.0).text("resize_corner_size"));
        ui.add(Slider::new(text_cursor_width, 0.0..=4.0).text("text_cursor_width"));
        ui.checkbox(text_cursor_preview, "Preview text cursor on hover");
        ui.add(Slider::new(clip_rect_margin, 0.0..=20.0).text("clip_rect_margin"));

        ui.checkbox(button_frame, "Button has a frame");
        ui.checkbox(collapsing_header_frame, "Collapsing header has a frame");

        ui.vertical_centered(|ui| reset_button(ui, self));
    }
}

impl DebugOptions {
    pub fn ui(&mut self, ui: &mut crate::Ui) {
        let Self {
            debug_on_hover,
            show_expand_width: debug_expand_width,
            show_expand_height: debug_expand_height,
            show_resize: debug_resize,
        } = self;

        ui.checkbox(debug_on_hover, "Show debug info on hover");
        ui.checkbox(
            debug_expand_width,
            "Show which widgets make their parent wider",
        );
        ui.checkbox(
            debug_expand_height,
            "Show which widgets make their parent higher",
        );
        ui.checkbox(debug_resize, "Debug Resize");

        ui.vertical_centered(|ui| reset_button(ui, self));
    }
}

// TODO: improve and standardize `slider_vec2`
fn slider_vec2<'a>(
    value: &'a mut Vec2,
    range: std::ops::RangeInclusive<f32>,
    text: &'a str,
) -> impl Widget + 'a {
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
            ui.label(text);
        })
        .response
    }
}

fn ui_color(ui: &mut Ui, srgba: &mut Color32, label: impl Into<WidgetText>) -> Response {
    ui.horizontal(|ui| {
        ui.color_edit_button_srgba(srgba);
        ui.label(label);
    })
    .response
}

fn rounding_ui(ui: &mut Ui, rounding: &mut Rounding) {
    const MAX: f32 = 20.0;
    let mut same = rounding.is_same();
    ui.group(|ui| {
        ui.horizontal(|ui| {
            ui.label("Rounding: ");
            ui.radio_value(&mut same, true, "Same");
            ui.radio_value(&mut same, false, "Separate");
        });

        if same {
            let mut cr = rounding.nw;
            ui.add(Slider::new(&mut cr, 0.0..=MAX));
            *rounding = Rounding::same(cr);
        } else {
            ui.add(Slider::new(&mut rounding.nw, 0.0..=MAX).text("North-West"));
            ui.add(Slider::new(&mut rounding.ne, 0.0..=MAX).text("North-East"));
            ui.add(Slider::new(&mut rounding.sw, 0.0..=MAX).text("South-West"));
            ui.add(Slider::new(&mut rounding.se, 0.0..=MAX).text("South-East"));
            if rounding.is_same() {
                rounding.se *= 1.00001;
            }
        }
    });
}
