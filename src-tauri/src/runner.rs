use futures_util::StreamExt;
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tauri::{AppHandle, Emitter};
use tokio::process::Command;

const UMU_URL: &str =
    "https://github.com/Open-Wine-Components/umu-launcher/releases/download/1.4.4/umu-launcher-1.4.4-zipapp.tar";
const GE_PROTON_TAG: &str = "GE-Proton11-12-x86_64";
const GE_PROTON_URL: &str =
    "https://github.com/GloriousEggroll/proton-ge-custom/releases/download/GE-Proton11-12/GE-Proton11-12-x86_64.tar.gz";

#[derive(Clone)]
pub struct RunnerPaths {
    pub umu_run: PathBuf,
    pub proton_dir: PathBuf,
    pub proton_ole32: PathBuf,
}

#[derive(Clone, serde::Serialize)]
struct LogEvent {
    line: String,
}

fn emit_log(app: Option<&AppHandle>, msg: impl Into<String>) {
    let line = msg.into();
    if let Some(a) = app {
        let _ = a.emit("install-log", LogEvent { line });
    } else {
        println!("{}", line);
    }
}

const APPIMAGE_POLLUTED_VARS: &[&str] = &[
    "PYTHONHOME",
    "PYTHONPATH",
    "LD_LIBRARY_PATH",
    "LD_PRELOAD",
    "PERLLIB",
    "GSETTINGS_SCHEMA_DIR",
    "QT_PLUGIN_PATH",
    "GTK_PATH",
    "GTK_EXE_PREFIX",
    "GTK_DATA_PREFIX",
];

fn in_appimage() -> bool {
    std::env::var_os("APPIMAGE").is_some() || std::env::var_os("APPDIR").is_some()
}

pub fn sanitize_child_env_tokio(cmd: &mut Command) {
    if in_appimage() {
        for var in APPIMAGE_POLLUTED_VARS {
            cmd.env_remove(var);
        }
    }
}

pub fn sanitize_child_env_std(cmd: &mut std::process::Command) {
    if in_appimage() {
        for var in APPIMAGE_POLLUTED_VARS {
            cmd.env_remove(var);
        }
    }
}

pub fn runner_base_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("office363/runner")
}

pub fn default_runner_paths() -> RunnerPaths {
    let base = runner_base_dir();
    let proton_dir = base.join(GE_PROTON_TAG);
    RunnerPaths {
        umu_run: base.join("umu/umu/umu-run"),
        proton_ole32: proton_dir.join("files/lib/wine/x86_64-windows/ole32.dll"),
        proton_dir,
    }
}

async fn download_file(
    app: Option<&AppHandle>,
    url: &str,
    dest: &Path,
    label: &str,
) -> Result<(), String> {
    let client = reqwest::Client::new();
    let resp = client
        .get(url)
        .header("User-Agent", "Office363/0.1")
        .send()
        .await
        .map_err(|e| format!("Download of {} failed: {}", label, e))?;

    if !resp.status().is_success() {
        return Err(format!("Download {} HTTP {}", label, resp.status()));
    }

    let total = resp.content_length().unwrap_or(0);
    let mut downloaded: u64 = 0;
    let mut last_logged_mb: u64 = 0;
    let mut file = File::create(dest).map_err(|e| format!("File error: {}", e))?;
    let mut stream = resp.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&data).map_err(|e| format!("Write error: {}", e))?;
        downloaded += data.len() as u64;
        let mb = downloaded / (1024 * 1024);
        if mb >= last_logged_mb + 25 {
            last_logged_mb = mb;
            if total > 0 {
                emit_log(
                    app,
                    format!(">>> {} Download: {} / {} MB", label, mb, total / (1024 * 1024)),
                );
            } else {
                emit_log(app, format!(">>> {} Download: {} MB", label, mb));
            }
        }
    }
    Ok(())
}

pub async fn ensure_runner(app: Option<&AppHandle>) -> Result<RunnerPaths, String> {
    let base = runner_base_dir();
    fs::create_dir_all(&base).map_err(|e| format!("Runner directory error: {}", e))?;
    let paths = default_runner_paths();

    if !paths.umu_run.exists() {
        emit_log(app, ">>> Downloading umu-launcher 1.4.4 (zipapp)...");
        let tar_path = base.join("umu-launcher.tar");
        let umu_dir = base.join("umu");
        fs::create_dir_all(&umu_dir).map_err(|e| format!("umu directory error: {}", e))?;
        download_file(app, UMU_URL, &tar_path, "umu-launcher").await?;

        let mut tar_cmd = Command::new("tar");
        sanitize_child_env_tokio(&mut tar_cmd);
        let status = tar_cmd
            .args(["-xf", &tar_path.to_string_lossy(), "-C", &umu_dir.to_string_lossy()])
            .status()
            .await
            .map_err(|e| format!("tar error: {}", e))?;
        let _ = fs::remove_file(&tar_path);
        if !status.success() || !paths.umu_run.exists() {
            return Err("Could not extract umu-launcher.".to_string());
        }
        emit_log(app, ">>> umu-launcher ready.");
    } else {
        emit_log(app, ">>> umu-launcher already available.");
    }

    if !paths.proton_ole32.exists() {
        emit_log(app, format!(">>> Downloading {} (~450 MB)...", GE_PROTON_TAG));
        let tgz_path = base.join(format!("{}.tar.gz", GE_PROTON_TAG));
        download_file(app, GE_PROTON_URL, &tgz_path, GE_PROTON_TAG).await?;

        emit_log(app, format!(">>> Extracting {}...", GE_PROTON_TAG));
        let mut tar_cmd = Command::new("tar");
        sanitize_child_env_tokio(&mut tar_cmd);
        let status = tar_cmd
            .args(["-xzf", &tgz_path.to_string_lossy(), "-C", &base.to_string_lossy()])
            .status()
            .await
            .map_err(|e| format!("tar error: {}", e))?;
        let _ = fs::remove_file(&tgz_path);
        if !status.success() || !paths.proton_ole32.exists() {
            return Err(format!("Could not extract {}.", GE_PROTON_TAG));
        }
        emit_log(app, format!(">>> {} ready.", GE_PROTON_TAG));
    } else {
        emit_log(app, format!(">>> {} already available.", GE_PROTON_TAG));
    }

    Ok(paths)
}
