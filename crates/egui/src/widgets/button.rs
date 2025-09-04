// Import necessary types and traits from the crate
use crate::{
    Atom,               // Individual UI element that can be laid out
    AtomExt as _,       // Extension traits for atoms (imported with underscore)
    AtomKind,           // Enum specifying the type of atom (Text, Image, etc.)
    AtomLayout,         // Layout manager for arranging atoms horizontally/vertically
    AtomLayoutResponse, // Response from atom layout operations with painting capabilities
    Color32,            // 32-bit color representation (RGBA)
    CornerRadius,       // Struct defining corner rounding for rectangles
    Frame,              // Container with background, borders, and margins
    Image,              // Image atom for displaying pictures/icons
    IntoAtoms,          // Trait for converting types into Atoms collection
    NumExt as _,        // Numeric extension traits
    Response,           // UI interaction response (clicks, hovers, etc.)
    Sense,              // What types of interactions the widget responds to
    Stroke,             // Line style (color, width) for borders
    TextStyle,          // Predefined text styling (Button, Body, Heading, etc.)
    TextWrapMode,       // How text should wrap (Wrap, Truncate, Extend)
    Ui,                 // Main UI context
    Vec2,               // 2D vector for positions and sizes
    Widget,             // Trait that all widgets must implement
    WidgetInfo,         // Accessibility and debugging information
    WidgetText,         // Text content for widgets
    WidgetType,         // Enum identifying the type of widget
};

/// A clickable button widget that can contain text, images, or both.
///
/// Buttons are one of the most fundamental UI elements, providing user interaction
/// through clicks, drags, or other input methods. This implementation supports
/// extensive customization including colors, sizing, frames, and content layout.
///
/// ## Basic Usage
/// The simplest way to create buttons is through [`Ui::button`], but this struct
/// provides much more control over appearance and behavior.
///
/// ## Examples
/// ```
/// # egui::__run_test_ui(|ui| {
/// # fn do_stuff() {}
///
/// // Simple text button
/// if ui.add(egui::Button::new("Click me")).clicked() {
///     do_stuff();
/// }
///
/// // Disabled button (greyed-out and non-interactive)
/// if ui.add_enabled(false, egui::Button::new("Can't click this")).clicked() {
///     unreachable!(); // This will never execute because button is disabled
/// }
///
/// // Button with custom styling
/// if ui.add(
///     egui::Button::new("Styled button")
///         .fill(egui::Color32::BLUE)
///         .corner_radius(10.0)
///         .min_size(egui::Vec2::new(100.0, 40.0))
/// ).clicked() {
///     // Handle click
/// }
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Button<'a> {
    /// The layout manager that handles arrangement of atoms (text, images, etc.)
    layout: AtomLayout<'a>,

    /// Optional override for background fill color
    /// If None, uses the theme's default button colors
    fill: Option<Color32>,

    /// Optional override for border stroke (color and width)
    /// If None, uses the theme's default button border
    stroke: Option<Stroke>,

    /// Whether this is a small button suitable for inline text embedding
    /// Small buttons have reduced vertical padding and minimum size
    small: bool,

    /// Optional override for whether to show a frame/background
    /// None = use theme default, Some(true/false) = force on/off
    frame: Option<bool>,

    /// Whether to show frame when button is not being interacted with
    /// Only relevant when frames are enabled
    frame_when_inactive: bool,

    /// Minimum size the button should occupy, regardless of content
    min_size: Vec2,

    /// Optional override for corner rounding
    /// If None, uses the theme's default button corner radius
    corner_radius: Option<CornerRadius>,

    /// Whether this button should appear in a "selected" state
    /// Affects coloring and visual feedback
    selected: bool,

    /// Whether image tint should match the text color
    /// Useful for icon buttons where you want the icon to change color on hover
    image_tint_follows_text_color: bool,

    /// Whether to limit image size to font height for consistent appearance
    /// Used by image-based button constructors
    limit_image_size: bool,
}

impl<'a> Button<'a> {
    /// Creates a new button with the specified content.
    ///
    /// This is the most flexible constructor - it accepts anything that can be
    /// converted into atoms (text, images, or combinations thereof).
    ///
    /// # Parameters
    /// - `atoms`: The content to display (typically text like `"Click me"`)
    ///
    /// # Returns
    /// A new `Button` instance with default settings
    ///
    /// # Example
    /// ```
    /// let button = egui::Button::new("My Button");
    /// let button2 = egui::Button::new(egui::WidgetText::from("Styled text"));
    /// ```
    pub fn new(atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            // Set up the layout with click sensing and button font styling
            layout: AtomLayout::new(atoms.into_atoms())
                .sense(Sense::click()) // Respond to mouse clicks
                .fallback_font(TextStyle::Button), // Use button text style as fallback

            // Initialize all customization options to defaults
            fill: None,                           // Use theme default fill
            stroke: None,                         // Use theme default stroke
            small: false,                         // Normal-sized button
            frame: None,                          // Use theme default frame setting
            frame_when_inactive: true,            // Show frame even when not hovered
            min_size: Vec2::ZERO,                 // No minimum size override
            corner_radius: None,                  // Use theme default corner radius
            selected: false,                      // Not selected by default
            image_tint_follows_text_color: false, // Images keep their original color
            limit_image_size: false,              // Don't limit image size by default
        }
    }

    /// Creates a selectable button that can be toggled on/off.
    ///
    /// This is a convenience constructor for creating buttons that represent
    /// a binary state (like toggle buttons or option selectors).
    ///
    /// # Parameters
    /// - `selected`: Whether the button should appear in selected state
    /// - `atoms`: The content to display on the button
    ///
    /// # Returns
    /// A configured button with appropriate selection styling
    ///
    /// # Equivalent to:
    /// ```rust
    /// # use egui::{Button, IntoAtoms, __run_test_ui};
    /// # __run_test_ui(|ui| {
    /// let selected = true;
    /// ui.add(Button::new("toggle me").selected(selected).frame_when_inactive(!selected).frame(true));
    /// # });
    /// ```
    ///
    /// # See also:
    /// - [`Ui::selectable_value`] - For enum/value selection
    /// - [`Ui::selectable_label`] - For simpler selectable text
    pub fn selectable(selected: bool, atoms: impl IntoAtoms<'a>) -> Self {
        Self::new(atoms)
            .selected(selected) // Set selection state
            .frame_when_inactive(selected) // Only show frame when selected and inactive
            .frame(true) // Always enable frame for selectables
    }

    /// Creates a button containing only an image.
    ///
    /// The image size is automatically limited to the default font height
    /// for consistent appearance with text elements.
    ///
    /// # Parameters
    /// - `image`: The image to display (can be from various sources)
    ///
    /// # Note
    /// Unlike [`Button::new`], this automatically enables image size limiting
    /// to ensure the button integrates well with text-based UI elements.
    pub fn image(image: impl Into<Image<'a>>) -> Self {
        Self::opt_image_and_text(Some(image.into()), None)
    }

    /// Creates a button with an image on the left and text on the right.
    ///
    /// This is commonly used for menu items, toolbar buttons, and other
    /// UI elements that benefit from both visual and textual representation.
    ///
    /// # Parameters
    /// - `image`: The image to display on the left side
    /// - `text`: The text to display on the right side
    ///
    /// # Note
    /// The image size is automatically limited to font height for consistency.
    pub fn image_and_text(image: impl Into<Image<'a>>, text: impl Into<WidgetText>) -> Self {
        Self::opt_image_and_text(Some(image.into()), Some(text.into()))
    }

    /// Creates a button with optional image and/or text content.
    ///
    /// This is the most flexible constructor for mixed content buttons.
    /// Either parameter can be None to create image-only or text-only buttons.
    ///
    /// # Parameters
    /// - `image`: Optional image to display (positioned left)
    /// - `text`: Optional text to display (positioned right of image)
    ///
    /// # Note
    /// Image size limiting is automatically enabled for consistent appearance.
    pub fn opt_image_and_text(image: Option<Image<'a>>, text: Option<WidgetText>) -> Self {
        // Start with an empty button (no default content)
        let mut button = Self::new(());

        // Add image to the right side of layout if provided
        if let Some(image) = image {
            button.layout.push_right(image);
        }

        // Add text to the right side of layout if provided (after image)
        if let Some(text) = text {
            button.layout.push_right(text);
        }

        // Enable automatic image size limiting for font-height consistency
        button.limit_image_size = true;
        button
    }

    /// Sets the text wrapping mode for text content in the button.
    ///
    /// By default, buttons use the UI's wrap mode setting, which can be
    /// overridden globally via [`crate::Style::wrap_mode`].
    ///
    /// # Parameters
    /// - `wrap_mode`: How text should behave when it exceeds button width
    ///
    /// # Note
    /// Newline characters (`\n`) in the text will always create line breaks
    /// regardless of the wrap mode setting.
    #[inline]
    pub fn wrap_mode(mut self, wrap_mode: TextWrapMode) -> Self {
        self.layout = self.layout.wrap_mode(wrap_mode);
        self
    }

    /// Convenience method to enable text wrapping.
    ///
    /// Sets [`Self::wrap_mode`] to [`TextWrapMode::Wrap`], allowing text
    /// to flow to multiple lines when the button is too narrow.
    #[inline]
    pub fn wrap(self) -> Self {
        self.wrap_mode(TextWrapMode::Wrap)
    }

    /// Convenience method to enable text truncation.
    ///
    /// Sets [`Self::wrap_mode`] to [`TextWrapMode::Truncate`], causing
    /// text to be cut off with "..." when it doesn't fit.
    #[inline]
    pub fn truncate(self) -> Self {
        self.wrap_mode(TextWrapMode::Truncate)
    }

    /// Override the background fill color of the button.
    ///
    /// This completely replaces theme-based coloring, including hover effects.
    /// The button frame is automatically enabled when using custom fill.
    ///
    /// # Parameters
    /// - `fill`: The color to use for the button background
    ///
    /// # Warning
    /// This will override **all** hover and interaction effects for the background.
    /// Consider using theme customization instead for better user experience.
    #[inline]
    pub fn fill(mut self, fill: impl Into<Color32>) -> Self {
        self.fill = Some(fill.into());
        self
    }

    /// Override the border stroke (outline) of the button.
    ///
    /// This replaces theme-based border styling, including hover effects.
    /// The button frame is automatically enabled when using custom stroke.
    ///
    /// # Parameters
    /// - `stroke`: The stroke style (color and width) for the button border
    ///
    /// # Warning
    /// This will override **all** hover and interaction effects for the border.
    #[inline]
    pub fn stroke(mut self, stroke: impl Into<Stroke>) -> Self {
        self.stroke = Some(stroke.into());
        self.frame = Some(true); // Automatically enable frame for custom stroke
        self
    }

    /// Makes this a small button suitable for embedding within text.
    ///
    /// Small buttons have:
    /// - No vertical padding
    /// - No minimum height requirement
    /// - Reduced visual weight for inline use
    ///
    /// # Use cases
    /// - Inline buttons within paragraphs
    /// - Compact toolbar buttons
    /// - Secondary actions that shouldn't dominate the UI
    #[inline]
    pub fn small(mut self) -> Self {
        self.small = true;
        self
    }

    /// Controls whether the button has a visible frame (background/border).
    ///
    /// # Parameters
    /// - `frame`: true to force frame on, false to force frame off
    ///
    /// # Note
    /// When disabled, the button appears as plain text until hovered.
    /// This is useful for creating subtle buttons that don't dominate the UI.
    #[inline]
    pub fn frame(mut self, frame: bool) -> Self {
        self.frame = Some(frame);
        self
    }

    /// Controls frame visibility when the button is not being interacted with.
    ///
    /// When set to `false`, the frame only appears on hover/click, creating
    /// a more subtle appearance for inactive buttons.
    ///
    /// # Parameters
    /// - `frame_when_inactive`: Whether to show frame when not interacting
    ///
    /// # Default
    /// `true` - frames are always visible when enabled
    ///
    /// # Note
    /// This setting has no effect when [`Self::frame`] or `ui.visuals().button_frame`
    /// is `false`.
    #[inline]
    pub fn frame_when_inactive(mut self, frame_when_inactive: bool) -> Self {
        self.frame_when_inactive = frame_when_inactive;
        self
    }

    /// Changes the interaction sensing mode of the button.
    ///
    /// By default, buttons respond to clicks. You can change this to create
    /// drag buttons, hover-sensitive buttons, or other interaction patterns.
    ///
    /// # Parameters
    /// - `sense`: The type of interaction to detect
    ///
    /// # Examples
    /// - `Sense::click()` - Normal button (default)
    /// - `Sense::drag()` - Drag button for sliders or draggable items
    /// - `Sense::hover()` - Button that responds to mouse hover
    #[inline]
    pub fn sense(mut self, sense: Sense) -> Self {
        self.layout = self.layout.sense(sense);
        self
    }

    /// Sets the minimum size the button should occupy.
    ///
    /// The button will be at least this large, even if the content is smaller.
    /// This is useful for creating consistent button sizes or ensuring
    /// touch-friendly target sizes.
    ///
    /// # Parameters
    /// - `min_size`: Minimum width and height in UI units
    ///
    /// # Example
    /// ```
    /// # use egui::{Button, Vec2};
    /// let button = Button::new("OK").min_size(Vec2::new(80.0, 30.0));
    /// ```
    #[inline]
    pub fn min_size(mut self, min_size: Vec2) -> Self {
        self.min_size = min_size;
        self
    }

    /// Sets the corner rounding of the button.
    ///
    /// This allows creating buttons with rounded corners, from slightly
    /// rounded rectangles to pill-shaped buttons.
    ///
    /// # Parameters
    /// - `corner_radius`: How much to round the corners (can be uniform or per-corner)
    ///
    /// # Examples
    /// ```
    /// # use egui::{Button, CornerRadius};
    /// let rounded_button = Button::new("Rounded").corner_radius(8.0);
    /// let pill_button = Button::new("Pill").corner_radius(CornerRadius::same(100.0));
    /// ```
    #[inline]
    pub fn corner_radius(mut self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius = Some(corner_radius.into());
        self
    }

    /// Deprecated alias for [`corner_radius`].
    ///
    /// This method has been renamed for clarity. Use `corner_radius` instead.
    #[inline]
    #[deprecated = "Renamed to `corner_radius`"]
    pub fn rounding(self, corner_radius: impl Into<CornerRadius>) -> Self {
        self.corner_radius(corner_radius)
    }

    /// Controls whether image tint follows the text color.
    ///
    /// When enabled, any images in the button will be tinted to match
    /// the current text color, including hover state changes.
    ///
    /// # Parameters
    /// - `image_tint_follows_text_color`: Whether to tint images to match text
    ///
    /// # Use cases
    /// - White/monochrome icons that should match theme colors
    /// - Icons that should change color on hover
    /// - Maintaining consistent color schemes across text and icons
    ///
    /// # Default
    /// `false` - images retain their original colors
    #[inline]
    pub fn image_tint_follows_text_color(mut self, image_tint_follows_text_color: bool) -> Self {
        self.image_tint_follows_text_color = image_tint_follows_text_color;
        self
    }

    /// Adds shortcut text on the right side of the button in a weak/subdued color.
    ///
    /// This is specifically designed for menu buttons to display keyboard
    /// shortcuts (like "Ctrl+S", "F1", etc.) in a visually secondary way.
    ///
    /// # Parameters
    /// - `shortcut_text`: The shortcut text to display (often created with
    ///   [`crate::Context::format_shortcut`])
    ///
    /// # Layout
    /// The shortcut text is positioned on the far right with a flexible
    /// space separator, creating a typical menu item appearance:
    /// `[Button Text          Ctrl+S]`
    ///
    /// # See also
    /// [`Self::right_text`] for normal-colored right-aligned text
    #[inline]
    pub fn shortcut_text(mut self, shortcut_text: impl Into<Atom<'a>>) -> Self {
        let mut atom = shortcut_text.into();

        // Convert the atom to weak/subdued styling if it's text
        atom.kind = match atom.kind {
            AtomKind::Text(text) => AtomKind::Text(text.weak()), // Apply weak styling
            other => other,                                      // Non-text atoms remain unchanged
        };

        // Add flexible space to push shortcut to the right
        self.layout.push_right(Atom::grow());
        // Add the styled shortcut text
        self.layout.push_right(atom);
        self
    }

    /// Adds normal text on the right side of the button.
    ///
    /// Unlike [`Self::shortcut_text`], this displays the text in normal
    /// color/weight, making it more prominent.
    ///
    /// # Parameters
    /// - `right_text`: The text to display on the right side
    ///
    /// # Layout
    /// Similar to shortcut text but with normal styling:
    /// `[Button Text          Right Text]`
    #[inline]
    pub fn right_text(mut self, right_text: impl Into<Atom<'a>>) -> Self {
        // Add flexible space to push text to the right
        self.layout.push_right(Atom::grow());
        // Add the text with normal styling
        self.layout.push_right(right_text.into());
        self
    }

    /// Marks this button as selected or unselected.
    ///
    /// Selected buttons typically appear with different coloring to indicate
    /// their state, similar to pressed or active buttons.
    ///
    /// # Parameters
    /// - `selected`: Whether the button should appear selected
    ///
    /// # Use cases
    /// - Toggle buttons showing on/off state
    /// - Tab buttons showing active tab
    /// - Tool palette showing selected tool
    #[inline]
    pub fn selected(mut self, selected: bool) -> Self {
        self.selected = selected;
        self
    }

    /// Renders the button and returns a detailed response for custom painting.
    ///
    /// This is the main rendering method that handles layout, interaction,
    /// and drawing. It returns an [`AtomLayoutResponse`] which includes
    /// both the user interaction response and information for custom painting.
    ///
    /// # Parameters
    /// - `ui`: The UI context to render into
    ///
    /// # Returns
    /// An [`AtomLayoutResponse`] containing:
    /// - `response`: User interaction information (clicks, hovers, etc.)
    /// - Layout information for custom drawing
    ///
    /// # Process
    /// 1. Calculate sizing and layout
    /// 2. Handle image size limiting if enabled
    /// 3. Allocate space in the UI
    /// 4. Apply visual styling based on interaction state
    /// 5. Draw the button frame and content
    /// 6. Set up accessibility information
    pub fn atom_ui(self, ui: &mut Ui) -> AtomLayoutResponse {
        // Destructure self to get owned values
        let Button {
            mut layout,
            fill,
            stroke,
            small,
            frame,
            frame_when_inactive,
            mut min_size,
            corner_radius,
            selected,
            image_tint_follows_text_color,
            limit_image_size,
        } = self;

        // Calculate minimum size based on button type
        if !small {
            // Normal buttons have minimum height for touch-friendly interaction
            min_size.y = min_size.y.at_least(ui.spacing().interact_size.y);
        }
        // Small buttons can be as small as their content

        // Apply image size limiting if requested
        if limit_image_size {
            layout.map_atoms(|atom| {
                if matches!(&atom.kind, AtomKind::Image(_)) {
                    // Limit image height to font size for consistency
                    atom.atom_max_height_font_size(ui)
                } else {
                    atom // Non-image atoms pass through unchanged
                }
            });
        }

        // Extract text for accessibility purposes
        let text = layout.text().map(String::from);

        // Determine if button should have frame/background
        let has_frame_margin = frame.unwrap_or_else(|| ui.visuals().button_frame);

        // Calculate padding based on frame presence
        let mut button_padding = if has_frame_margin {
            ui.spacing().button_padding // Normal padding for framed buttons
        } else {
            Vec2::ZERO // No padding for frameless buttons
        };

        // Small buttons get no vertical padding
        if small {
            button_padding.y = 0.0;
        }

        // Set up layout with frame and allocate space
        let mut prepared = layout
            .frame(Frame::new().inner_margin(button_padding)) // Add padding frame
            .min_size(min_size) // Apply minimum size
            .allocate(ui); // Reserve space and get interaction

        // Only render if the button is visible on screen (optimization)
        let response = if ui.is_rect_visible(prepared.response.rect) {
            // Get visual styling based on interaction state and selection
            let visuals = ui.style().interact_selectable(&prepared.response, selected);

            // Determine when to show the frame
            let visible_frame = if frame_when_inactive {
                // Always show frame when enabled
                has_frame_margin
            } else {
                // Only show frame during interaction
                has_frame_margin
                    && (prepared.response.hovered()                    // Mouse over
                        || prepared.response.is_pointer_button_down_on() // Mouse pressed
                        || prepared.response.has_focus()) // Keyboard focus
            };

            // Apply image tinting if enabled
            if image_tint_follows_text_color {
                prepared.map_images(|image| image.tint(visuals.text_color()));
            }

            // Set fallback text color from visual styling
            prepared.fallback_text_color = visuals.text_color();

            // Draw the frame if it should be visible
            if visible_frame {
                // Use custom stroke or theme default
                let stroke = stroke.unwrap_or(visuals.bg_stroke);
                // Use custom fill or theme default
                let fill = fill.unwrap_or(visuals.weak_bg_fill);

                // Configure the frame appearance
                prepared.frame = prepared
                    .frame
                    // Adjust inner margin for visual expansion and stroke width
                    .inner_margin(
                        button_padding + Vec2::splat(visuals.expansion) - Vec2::splat(stroke.width),
                    )
                    // Adjust outer margin for visual expansion
                    .outer_margin(-Vec2::splat(visuals.expansion))
                    // Apply colors and styling
                    .fill(fill)
                    .stroke(stroke)
                    .corner_radius(corner_radius.unwrap_or(visuals.corner_radius));
            }

            // Paint the button and get the final response
            prepared.paint(ui)
        } else {
            // Button is not visible, return empty response to save processing
            AtomLayoutResponse::empty(prepared.response)
        };

        // Set up accessibility information for screen readers
        response.response.widget_info(|| {
            if let Some(text) = &text {
                // Button with text label
                WidgetInfo::labeled(WidgetType::Button, ui.is_enabled(), text)
            } else {
                // Button without text (image-only, etc.)
                WidgetInfo::new(WidgetType::Button)
            }
        });

        response
    }
}

/// Implementation of the Widget trait for standard UI integration
impl Widget for Button<'_> {
    /// Renders the button and returns a standard interaction response.
    ///
    /// This is the standard Widget trait implementation that most users
    /// will interact with through `ui.add(button)`.
    ///
    /// # Parameters
    /// - `ui`: The UI context to render into
    ///
    /// # Returns
    /// A [`Response`] containing user interaction information
    ///
    /// # Note
    /// This is a convenience wrapper around [`Button::atom_ui`] that
    /// extracts just the interaction response.
    fn ui(self, ui: &mut Ui) -> Response {
        self.atom_ui(ui).response
    }
}
