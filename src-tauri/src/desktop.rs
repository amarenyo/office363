use crate::runner::default_runner_paths;
use serde::Serialize;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct DetectedApp {
    pub id: String,
    pub name: String,
    pub exe_path: String,
    pub desktop_installed: bool,
}

const OFFICE_APPS: &[(&str, &str, &str)] = &[
    ("word", "Microsoft Word", "WINWORD.EXE"),
    ("excel", "Microsoft Excel", "EXCEL.EXE"),
    ("powerpoint", "Microsoft PowerPoint", "POWERPNT.EXE"),
    ("outlook", "Microsoft Outlook", "OUTLOOK.EXE"),
    ("onenote", "Microsoft OneNote", "ONENOTE.EXE"),
];

fn find_office_root(prefix: &Path) -> Vec<PathBuf> {
    let candidates = [
        prefix.join("pfx/drive_c/Program Files/Microsoft Office/root/Office16"),
        prefix.join("drive_c/Program Files/Microsoft Office/root/Office16"),
    ];
    candidates.into_iter().filter(|p| p.exists()).collect()
}

fn desktop_dir() -> PathBuf {
    dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("applications")
}

#[tauri::command]
pub fn scan_installed_apps(prefix_path: String) -> Vec<DetectedApp> {
    let prefix = PathBuf::from(&prefix_path);
    let roots = find_office_root(&prefix);
    let apps_dir = desktop_dir();

    OFFICE_APPS
        .iter()
        .filter_map(|(id, name, exe)| {
            for root in &roots {
                let exe_path = root.join(exe);
                if exe_path.exists() {
                    let desktop_file = apps_dir.join(format!("office363-{}.desktop", id));
                    return Some(DetectedApp {
                        id: id.to_string(),
                        name: name.to_string(),
                        exe_path: exe_path.to_string_lossy().to_string(),
                        desktop_installed: desktop_file.exists(),
                    });
                }
            }
            None
        })
        .collect()
}

#[tauri::command]
pub fn create_desktop_entries(prefix_path: String) -> Result<usize, String> {
    let apps = scan_installed_apps(prefix_path.clone());
    let apps_dir = desktop_dir();
    let runner = default_runner_paths();
    fs::create_dir_all(&apps_dir).map_err(|e| format!("Directory error: {}", e))?;

    let mut count = 0;
    for app in apps {
        let extra_arg = if app.id == "word" { " /q" } else { "" };
        let content = format!(
            "[Desktop Entry]\nType=Application\nName={}\nExec=env PROTONPATH=\"{}\" WINEPREFIX=\"{}\" PROTON_VERB=run PROTON_USE_XALIA=0 DXVK_CONFIG=\"dxgi.syncInterval = 0\" \"{}\" \"{}\"{}\nIcon=application-x-ms-dos-executable\nCategories=Office;\nTerminal=false\n",
            app.name,
            runner.proton_dir.display(),
            prefix_path,
            runner.umu_run.display(),
            app.exe_path,
            extra_arg
        );
        let file_path = apps_dir.join(format!("office363-{}.desktop", app.id));
        fs::write(&file_path, content).map_err(|e| format!("Write error: {}", e))?;
        count += 1;
    }

    Ok(count)
}

#[tauri::command]
pub fn launch_wine_tool(prefix_path: String, tool: String) -> Result<(), String> {
    let runner = default_runner_paths();

    if tool == "kill" {
        let pfx = PathBuf::from(&prefix_path).join("pfx");
        let mut cmd = Command::new(runner.proton_dir.join("files/bin/wineserver"));
        crate::runner::sanitize_child_env_std(&mut cmd);
        cmd.arg("-k")
            .env("WINEPREFIX", pfx)
            .spawn()
            .map_err(|e| format!("wineserver error: {}", e))?;
        return Ok(());
    }

    let mut cmd = Command::new(&runner.umu_run);
    crate::runner::sanitize_child_env_std(&mut cmd);
    cmd.env("PROTONPATH", &runner.proton_dir)
        .env("WINEPREFIX", &prefix_path)
        .env("PROTON_VERB", "run")
        .env("PROTON_USE_XALIA", "0")
        .env("DXVK_CONFIG", "dxgi.syncInterval = 0");

    if tool == "winecfg" {
        cmd.arg("winecfg");
    } else {
        cmd.arg(&tool);
        if tool.to_uppercase().ends_with("WINWORD.EXE") {
            cmd.arg("/q");
        }
    }

    cmd.spawn().map_err(|e| format!("Launch error: {}", e))?;
    Ok(())
}
