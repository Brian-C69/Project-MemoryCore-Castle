// No console window behind Castle in a release build.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    memorycore_castle_lib::run()
}
