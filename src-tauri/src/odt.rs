use futures_util::StreamExt;
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::path::{Path, PathBuf};
use tokio::process::Command;

const ODT_PAGE_URL: &str = "https://www.microsoft.com/en-us/download/details.aspx?id=49117";

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct InstallConfig {
    pub prefix_path: String,
    pub wine_arch: String,
    pub product_id: String,
    pub channel: String,
    pub language: String,
    pub excluded_apps: Vec<String>,
    pub winetricks_verbs: Vec<String>,
    pub apply_registry_fixes: bool,
    pub custom_odt_path: Option<String>,
}

pub fn build_configuration_xml(cfg: &InstallConfig) -> String {
    let edition = if cfg.wine_arch == "win32" { "32" } else { "64" };
    let excludes: String = cfg
        .excluded_apps
        .iter()
        .map(|app| format!("      <ExcludeApp ID=\"{}\" />\n", app))
        .collect();

    format!(
        "<Configuration>\n  <Add OfficeClientEdition=\"{}\" Channel=\"{}\">\n    <Product ID=\"{}\">\n      <Language ID=\"{}\" />\n{}    </Product>\n  </Add>\n  <Display Level=\"None\" AcceptEULA=\"TRUE\" />\n  <Property Name=\"AUTOACTIVATE\" Value=\"1\" />\n</Configuration>\n",
        edition, cfg.channel, cfg.product_id, cfg.language, excludes
    )
}

#[tauri::command]
pub fn generate_odt_xml(config: InstallConfig) -> String {
    build_configuration_xml(&config)
}

async fn resolve_odt_download_url(client: &reqwest::Client) -> Result<String, String> {
    let html = client
        .get(ODT_PAGE_URL)
        .header("User-Agent", "Mozilla/5.0 (X11; Linux x86_64)")
        .send()
        .await
        .map_err(|e| format!("Cannot reach the ODT download page: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Cannot read the ODT page: {}", e))?;

    for marker in ["https://download.microsoft.com/download/", "https://officecdn.microsoft.com/"] {
        if let Some(start) = html.find(marker) {
            let rest = &html[start..];
            if let Some(end) = rest.find(".exe") {
                return Ok(rest[..end + 4].to_string());
            }
        }
    }

    Err("Could not resolve the ODT download URL. Download the Office Deployment Tool manually and provide its path.".to_string())
}

pub async fn prepare_odt_setup(
    work_dir: &Path,
    custom_odt_path: Option<&str>,
) -> Result<PathBuf, String> {
    fs::create_dir_all(work_dir).map_err(|e| format!("Cannot create the working directory: {}", e))?;

    if let Some(custom) = custom_odt_path.filter(|s| !s.trim().is_empty()) {
        let p = PathBuf::from(custom.trim());
        if !p.exists() {
            return Err(format!("The specified ODT file does not exist: {}", p.display()));
        }
        if p.file_name().and_then(|n| n.to_str()) == Some("setup.exe") {
            return Ok(p);
        }
        return extract_odt(&p, work_dir).await;
    }

    let setup_exe = work_dir.join("setup.exe");
    if setup_exe.exists() {
        return Ok(setup_exe);
    }

    let client = reqwest::Client::new();
    let url = resolve_odt_download_url(&client).await?;
    let installer_path = work_dir.join("officedeploymenttool.exe");

    let resp = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("ODT download failed: {}", e))?;

    if !resp.status().is_success() {
        return Err(format!("ODT-Download HTTP {}", resp.status()));
    }

    let mut file = File::create(&installer_path)
        .map_err(|e| format!("Cannot save the ODT file: {}", e))?;
    let mut stream = resp.bytes_stream();
    while let Some(chunk) = stream.next().await {
        let data = chunk.map_err(|e| format!("Stream error: {}", e))?;
        file.write_all(&data)
            .map_err(|e| format!("Write error: {}", e))?;
    }

    extract_odt(&installer_path, work_dir).await
}

async fn extract_odt(archive_exe: &Path, work_dir: &Path) -> Result<PathBuf, String> {
    let mut cmd = Command::new("cabextract");
    crate::runner::sanitize_child_env_tokio(&mut cmd);
    let status = cmd
        .arg("-d")
        .arg(work_dir)
        .arg(archive_exe)
        .status()
        .await;

    if let Ok(s) = status {
        if s.success() && work_dir.join("setup.exe").exists() {
            return Ok(work_dir.join("setup.exe"));
        }
    }

    let mut cmd_7z = Command::new("7z");
    crate::runner::sanitize_child_env_tokio(&mut cmd_7z);
    let status_7z = cmd_7z
        .arg("x")
        .arg("-y")
        .arg(format!("-o{}", work_dir.display()))
        .arg(archive_exe)
        .status()
        .await;

    if let Ok(s) = status_7z {
        if s.success() && work_dir.join("setup.exe").exists() {
            return Ok(work_dir.join("setup.exe"));
        }
    }

    Err("Could not extract setup.exe from the ODT archive (cabextract / 7z).".to_string())
}
