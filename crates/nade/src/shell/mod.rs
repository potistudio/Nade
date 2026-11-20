//! Window chrome around the panel grid: title strip, toolbar, status bar
//! and transient toasts.

pub mod menu;
#[cfg(target_os = "macos")]
pub mod native_menu;
pub mod status_bar;
pub mod title_bar;
pub mod toast;
pub mod toolbar;
