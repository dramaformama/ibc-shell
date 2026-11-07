//! iBC Shell desktop entry point.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    ibc_shell_lib::run();
}
