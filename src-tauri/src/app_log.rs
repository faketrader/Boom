use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use std::process::Command;

use chrono::Utc;
use tauri::{AppHandle, Manager};

use crate::score::AppError;

pub fn log_dir(app: &AppHandle) -> Result<PathBuf, AppError> {
    let mut dir = app
        .path()
        .app_data_dir()
        .map_err(|e| AppError::new(format!("获取日志目录失败: {e}")))?;
    dir.push("logs");
    fs::create_dir_all(&dir).map_err(|e| AppError::new(format!("创建日志目录失败: {e}")))?;
    Ok(dir)
}

pub fn log_path(app: &AppHandle) -> Result<PathBuf, AppError> {
    let mut path = log_dir(app)?;
    path.push("app.log");
    Ok(path)
}

pub fn append_log(
    app: &AppHandle,
    level: &str,
    scope: &str,
    message: &str,
) -> Result<(), AppError> {
    let path = log_path(app)?;
    append_log_to_path(&path, level, scope, message)
}

pub fn append_log_to_path(
    path: &std::path::Path,
    level: &str,
    scope: &str,
    message: &str,
) -> Result<(), AppError> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::new(format!("创建日志目录失败: {e}")))?;
    }
    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .map_err(|e| AppError::new(format!("打开日志文件失败: {e}")))?;
    let now = Utc::now().to_rfc3339();
    writeln!(
        file,
        "[{now}] [{}] [{}] {}",
        level.trim().to_uppercase(),
        scope.trim(),
        message.trim().replace('\r', " ").replace('\n', " | ")
    )
    .map_err(|e| AppError::new(format!("写入日志文件失败: {e}")))?;
    Ok(())
}

pub fn log_error(app: &AppHandle, scope: &str, error: &str) {
    let _ = append_log(app, "error", scope, error);
}

#[tauri::command]
pub fn append_app_log(
    app: AppHandle,
    level: String,
    scope: String,
    message: String,
) -> Result<(), String> {
    append_log(&app, &level, &scope, &message).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_app_log_path(app: AppHandle) -> Result<String, String> {
    log_path(&app)
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn reveal_in_explorer(path: String) -> Result<(), String> {
    let target = path.trim();
    if target.is_empty() {
        return Err("路径不能为空".to_string());
    }
    let path = PathBuf::from(target);
    if !path.exists() {
        return Err("路径不存在".to_string());
    }

    #[cfg(target_os = "windows")]
    {
        let mut command = Command::new("explorer.exe");
        command.arg("/select,").arg(&path);

        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x08000000;
        command.creation_flags(CREATE_NO_WINDOW);

        command
            .spawn()
            .map_err(|e| format!("打开资源管理器失败: {e}"))?;
    }

    #[cfg(target_os = "macos")]
    {
        let mut command = Command::new("open");
        if path.is_file() {
            command.arg("-R").arg(&path);
        } else {
            command.arg(&path);
        }
        command
            .spawn()
            .map_err(|e| format!("打开 Finder 失败: {e}"))?;
    }

    #[cfg(all(unix, not(target_os = "macos")))]
    {
        let mut command = Command::new("xdg-open");
        if path.is_file() {
            if let Some(parent) = path.parent() {
                command.arg(parent);
            } else {
                command.arg(&path);
            }
        } else {
            command.arg(&path);
        }
        command
            .spawn()
            .map_err(|e| format!("打开文件管理器失败: {e}"))?;
    }

    Ok(())
}
