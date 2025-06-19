# Layout tutorials

## Make group fill full height of its parent

```rust
ui.group(|ui| {
    // Stuff blah blah blah
    ui.set_height(ui.available_height());
});
```
