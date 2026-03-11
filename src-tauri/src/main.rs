// src-tauri/src/main.rs
// Thin entry point – all logic lives in lib.rs
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    vocab_trainer_lib::run();
}
