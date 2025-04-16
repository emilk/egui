.DEFAULT_GOAL := apk
.PHONY: jni apk run-on-device

jni:
	cd rust && cargo ndk --target arm64-v8a -o java/app/src/main/jniLibs/ build --profile release

apk: jni
	cd java && ./gradlew build

run-on-device: jni
	cd java && ./gradlew installDebug
	adb shell am start -n local.walkers/.MainActivity
	adb logcat -v color -s walkers RustStdoutStderr
