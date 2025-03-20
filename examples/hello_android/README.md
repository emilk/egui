Hello world example for Android.

Use `cargo-apk` to build and run. Requires a patch to workaround [an upstream bug](https://github.com/rust-mobile/cargo-subcommand/issues/29).

One-time setup:

```sh
cargo install \
    --git https://github.com/parasyte/cargo-apk.git \
    --rev 282639508eeed7d73f2e1eaeea042da2716436d5 \
    cargo-apk
```

Build and run:

```sh
# Run on android
cargo apk run -p hello_android --lib

# Run on your desktop
cargo run -p hello_android
```

![](screenshot.png)
