// Vo Windows release builde neotvárať popri aplikácii aj konzolové okno.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    bank_analyzer_lib::run()
}
