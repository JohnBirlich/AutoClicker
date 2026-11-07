#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(any(target_os = "windows", test))]
mod clicker;

#[cfg(target_os = "windows")]
mod windows_app;

fn main() {
    #[cfg(target_os = "windows")]
    windows_app::run();

    #[cfg(not(target_os = "windows"))]
    eprintln!("AutoClicker is a Windows desktop application. Build it for Windows to run it.");
}
