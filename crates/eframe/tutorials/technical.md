# Technical tutorials

## Press a button when a key is pressed

If you want to have something that happens when either:

- a button is clicked, or
- a key is pressed

This is how you can do that:

```rust
let my_button = ui.button("Press me");

if my_button.clicked() || ui.input(|i| i.key_pressed(egui::Key::Enter)) {
    // Do awesome stuff
}
```

Now, whenever `my_button` is clicked _or_ the `Enter` key is pressed, they will do the same thing.
