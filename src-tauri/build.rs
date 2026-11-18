fn main() {
    // macOS: Info.plist (the microphone line) is read when the app is compiled; rebuild when it changes.
    println!("cargo:rerun-if-changed=Info.plist");
    tauri_build::build()
}
