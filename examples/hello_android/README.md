# Hello world example for Android.

## Desktop pre-requisites

Android application require you to have toolchains and Android SDK installed:

1. `rustup target add armv7-linux-androideabi aarch64-linux-android` - install targets for android
2. Set environment variables (these variables are required each time for `cargo apk`):
  ```
  export ANDROID_HOME="$HOME/tools/android"
  export ANDROID_NDK_ROOT="${ANDROID_HOME}/ndk/29.0.14206865"
  export PATH="$PATH:${ANDROID_NDK_ROOT}:${ANDROID_HOME}/build-tools/${BUILDTOOLS_VERSION}:${ANDROID_HOME}/cmdline-tools/bin"
  ```
3. Install command line tools:
  ```
  mkdir -p "${ANDROID_HOME}/cmdline-tools"
  curl -sLo /tmp/clt.zip https://dl.google.com/android/repository/commandlinetools-linux-14742923_latest.zip
  unzip -d "${ANDROID_HOME}" /tmp/clt.zip
  ```
4. Install SDK components: `sdkmanager --sdk_root="${ANDROID_HOME}" --install "build-tools;36.0.0" "ndk;29.0.14206865" "platforms;android-35"`
  > You may need to change SDK versions
5. Install cargo-apk: `cargo install --git https://github.com/parasyte/cargo-apk.git --rev 282639508eeed7d73f2e1eaeea042da2716436d5 cargo-apk`
  > There was a [bug](https://github.com/rust-mobile/cargo-subcommand/issues/29) in the upstream, so the above installs a patched version. 

## Build & run

Use `cargo-apk` to build and run:
1. On android: `cargo apk run -p hello_android --lib`
2. On desktop: `cargo run -p hello_android`

![](screenshot.png)
