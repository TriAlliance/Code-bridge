//! Notification display implementations

use crate::{Notification, NotificationError, Result};

/// Show a notification using platform-native API
pub async fn show_notification(notification: &Notification) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        show_linux(notification).await
    }

    #[cfg(target_os = "macos")]
    {
        show_macos(notification).await
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Err(NotificationError::Failed("Platform not supported".to_string()))
    }
}

#[cfg(target_os = "linux")]
async fn show_linux(notification: &Notification) -> Result<()> {
    use notify_rust::Notification as NotifyNotification;

    let urgency = match notification.priority {
        crate::Priority::Low => notify_rust::Urgency::Low,
        crate::Priority::Normal => notify_rust::Urgency::Normal,
        crate::Priority::High | crate::Priority::Critical => notify_rust::Urgency::Critical,
    };

    let mut notif = NotifyNotification::new();
    notif
        .summary(&notification.title)
        .body(&notification.body)
        .appname(&notification.app_name)
        .urgency(urgency)
        .timeout(notify_rust::Timeout::Milliseconds(5000));

    // Add actions
    for action in &notification.actions {
        notif.action(&action.id, &action.label);
    }

    notif
        .show()
        .map_err(|e| NotificationError::Failed(e.to_string()))?;

    Ok(())
}

#[cfg(target_os = "macos")]
async fn show_macos(notification: &Notification) -> Result<()> {
    use mac_notification_sys::send_notification;

    send_notification(&notification.title, None, &notification.body, None)
        .map_err(|e| NotificationError::Failed(e.to_string()))?;

    Ok(())
}

/// Clear all notifications
pub async fn clear_all() -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        // Linux doesn't have a standard way to clear notifications
        Ok(())
    }

    #[cfg(target_os = "macos")]
    {
        // Use AppleScript to clear notifications
        std::process::Command::new("osascript")
            .args(["-e", "tell application \"System Events\" to tell process \"NotificationCenter\" to click menu item 1 of menu 1 of menu bar item 1 of menu bar 2"])
            .output()
            .ok();
        Ok(())
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(())
    }
}

/// Get notification permission status
pub async fn check_permission() -> Result<bool> {
    #[cfg(target_os = "linux")]
    {
        // Linux typically doesn't require explicit permission
        Ok(true)
    }

    #[cfg(target_os = "macos")]
    {
        // Check macOS notification permission
        // This is simplified; real implementation would use UserNotifications framework
        Ok(true)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(false)
    }
}

/// Request notification permission
pub async fn request_permission() -> Result<bool> {
    #[cfg(target_os = "macos")]
    {
        // Request permission using UserNotifications framework
        // This is a simplified version
        Ok(true)
    }

    #[cfg(target_os = "linux")]
    {
        Ok(true)
    }

    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        Ok(false)
    }
}
