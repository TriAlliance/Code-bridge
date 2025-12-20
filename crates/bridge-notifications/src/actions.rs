//! Notification action execution

use crate::{ActionType, NotificationError, Result};
use std::process::Command;

/// Execute a notification action
pub async fn execute_action(action: &ActionType) -> Result<()> {
    match action {
        ActionType::OpenUrl(url) => open_url(url).await,
        ActionType::OpenFile(path) => open_file(path).await,
        ActionType::RunCommand { command, cwd } => run_command(command, cwd.as_deref()).await,
        ActionType::ApiCall {
            method,
            url,
            body,
            headers,
        } => api_call(method, url, body.as_deref(), headers).await,
        ActionType::Dismiss => Ok(()), // Already handled
        ActionType::Custom(action_name) => {
            tracing::info!("Custom action triggered: {}", action_name);
            Ok(())
        }
    }
}

/// Open a URL in default browser
async fn open_url(url: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(url)
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(url)
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", url])
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    Ok(())
}

/// Open a file in default application
async fn open_file(path: &str) -> Result<()> {
    #[cfg(target_os = "linux")]
    {
        Command::new("xdg-open")
            .arg(path)
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    #[cfg(target_os = "macos")]
    {
        Command::new("open")
            .arg(path)
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    #[cfg(target_os = "windows")]
    {
        Command::new("cmd")
            .args(["/C", "start", "", path])
            .spawn()
            .map_err(|e| NotificationError::Failed(e.to_string()))?;
    }

    Ok(())
}

/// Run a shell command
async fn run_command(command: &str, cwd: Option<&str>) -> Result<()> {
    let shell = if cfg!(target_os = "windows") {
        "cmd"
    } else {
        "sh"
    };

    let shell_arg = if cfg!(target_os = "windows") {
        "/C"
    } else {
        "-c"
    };

    let mut cmd = Command::new(shell);
    cmd.arg(shell_arg).arg(command);

    if let Some(dir) = cwd {
        cmd.current_dir(dir);
    }

    let output = cmd
        .output()
        .map_err(|e| NotificationError::Failed(e.to_string()))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(NotificationError::Failed(format!(
            "Command failed: {}",
            stderr
        )));
    }

    Ok(())
}

/// Make an API call
async fn api_call(
    method: &str,
    url: &str,
    body: Option<&str>,
    headers: &std::collections::HashMap<String, String>,
) -> Result<()> {
    let client = reqwest::Client::new();

    let mut request = match method.to_uppercase().as_str() {
        "GET" => client.get(url),
        "POST" => client.post(url),
        "PUT" => client.put(url),
        "PATCH" => client.patch(url),
        "DELETE" => client.delete(url),
        _ => return Err(NotificationError::Failed(format!("Unknown method: {}", method))),
    };

    for (key, value) in headers {
        request = request.header(key, value);
    }

    if let Some(body) = body {
        request = request.body(body.to_string());
    }

    request.send().await?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_run_command() {
        let result = run_command("echo test", None).await;
        assert!(result.is_ok());
    }
}
