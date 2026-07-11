#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Some(result) = tokenstack_lib::packaged_smoke_from_process() {
        match result {
            Ok(_) => std::process::exit(0),
            Err(_) => std::process::exit(2),
        }
    }
    tokenstack_lib::run();
}
