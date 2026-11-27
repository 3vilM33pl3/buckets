// SPDX-License-Identifier: MIT
// Hide the console window on Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    buckets_desktop_lib::run()
}
