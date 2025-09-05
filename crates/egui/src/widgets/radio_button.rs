// Import necessary types and traits from the crate
use crate::{
    Atom,        // Individual UI element that can be laid out
    AtomLayout,  // Layout manager for atoms
    Atoms,       // Collection of atoms
    Id,          // Unique identifier for UI elements
    IntoAtoms,   // Trait for converting types into Atoms
    NumExt as _, // Numeric extension traits (imported with underscore to avoid naming conflicts)
    Response,    // UI interaction response (clicks, hovers, etc.)
    Sense,       // What types of interactions the widget responds to
    Ui,          // Main UI context
    Vec2,        // 2D vector for positions and sizes
    Widget,      // Trait that all widgets must implement
    WidgetInfo,  // Accessibility and debugging information about widgets
    WidgetType,  // Enum identifying the type of widget
    epaint,      // Low-level painting/drawing functionality
};

/// A radio button widget representing one option out of several mutually exclusive alternatives.
/// The radio button can be either selected (checked) or not selected (unchecked).
///
/// Radio buttons are typically used in groups where only one option can be selected at a time,
/// unlike checkboxes which allow multiple selections.
///
/// ## Usage
/// Usually you'd use the convenience methods [`Ui::radio_value`] or [`Ui::radio`] instead
/// of creating this widget directly, but this struct gives you more control over the behavior.
///
/// ## Example
/// ```
/// # egui::__run_test_ui(|ui| {
/// // Define an enum for our radio button options
/// #[derive(PartialEq)]
/// enum Enum { First, Second, Third }
/// let mut my_enum = Enum::First;
///
/// // Simple way using ui.radio_value (recommended)
/// ui.radio_value(&mut my_enum, Enum::First, "First");
///
/// // Equivalent manual way using RadioButton directly:
/// if ui.add(egui::RadioButton::new(my_enum == Enum::First, "First")).clicked() {
///     my_enum = Enum::First
/// }
/// # });
/// ```
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct RadioButton<'a> {
    /// Whether this radio button is currently selected/checked
    checked: bool,

    /// The content atoms that make up this radio button (typically text label)
    /// Uses a lifetime parameter 'a to avoid unnecessary string allocations
    atoms: Atoms<'a>,
}

impl<'a> RadioButton<'a> {
    /// Creates a new radio button with the specified checked state and content.
    ///
    /// # Parameters
    /// - `checked`: Whether the radio button should appear selected
    /// - `atoms`: The content to display (usually text, but can be other UI elements)
    ///   This accepts anything that implements `IntoAtoms`, such as `&str`, `String`, etc.
    ///
    /// # Returns
    /// A new `RadioButton` instance ready to be added to a UI
    ///
    /// # Example
    /// ```
    /// let radio = egui::RadioButton::new(true, "Selected option");
    /// let radio2 = egui::RadioButton::new(false, "Unselected option");
    /// ```
    pub fn new(checked: bool, atoms: impl IntoAtoms<'a>) -> Self {
        Self {
            checked,
            atoms: atoms.into_atoms(), // Convert the input into the internal Atoms representation
        }
    }
}

/// Implementation of the Widget trait, which defines how the radio button is rendered and behaves
impl Widget for RadioButton<'_> {
    /// Main method that handles the widget's layout, interaction, and rendering
    ///
    /// # Parameters
    /// - `ui`: Mutable reference to the UI context where this widget will be displayed
    ///
    /// # Returns
    /// A `Response` containing information about user interactions (clicks, hovers, etc.)
    fn ui(self, ui: &mut Ui) -> Response {
        // Destructure self to get owned values (required since ui() consumes self)
        let Self { checked, mut atoms } = self;

        // Get spacing configuration from the UI style
        let spacing = &ui.spacing();

        // Calculate the width of the radio button icon based on UI spacing settings
        let icon_width = spacing.icon_width;

        // Set minimum size for the entire widget
        // Start with a square based on the standard interaction size height
        let mut min_size = Vec2::splat(spacing.interact_size.y);
        // Ensure the height is at least as tall as the icon width for proper visual balance
        min_size.y = min_size.y.at_least(icon_width);

        // Calculate the actual size of the radio button icon
        // Start with a square icon based on the icon width
        let mut icon_size = Vec2::splat(icon_width);
        // Ensure the icon height matches the minimum widget height for proper centering
        icon_size.y = icon_size.y.at_least(min_size.y);

        // Create a unique identifier for the radio button's circular icon area
        // This ID is used internally for layout and hit-testing
        let rect_id = Id::new("egui::radio_button");

        // Add the radio button icon as a custom atom to the left side of the widget
        // This reserves space for the circular radio button graphic
        atoms.push_left(Atom::custom(rect_id, icon_size));

        // Extract text content from atoms for accessibility purposes
        // Convert to owned String if text exists, otherwise None
        let text = atoms.text().map(String::from);

        // Create the layout for all atoms and handle user interaction
        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click()) // Make the widget respond to mouse clicks
            .min_size(min_size) // Set the minimum size we calculated
            .allocate(ui); // Reserve space in the UI and get interaction response

        // Set up accessibility information for screen readers and debugging tools
        prepared.response.widget_info(|| {
            WidgetInfo::selected(
                WidgetType::RadioButton,       // Identify this as a radio button
                ui.is_enabled(),               // Whether the widget is interactive
                checked,                       // Current selection state
                text.as_deref().unwrap_or(""), // Text label or empty string
            )
        });

        // Only proceed with rendering if the widget is actually visible on screen
        // This is an optimization to avoid unnecessary drawing operations
        if ui.is_rect_visible(prepared.response.rect) {
            // Get visual styling based on the widget's interaction state
            // Note: We use the general interact() method instead of interact_selectable()
            // because the selectable version can be "too colorful" according to the comment
            let visuals = *ui.style().interact(&prepared.response);

            // Set the fallback text color for any text atoms
            prepared.fallback_text_color = visuals.text_color();

            // Paint the widget content (text and other atoms) and get the final response
            let response = prepared.paint(ui);

            // Draw the radio button icon if we have a rectangle allocated for it
            if let Some(rect) = response.rect(rect_id) {
                // Calculate two concentric rectangles for the radio button:
                // - big_icon_rect: outer circle (background and border)
                // - small_icon_rect: inner filled circle (when selected)
                let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

                // Get the painter object for drawing shapes
                let painter = ui.painter();

                // Draw the outer circle (background circle with border)
                painter.add(epaint::CircleShape {
                    center: big_icon_rect.center(), // Center point
                    radius: big_icon_rect.width() / 2.0 + visuals.expansion, // Radius with visual expansion
                    fill: visuals.bg_fill,     // Background fill color
                    stroke: visuals.bg_stroke, // Border color and width
                });

                // If this radio button is selected, draw the inner filled circle
                if checked {
                    painter.add(epaint::CircleShape {
                        center: small_icon_rect.center(),      // Same center as outer circle
                        radius: small_icon_rect.width() / 3.0, // Much smaller radius for inner dot
                        fill: visuals.fg_stroke.color, // Use foreground stroke color for fill
                        // Note: Intentionally using stroke color instead of fill color
                        // visuals.selection.stroke.color would be "too much color"
                        stroke: Default::default(), // No border on inner circle
                    });
                }
            }

            // Return the response from the painted widget
            response.response
        } else {
            // Widget is not visible, so just return the basic response without rendering
            prepared.response
        }
    }
}
