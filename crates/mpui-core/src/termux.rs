use std::process::Command;

/// Check if running within a Termux environment.
pub fn is_termux() -> bool {
    std::env::var("TERMUX_VERSION").is_ok()
        || std::path::Path::new("/data/data/com.termux").exists()
}

/// Acquire an Android wake lock via termux-api to enable seamless background playback.
pub fn acquire_wake_lock() {
    if !is_termux() {
        return;
    }
    let _ = Command::new("termux-wake-lock").spawn();
}

/// Release the Android wake lock via termux-api.
pub fn release_wake_lock() {
    if !is_termux() {
        return;
    }
    let _ = Command::new("termux-wake-unlock").spawn();
}

/// Send a native Android notification via termux-notification.
pub fn send_notification(title: &str, content: &str) {
    if !is_termux() {
        return;
    }
    let _ = Command::new("termux-notification")
        .args([
            "--title",
            title,
            "--content",
            content,
            "--id",
            "mpui_playback",
            "--priority",
            "low",
        ])
        .spawn();
}
