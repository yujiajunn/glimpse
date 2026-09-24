// 阻止额外的 console 窗口
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    glimpse_lib::run()
}