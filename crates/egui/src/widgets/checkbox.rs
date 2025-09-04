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
    Shape,       // Geometric shapes for drawing (lines, rectangles, etc.)
    Ui,          // Main UI context
    Vec2,        // 2D vector for positions and sizes
    Widget,      // Trait that all widgets must implement
    WidgetInfo,  // Accessibility and debugging information about widgets
    WidgetType,  // Enum identifying the type of widget
    epaint,      // Low-level painting/drawing functionality
    pos2,        // Function to create 2D position coordinates
};

// TODO(emilk): allow checkbox without a text label
/// A boolean on/off control widget with an optional text label.
///
/// Checkboxes allow users to toggle between two states: checked (true) and unchecked (false).
/// Unlike radio buttons which are mutually exclusive, checkboxes are independent and can be
/// used for multiple selections or individual boolean settings.
///
/// The checkbox displays as a square box that shows a checkmark when selected, and can also
/// display an indeterminate state (horizontal line) for tri-state logic.
///
/// ## Usage
/// Usually you'd use the convenience method [`Ui::checkbox`] instead of creating this
/// widget directly, but this struct gives you more control over the behavior.
///
/// ## Examples
/// ```
/// # egui::__run_test_ui(|ui| {
/// # let mut my_bool = true;
/// // Simple convenience method (recommended for most cases)
/// ui.checkbox(&mut my_bool, "Checked");
///
/// // Equivalent manual creation:
/// ui.add(egui::Checkbox::new(&mut my_bool, "Checked"));
///
/// // Checkbox without text label:
/// ui.add(egui::Checkbox::without_text(&mut my_bool));
///
/// // Indeterminate checkbox (shows horizontal line instead of checkmark):
/// ui.add(egui::Checkbox::new(&mut my_bool, "Maybe").indeterminate(true));
/// # });
/// ```
///
/// ## Visual States
/// - **Unchecked**: Empty square box
/// - **Checked**: Square box with checkmark
/// - **Indeterminate**: Square box with horizontal line (for tri-state logic)
#[must_use = "You should put this widget in a ui with `ui.add(widget);`"]
pub struct Checkbox<'a> {
    /// Mutable reference to the boolean value this checkbox controls
    /// When clicked, this value will be toggled (!*checked)
    checked: &'a mut bool,

    /// The content atoms that make up this checkbox (typically text label)
    /// Uses a lifetime parameter 'a to avoid unnecessary string allocations
    atoms: Atoms<'a>,

    /// Whether to display in indeterminate state (horizontal line instead of checkmark)
    /// Useful for tri-state logic where the checkbox represents "some of many" selected
    indeterminate: bool,
}

impl<'a> Checkbox<'a> {
    /// Creates a new checkbox with the specified boolean reference and content.
    ///
    /// # Parameters
    /// - `checked`: Mutable reference to a boolean that this checkbox will control.
    ///   When the checkbox is clicked, this value will be toggled.
    /// - `atoms`: The content to display next to the checkbox (usually text label).
    ///   This accepts anything that implements `IntoAtoms`, such as `&str`, `String`, etc.
    ///
    /// # Returns
    /// A new `Checkbox` instance ready to be added to a UI
    ///
    /// # Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut enabled = false;
    /// let checkbox = egui::Checkbox::new(&mut enabled, "Enable feature");
    /// if ui.add(checkbox).clicked() {
    ///     println!("Checkbox toggled! New value: {}", enabled);
    /// }
    /// # });
    /// ```
    pub fn new(checked: &'a mut bool, atoms: impl IntoAtoms<'a>) -> Self {
        Checkbox {
            checked,
            atoms: atoms.into_atoms(), // Convert the input into the internal Atoms representation
            indeterminate: false,      // Default to normal checked/unchecked behavior
        }
    }

    /// Creates a checkbox without any text label.
    ///
    /// This is useful when you want a standalone checkbox, perhaps in a table cell
    /// or when the context makes the purpose clear without additional text.
    ///
    /// # Parameters
    /// - `checked`: Mutable reference to the boolean this checkbox controls
    ///
    /// # Returns
    /// A new `Checkbox` instance with no text content
    ///
    /// # Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut selected = true;
    /// ui.add(egui::Checkbox::without_text(&mut selected));
    /// # });
    /// ```
    pub fn without_text(checked: &'a mut bool) -> Self {
        Self::new(checked, ()) // Empty tuple converts to empty atoms
    }

    /// Sets the checkbox to display in indeterminate state.
    ///
    /// In indeterminate state, the checkbox shows a horizontal line instead of a checkmark,
    /// indicating a "mixed" or "partially selected" state. This is commonly used in
    /// hierarchical selections where some but not all child items are selected.
    ///
    /// # Parameters
    /// - `indeterminate`: Whether to show indeterminate state (horizontal line)
    ///
    /// # Important Note
    /// This only affects the checkbox's **visual appearance**. The underlying boolean
    /// value still behaves normally - clicking will still toggle between true/false.
    /// The indeterminate state is purely visual feedback for the user.
    ///
    /// # Use Cases
    /// - Parent checkbox in a tree where some children are selected
    /// - "Select All" checkbox when only some items are selected
    /// - Any tri-state logic display (true/false/mixed)
    ///
    /// # Example
    /// ```
    /// # egui::__run_test_ui(|ui| {
    /// let mut parent_selected = false;
    /// let some_children_selected = true; // Some logic determines this
    ///
    /// ui.add(
    ///     egui::Checkbox::new(&mut parent_selected, "Select All")
    ///         .indeterminate(some_children_selected && !parent_selected)
    /// );
    /// # });
    /// ```
    #[inline]
    pub fn indeterminate(mut self, indeterminate: bool) -> Self {
        self.indeterminate = indeterminate;
        self
    }
}

/// Implementation of the Widget trait, which defines how the checkbox is rendered and behaves
impl Widget for Checkbox<'_> {
    /// Main method that handles the widget's layout, interaction, and rendering
    ///
    /// # Parameters
    /// - `ui`: Mutable reference to the UI context where this widget will be displayed
    ///
    /// # Returns
    /// A `Response` containing information about user interactions (clicks, hovers, etc.)
    ///
    /// # Process Overview
    /// 1. Calculate sizing for the checkbox icon and overall widget
    /// 2. Set up the atom layout with the checkbox icon on the left
    /// 3. Handle user interaction (clicking toggles the boolean)
    /// 4. Set up accessibility information
    /// 5. If visible, draw the checkbox background and appropriate state indicator
    fn ui(self, ui: &mut Ui) -> Response {
        // Destructure self to get owned values (required since ui() consumes self)
        let Checkbox {
            checked,
            mut atoms,
            indeterminate,
        } = self;

        // Get spacing configuration from the UI style
        let spacing = &ui.spacing();

        // Calculate the width of the checkbox icon based on UI spacing settings
        let icon_width = spacing.icon_width;

        // Set minimum size for the entire widget
        // Start with a square based on the standard interaction size height
        let mut min_size = Vec2::splat(spacing.interact_size.y);
        // Ensure the height is at least as tall as the icon width for proper visual balance
        min_size.y = min_size.y.at_least(icon_width);

        // Calculate the actual size of the checkbox icon
        // Start with a square icon based on the icon width
        let mut icon_size = Vec2::splat(icon_width);
        // Ensure the icon height matches the minimum widget height for proper centering
        icon_size.y = icon_size.y.at_least(min_size.y);

        // Create a unique identifier for the checkbox's square icon area
        // This ID is used internally for layout and hit-testing
        let rect_id = Id::new("egui::checkbox");

        // Add the checkbox icon as a custom atom to the left side of the widget
        // This reserves space for the square checkbox graphic
        atoms.push_left(Atom::custom(rect_id, icon_size));

        // Extract text content from atoms for accessibility purposes
        // Convert to owned String if text exists, otherwise None
        let text = atoms.text().map(String::from);

        // Create the layout for all atoms and handle user interaction
        let mut prepared = AtomLayout::new(atoms)
            .sense(Sense::click()) // Make the widget respond to mouse clicks
            .min_size(min_size) // Set the minimum size we calculated
            .allocate(ui); // Reserve space in the UI and get interaction response

        // Handle click interaction - toggle the boolean value
        if prepared.response.clicked() {
            *checked = !*checked; // Toggle the boolean value
            prepared.response.mark_changed(); // Mark this as a state change for UI framework
        }

        // Set up accessibility information for screen readers and debugging tools
        prepared.response.widget_info(|| {
            if indeterminate {
                // For indeterminate state, we can't use "selected" since it's neither true nor false
                // Just provide a labeled checkbox without selection state
                WidgetInfo::labeled(
                    WidgetType::Checkbox,          // Identify this as a checkbox
                    ui.is_enabled(),               // Whether the widget is interactive
                    text.as_deref().unwrap_or(""), // Text label or empty string
                )
            } else {
                // For normal checked/unchecked state, provide full selection information
                WidgetInfo::selected(
                    WidgetType::Checkbox,          // Identify this as a checkbox
                    ui.is_enabled(),               // Whether the widget is interactive
                    *checked,                      // Current checked state
                    text.as_deref().unwrap_or(""), // Text label or empty string
                )
            }
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

            // Draw the checkbox icon if we have a rectangle allocated for it
            if let Some(rect) = response.rect(rect_id) {
                // Calculate two concentric rectangles for the checkbox:
                // - big_icon_rect: outer square (background and border)
                // - small_icon_rect: inner area (for checkmark or indeterminate line)
                let (small_icon_rect, big_icon_rect) = ui.spacing().icon_rectangles(rect);

                // Draw the outer square (background square with border)
                ui.painter().add(epaint::RectShape::new(
                    big_icon_rect.expand(visuals.expansion), // Rectangle with visual expansion
                    visuals.corner_radius,                   // Corner rounding from theme
                    visuals.bg_fill,                         // Background fill color
                    visuals.bg_stroke,                       // Border color and width
                    epaint::StrokeKind::Inside,              // Draw stroke inside the rectangle
                ));

                // Draw the appropriate state indicator inside the checkbox
                if indeterminate {
                    // Indeterminate state: draw horizontal line across the middle
                    ui.painter().add(Shape::hline(
                        small_icon_rect.x_range(),  // Horizontal span of the line
                        small_icon_rect.center().y, // Vertical position (center)
                        visuals.fg_stroke,          // Line color and thickness
                    ));
                } else if *checked {
                    // Checked state: draw a checkmark
                    // The checkmark is drawn as a polyline with three points forming a "✓" shape
                    ui.painter().add(Shape::line(
                        vec![
                            // Start point: left side, vertically centered
                            pos2(small_icon_rect.left(), small_icon_rect.center().y),
                            // Middle point: center horizontally, bottom vertically (the "knee" of the checkmark)
                            pos2(small_icon_rect.center().x, small_icon_rect.bottom()),
                            // End point: right side, top (completing the checkmark)
                            pos2(small_icon_rect.right(), small_icon_rect.top()),
                        ],
                        visuals.fg_stroke, // Line color and thickness
                    ));
                }
                // If unchecked and not indeterminate, we draw nothing inside (just the empty square)
            }

            // Return the response from the painted widget
            response.response
        } else {
            // Widget is not visible, so just return the basic response without rendering
            prepared.response
        }
    }
}
