use crate::odt::{build_configuration_xml, prepare_odt_setup, InstallConfig};
use crate::runner::ensure_runner;
use serde::Serialize;
use std::fs::{self, OpenOptions};
use std::io::Write;
use std::os::unix::fs::symlink;
use std::path::{Path, PathBuf};
use std::process::Stdio;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};
use tokio::io::{AsyncBufReadExt, BufReader};
use tokio::process::Command;

const SPPC_SHIM_BYTES: &[u8] = include_bytes!("../resources/sppc.dll");
const OLE32_SHIM_BYTES: &[u8] = include_bytes!("../resources/ole32.dll");
const INK_SHIM_BYTES: &[u8] = include_bytes!("../resources/ms365ink.dll");

pub struct InstallerState {
    pub cancelled: Arc<AtomicBool>,
    pub active_prefix: std::sync::Mutex<Option<String>>,
}

impl Default for InstallerState {
    fn default() -> Self {
        Self {
            cancelled: Arc::new(AtomicBool::new(false)),
            active_prefix: std::sync::Mutex::new(None),
        }
    }
}

#[derive(Clone, Serialize)]
struct LogEvent {
    line: String,
}

#[derive(Clone, Serialize)]
struct ProgressEvent {
    step: String,
    percent: u8,
}

fn append_install_log(line: &str) {
    let log_path = dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("office363/install.log");
    if let Some(parent) = log_path.parent() {
        let _ = fs::create_dir_all(parent);
    }
    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_path) {
        let _ = writeln!(f, "{}", line);
    }
}

fn emit_log(app: Option<&AppHandle>, msg: impl Into<String>) {
    let line = msg.into();
    append_install_log(&line);
    if let Some(a) = app {
        let _ = a.emit("install-log", LogEvent { line });
    } else {
        println!("{}", line);
    }
}

fn emit_progress(app: Option<&AppHandle>, step: impl Into<String>, percent: u8) {
    let step = step.into();
    append_install_log(&format!("[{:>3}%] {}", percent, step));
    if let Some(a) = app {
        let _ = a.emit("install-progress", ProgressEvent { step, percent });
    } else {
        println!("[{:>3}%] {}", percent, step);
    }
}

fn is_odt_setup_running() -> bool {
    if let Ok(entries) = fs::read_dir("/proc") {
        for entry in entries.flatten() {
            if let Ok(cmd) = fs::read(entry.path().join("cmdline")) {
                let s = String::from_utf8_lossy(&cmd).to_ascii_lowercase();
                if s.contains("setup.exe")
                    && (s.contains("/configure") || s.contains("relaunched"))
                    && !s.contains("umu-run")
                    && !s.contains("waitforexitandrun")
                    && !s.contains("umu.exe")
                {
                    return true;
                }
            }
        }
    }
    false
}

fn dir_size_bytes(path: &Path) -> u64 {
    let mut total = 0;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let p = entry.path();
            if let Ok(meta) = fs::symlink_metadata(&p) {
                if meta.is_dir() {
                    total += dir_size_bytes(&p);
                } else if meta.is_file() {
                    total += meta.len();
                }
            }
        }
    }
    total
}

fn symlink_tree_no_overwrite(src: &Path, dst: &Path) {
    if !src.exists() {
        return;
    }
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let src_path = entry.path();
            let dst_path = dst.join(entry.file_name());
            if let Ok(meta) = fs::symlink_metadata(&src_path) {
                if meta.is_dir() {
                    symlink_tree_no_overwrite(&src_path, &dst_path);
                } else if !dst_path.exists() {
                    let _ = symlink(&src_path, &dst_path);
                }
            }
        }
    }
}

fn copy_dlls_overwrite(src: &Path, dst: &Path) -> usize {
    let mut count = 0;
    if !src.exists() {
        return 0;
    }
    let _ = fs::create_dir_all(dst);
    if let Ok(entries) = fs::read_dir(src) {
        for entry in entries.flatten() {
            let p = entry.path();
            let is_dll = p
                .extension()
                .and_then(|e| e.to_str())
                .map(|e| e.eq_ignore_ascii_case("dll"))
                .unwrap_or(false);
            if is_dll && p.is_file() {
                let target = dst.join(entry.file_name());
                let _ = fs::remove_file(&target);
                if fs::copy(&p, &target).is_ok() {
                    count += 1;
                }
            }
        }
    }
    count
}

fn write_replacing_symlink(dst: &Path, bytes: &[u8]) -> std::io::Result<()> {
    let _ = fs::remove_file(dst);
    fs::write(dst, bytes)
}

fn copy_replacing_symlink(src: &Path, dst: &Path) -> std::io::Result<u64> {
    let _ = fs::remove_file(dst);
    fs::copy(src, dst)
}

fn install_shims(app: Option<&AppHandle>, pfx: &Path, proton_ole32: &Path) -> Result<(), String> {
    let sys32 = pfx.join("drive_c/windows/system32");
    fs::create_dir_all(&sys32).map_err(|e| format!("system32 directory error: {}", e))?;

    write_replacing_symlink(&sys32.join("sppc.dll"), SPPC_SHIM_BYTES)
        .map_err(|e| format!("sppc.dll write error: {}", e))?;
    copy_replacing_symlink(proton_ole32, &sys32.join("ole32_wine.dll"))
        .map_err(|e| format!("ole32_wine.dll copy error: {}", e))?;
    write_replacing_symlink(&sys32.join("ole32.dll"), OLE32_SHIM_BYTES)
        .map_err(|e| format!("ole32.dll write error: {}", e))?;
    write_replacing_symlink(&sys32.join("ms365shim.dll"), OLE32_SHIM_BYTES)
        .map_err(|e| format!("ms365shim.dll write error: {}", e))?;
    write_replacing_symlink(&sys32.join("ms365ink.dll"), INK_SHIM_BYTES)
        .map_err(|e| format!("ms365ink.dll write error: {}", e))?;

    emit_log(app, ">>> 64-bit shims installed (sppc.dll, ole32.dll, ms365shim.dll, ms365ink.dll).");
    Ok(())
}

fn install_d2d1_fix(app: Option<&AppHandle>, pfx: &Path) -> bool {
    let candidates = [
        "/usr/lib64/wine-wow64/wine/x86_64-windows/d2d1.dll",
        "/usr/lib64/wine/x86_64-windows/d2d1.dll",
        "/usr/lib/x86_64-linux-gnu/wine/x86_64-windows/d2d1.dll",
        "/usr/lib/wine/x86_64-windows/d2d1.dll",
    ];
    let Some(src) = candidates.iter().map(Path::new).find(|p| p.exists()) else {
        return false;
    };
    let Ok(mut data) = fs::read(src) else {
        return false;
    };
    let sig = b"Wine builtin DLL";
    if let Some(pos) = data.windows(sig.len()).position(|w| w == sig) {
        data[pos..pos + sig.len()].fill(0);
    }
    let dst = pfx.join("drive_c/windows/system32/d2d1.dll");
    if write_replacing_symlink(&dst, &data).is_ok() {
        emit_log(
            app,
            format!(">>> d2d1.dll UI fix installed (from {}).", src.display()),
        );
        true
    } else {
        false
    }
}

async fn install_selawik_fonts(app: Option<&AppHandle>, pfx: &Path, work_dir: &Path) {
    let fonts_dir = pfx.join("drive_c/windows/Fonts");
    if fonts_dir.join("selawk.ttf").exists() {
        return;
    }
    let _ = fs::create_dir_all(&fonts_dir);
    let zip_path = work_dir.join("selawik.zip");
    let extract_dir = work_dir.join("selawik");
    let url = "https://github.com/microsoft/Selawik/releases/download/1.01/Selawik_Release.zip";

    let client = reqwest::Client::new();
    let Ok(resp) = client
        .get(url)
        .header("User-Agent", "Office363/0.1")
        .send()
        .await
    else {
        return;
    };
    if !resp.status().is_success() {
        return;
    }
    let Ok(bytes) = resp.bytes().await else {
        return;
    };
    if fs::write(&zip_path, &bytes).is_err() {
        return;
    }
    let _ = fs::create_dir_all(&extract_dir);
    let mut py_cmd = Command::new("python3");
    crate::runner::sanitize_child_env_tokio(&mut py_cmd);
    let ok = py_cmd
        .args([
            "-m",
            "zipfile",
            "-e",
            &zip_path.to_string_lossy(),
            &extract_dir.to_string_lossy(),
        ])
        .status()
        .await
        .map(|s| s.success())
        .unwrap_or(false);

    if ok {
        let mut copied = 0;
        if let Ok(entries) = fs::read_dir(&extract_dir) {
            for entry in entries.flatten() {
                let p = entry.path();
                if p.extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("ttf"))
                    .unwrap_or(false)
                {
                    if fs::copy(&p, fonts_dir.join(entry.file_name())).is_ok() {
                        copied += 1;
                    }
                }
            }
        }
        if copied > 0 {
            emit_log(
                app,
                format!(">>> Microsoft Selawik fonts ({} TTFs) installed as a Segoe UI replacement.", copied),
            );
        }
    }
    let _ = fs::remove_file(&zip_path);
    let _ = fs::remove_dir_all(&extract_dir);
}

fn apply_post_install_fixes(app: Option<&AppHandle>, pfx: &Path) {
    let drive_c = pfx.join("drive_c");
    let office16 = drive_c.join("Program Files/Microsoft Office/root/Office16");
    let c2r_dir = drive_c.join("Program Files/Common Files/Microsoft Shared/ClickToRun");

    for dll in ["AppvIsvSubsystems64.dll", "C2R64.dll"] {
        let src = c2r_dir.join(dll);
        let dst = office16.join(dll);
        if src.exists() && office16.exists() {
            let _ = copy_replacing_symlink(&src, &dst);
        }
    }
    emit_log(app, ">>> AppvIsvSubsystems64.dll and C2R64.dll copied to Office16.");

    let vfs = drive_c.join("Program Files/Microsoft Office/root/vfs");
    if !vfs.exists() {
        emit_log(app, ">>> Note: Office VFS directory not found; skipping VFS mirroring.");
        return;
    }

    emit_log(app, ">>> Copying native VC++ runtime DLLs to system32 / syswow64...");
    let sys64 = copy_dlls_overwrite(&vfs.join("System"), &drive_c.join("windows/system32"));
    let sys32 = copy_dlls_overwrite(&vfs.join("SystemX86"), &drive_c.join("windows/syswow64"));
    emit_log(app, format!(">>> Copied {} x64 DLLs and {} x86 DLLs.", sys64, sys32));

    emit_log(app, ">>> Mirroring the Office App-V VFS layout with symbolic links...");
    let mappings = [
        ("Common AppData", "ProgramData"),
        ("Fonts", "windows/Fonts"),
        ("ProgramFilesCommonX64", "Program Files/Common Files"),
        ("ProgramFilesCommonX86", "Program Files (x86)/Common Files"),
        ("ProgramFilesX64", "Program Files"),
        ("ProgramFilesX86", "Program Files (x86)"),
        ("System", "windows/system32"),
        ("SystemX86", "windows/syswow64"),
    ];
    for (src_rel, dst_rel) in mappings {
        symlink_tree_no_overwrite(&vfs.join(src_rel), &drive_c.join(dst_rel));
    }

    let vfs_shared_office = vfs.join("ProgramFilesCommonX64/Microsoft Shared/Office16");
    if vfs_shared_office.exists() && office16.exists() {
        if let Ok(entries) = fs::read_dir(&vfs_shared_office) {
            for entry in entries.flatten() {
                let p = entry.path();
                let is_dll = p
                    .extension()
                    .and_then(|e| e.to_str())
                    .map(|e| e.eq_ignore_ascii_case("dll"))
                    .unwrap_or(false);
                if is_dll {
                    let target = office16.join(entry.file_name());
                    if !target.exists() {
                        let _ = symlink(&p, &target);
                    }
                }
            }
        }
    }
    emit_log(app, ">>> VFS mirroring and DLL linking complete.");
}

async fn kill_proton_wineserver(proton_dir: &Path, pfx: &Path) {
    let wineserver = proton_dir.join("files/bin/wineserver");
    let mut cmd = Command::new(&wineserver);
    crate::runner::sanitize_child_env_tokio(&mut cmd);
    let _ = cmd
        .arg("-k")
        .env("WINEPREFIX", pfx)
        .status()
        .await;
}

async fn run_streamed(
    app: Option<&AppHandle>,
    cancelled: &Arc<AtomicBool>,
    program: &str,
    args: &[&str],
    envs: &[(&str, &str)],
    work_dir: Option<&Path>,
) -> Result<(), String> {
    if cancelled.load(Ordering::Relaxed) {
        return Err("Installation canceled.".to_string());
    }

    let mut cmd = Command::new(program);
    crate::runner::sanitize_child_env_tokio(&mut cmd);
    cmd.args(args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    if let Some(dir) = work_dir {
        cmd.current_dir(dir);
    }

    for (k, v) in envs {
        cmd.env(k, v);
    }

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Cannot start {}: {}", program, e))?;

    let stdout = child.stdout.take();
    let stderr = child.stderr.take();

    let app_out = app.cloned();
    let out_task = tokio::spawn(async move {
        if let Some(s) = stdout {
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(app_out.as_ref(), line);
            }
        }
    });

    let app_err = app.cloned();
    let err_task = tokio::spawn(async move {
        if let Some(s) = stderr {
            let mut lines = BufReader::new(s).lines();
            while let Ok(Some(line)) = lines.next_line().await {
                emit_log(app_err.as_ref(), line);
            }
        }
    });

    loop {
        if cancelled.load(Ordering::Relaxed) {
            let _ = child.kill().await;
            return Err("Installation canceled by the user.".to_string());
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let _ = out_task.await;
                let _ = err_task.await;
                if status.success() {
                    return Ok(());
                }
                return Err(format!("{} exited with code: {:?}", program, status.code()));
            }
            Ok(None) => tokio::time::sleep(std::time::Duration::from_millis(200)).await,
            Err(e) => return Err(format!("Process error: {}", e)),
        }
    }
}

#[tauri::command]
pub fn cancel_installation(state: State<'_, InstallerState>) {
    state.cancelled.store(true, Ordering::Relaxed);
    if let Ok(guard) = state.active_prefix.lock() {
        if let Some(prefix) = guard.as_deref() {
            let runner = crate::runner::default_runner_paths();
            let pfx = PathBuf::from(prefix).join("pfx");
            let _ = std::process::Command::new(runner.proton_dir.join("files/bin/wineserver"))
                .arg("-k")
                .env("WINEPREFIX", pfx)
                .spawn();
        }
    }
}

fn detect_windows_klid() -> &'static str {
    let mut layout = String::new();
    if let Some(home) = dirs::home_dir() {
        if let Ok(content) = fs::read_to_string(home.join(".config/kxkbrc")) {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("LayoutList=") {
                    if let Some(first) = val.split(',').next() {
                        layout = first.trim().to_ascii_lowercase();
                        break;
                    }
                }
            }
        }
    }
    if layout.is_empty() {
        if let Ok(content) = fs::read_to_string("/etc/vconsole.conf") {
            for line in content.lines() {
                if let Some(val) = line.strip_prefix("XKBLAYOUT=").or_else(|| line.strip_prefix("KEYMAP=")) {
                    layout = val.trim().trim_matches('"').to_ascii_lowercase();
                    break;
                }
            }
        }
    }
    match layout.as_str() {
        "ch" | "sg" | "de_ch" | "ch(de)" => "00000807",
        "ch(fr)" | "fr_ch" => "0000100c",
        "de" | "de_de" | "de-latin1" | "de-latin1-nodeadkeys" => "00000407",
        "at" | "de_at" => "00000c07",
        "fr" | "fr_fr" => "0000040c",
        "it" | "it_it" => "00000410",
        "es" | "es_es" => "0000040a",
        "gb" | "uk" => "00000809",
        _ => "00000407",
    }
}

fn build_office_fixes_reg(product_id: &str, has_d2d1_fix: bool) -> String {
    let klid = detect_windows_klid();
    let d2d1_override = if has_d2d1_fix {
        "\"d2d1\"=\"native\"\r\n"
    } else {
        ""
    };
    format!(
        "Windows Registry Editor Version 5.00\r\n\r\n\
[HKEY_CURRENT_USER\\Keyboard Layout\\Preload]\r\n\
\"1\"=\"{}\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\Graphics]\r\n\
\"DisableHardwareAcceleration\"=dword:00000000\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\General]\r\n\
\"DisableBootToOfficeStart\"=dword:00000001\r\n\r\n\
[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Windows NT\\CurrentVersion\\Fonts]\r\n\
\"Selawik (TrueType)\"=\"selawk.ttf\"\r\n\
\"Selawik Bold (TrueType)\"=\"selawkb.ttf\"\r\n\
\"Selawik Light (TrueType)\"=\"selawkl.ttf\"\r\n\
\"Selawik Semibold (TrueType)\"=\"selawksb.ttf\"\r\n\
\"Selawik Semilight (TrueType)\"=\"selawksl.ttf\"\r\n\r\n\
[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Windows NT\\CurrentVersion\\FontSubstitutes]\r\n\
\"Segoe UI\"=\"Selawik\"\r\n\
\"Segoe UI Bold\"=\"Selawik Bold\"\r\n\
\"Segoe UI Light\"=\"Selawik Light\"\r\n\
\"Segoe UI Semibold\"=\"Selawik Semibold\"\r\n\
\"Segoe UI Semilight\"=\"Selawik Semilight\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\Identity]\r\n\
\"EnableADAL\"=dword:00000001\r\n\
\"DisableADALatopWAMOverride\"=dword:00000001\r\n\
\"DisableAADWAM\"=dword:00000001\r\n\
\"DisableMSAWAM\"=dword:00000001\r\n\
\"DisableOneAuth\"=dword:00000000\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Policies\\Microsoft\\Office\\16.0\\Common\\Identity]\r\n\
\"EnableADAL\"=dword:00000001\r\n\
\"DisableADALatopWAMOverride\"=dword:00000001\r\n\
\"DisableAADWAM\"=dword:00000001\r\n\
\"DisableMSAWAM\"=dword:00000001\r\n\r\n\
[HKEY_LOCAL_MACHINE\\Software\\Policies\\Microsoft\\Office\\16.0\\Common\\Identity]\r\n\
\"EnableADAL\"=dword:00000001\r\n\
\"DisableADALatopWAMOverride\"=dword:00000001\r\n\
\"DisableAADWAM\"=dword:00000001\r\n\
\"DisableMSAWAM\"=dword:00000001\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\ExperimentConfigs\\ExternalFeatureOverrides\\word]\r\n\
\"Microsoft.Office.Identity.TestGate.DisableBrokerForOneAuth\"=\"true\"\r\n\
\"Microsoft.Office.Identity.FG.IsWebView2ForOneAuthEnabled\"=\"true\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\ExperimentConfigs\\ExternalFeatureOverrides\\excel]\r\n\
\"Microsoft.Office.Identity.TestGate.DisableBrokerForOneAuth\"=\"true\"\r\n\
\"Microsoft.Office.Identity.FG.IsWebView2ForOneAuthEnabled\"=\"true\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\ExperimentConfigs\\ExternalFeatureOverrides\\powerpoint]\r\n\
\"Microsoft.Office.Identity.TestGate.DisableBrokerForOneAuth\"=\"true\"\r\n\
\"Microsoft.Office.Identity.FG.IsWebView2ForOneAuthEnabled\"=\"true\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\ExperimentConfigs\\ExternalFeatureOverrides\\onenote]\r\n\
\"Microsoft.Office.Identity.TestGate.DisableBrokerForOneAuth\"=\"true\"\r\n\
\"Microsoft.Office.Identity.FG.IsWebView2ForOneAuthEnabled\"=\"true\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\ExperimentConfigs\\ExternalFeatureOverrides\\outlook]\r\n\
\"Microsoft.Office.Identity.TestGate.DisableBrokerForOneAuth\"=\"true\"\r\n\
\"Microsoft.Office.Identity.FG.IsWebView2ForOneAuthEnabled\"=\"true\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Microsoft\\Office\\16.0\\Common\\Licensing\\LicensingNext]\r\n\
\"{}\"=dword:00000002\r\n\
\"O365HomePremRetail\"=dword:00000002\r\n\
\"O365ProPlusRetail\"=dword:00000002\r\n\
\"O365BusinessRetail\"=dword:00000002\r\n\r\n\
[HKEY_LOCAL_MACHINE\\Software\\Microsoft\\Office\\ClickToRun\\Updates]\r\n\
\"UpdatesSkuToSkuBlocked\"=\"1\"\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Microsoft\\Windows NT\\CurrentVersion\\Windows]\r\n\
\"AppInit_DLLs\"=\"ms365shim.dll\"\r\n\
\"LoadAppInit_DLLs\"=dword:00000001\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{937C1A34-151D-4610-9CA6-A8CC9BDB5D83}}]\r\n\
@=\"InkDisp Class\"\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{937C1A34-151D-4610-9CA6-A8CC9BDB5D83}}\\InprocServer32]\r\n\
@=\"C:\\\\windows\\\\system32\\\\ms365ink.dll\"\r\n\
\"ThreadingModel\"=\"Both\"\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{43FB1553-AD74-4EE8-88E4-3E6DAAC915DB}}]\r\n\
@=\"InkCollector Class\"\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{43FB1553-AD74-4EE8-88E4-3E6DAAC915DB}}\\InprocServer32]\r\n\
@=\"C:\\\\windows\\\\system32\\\\ms365ink.dll\"\r\n\
\"ThreadingModel\"=\"Both\"\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{65D00646-CDE3-4A88-9163-6769F0F1A97D}}]\r\n\
@=\"InkOverlay Class\"\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{65D00646-CDE3-4A88-9163-6769F0F1A97D}}\\InprocServer32]\r\n\
@=\"C:\\\\windows\\\\system32\\\\ms365ink.dll\"\r\n\
\"ThreadingModel\"=\"Both\"\r\n\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{9C1CC6E4-D7EB-4EEB-9091-15A7C3791601}}]\r\n\
@=\"InkRenderer Class\"\r\n\
[HKEY_LOCAL_MACHINE\\SOFTWARE\\Classes\\CLSID\\{{9C1CC6E4-D7EB-4EEB-9091-15A7C3791601}}\\InprocServer32]\r\n\
@=\"C:\\\\windows\\\\system32\\\\ms365ink.dll\"\r\n\
\"ThreadingModel\"=\"Both\"\r\n\r\n\
[HKEY_CURRENT_USER\\Software\\Wine\\DllOverrides]\r\n\
\"sppc\"=\"native\"\r\n\
\"ole32\"=\"native,builtin\"\r\n\
{}\
\"riched20\"=\"native,builtin\"\r\n\
\"msxml6\"=\"native,builtin\"\r\n\
\"vcruntime140\"=\"native,builtin\"\r\n\
\"vcruntime140_1\"=\"native,builtin\"\r\n\
\"msvcp140\"=\"native,builtin\"\r\n\
\"msvcp140_1\"=\"native,builtin\"\r\n\
\"msvcp140_2\"=\"native,builtin\"\r\n\
\"msvcp140_atomic_wait\"=\"native,builtin\"\r\n\
\"msvcp140_codecvt_ids\"=\"native,builtin\"\r\n\
\"concrt140\"=\"native,builtin\"\r\n\
\r\n\
[HKEY_CURRENT_USER\\Software\\Wine\\AppDefaults\\ONENOTE.EXE\\DllOverrides]\r\n\
\"webservices\"=\"native,builtin\"\r\n\
\"xmllite\"=\"native,builtin\"\r\n",
        klid, product_id, d2d1_override
    )
}

pub async fn run_repair(
    app: Option<AppHandle>,
    prefix_path: String,
    product_id: String,
) -> Result<String, String> {
    let app_ref = app.as_ref();
    let cancelled = Arc::new(AtomicBool::new(false));
    let runner = ensure_runner(app_ref).await?;
    let umu_str = runner.umu_run.to_string_lossy().to_string();
    let proton_str = runner.proton_dir.to_string_lossy().to_string();
    let compat_dir = PathBuf::from(&prefix_path);
    let pfx_dir = compat_dir.join("pfx");
    if !pfx_dir.exists() {
        return Err(format!("Prefix not found: {}", pfx_dir.display()));
    }
    let work_dir = compat_dir.join("odt_work");
    let _ = fs::create_dir_all(&work_dir);
    let compat_str = compat_dir.to_string_lossy().to_string();
    let envs = [
        ("WINEPREFIX", compat_str.as_str()),
        ("PROTONPATH", proton_str.as_str()),
        ("PROTON_USE_XALIA", "0"),
        ("PROTON_VERB", "run"),
    ];

    emit_progress(app_ref, "Updating shims, UI fixes and registry...", 25);
    kill_proton_wineserver(&runner.proton_dir, &pfx_dir).await;
    install_shims(app_ref, &pfx_dir, &runner.proton_ole32)?;
    let has_d2d1_fix = install_d2d1_fix(app_ref, &pfx_dir);
    install_selawik_fonts(app_ref, &pfx_dir, &work_dir).await;
    apply_post_install_fixes(app_ref, &pfx_dir);
    emit_log(app_ref, ">>> Installing verified 64-bit WWSAPI and XmlLite for OneNote/SharePoint...");
    crate::webservices::install(&pfx_dir, &work_dir).await?;

    let reg_content = build_office_fixes_reg(&product_id, has_d2d1_fix);
    let reg_path = work_dir.join("office_fixes.reg");
    fs::write(&reg_path, reg_content).map_err(|e| format!("Registry file error: {}", e))?;
    let reg_str = reg_path.to_string_lossy().to_string();
    run_streamed(app_ref, &cancelled, &umu_str, &["regedit", "/S", &reg_str], &envs, None).await?;

    let webview2_dir = pfx_dir.join("drive_c/Program Files (x86)/Microsoft/EdgeWebView");
    if !webview2_dir.exists() {
        emit_progress(app_ref, "Installing Microsoft Edge WebView2 for account sign-in...", 75);
        emit_log(app_ref, ">>> Installing webview2 through umu-run winetricks (for Microsoft 365 sign-in)...");
        let _ = run_streamed(app_ref, &cancelled, &umu_str, &["winetricks", "-q", "webview2"], &envs, None).await;
    }

    emit_progress(app_ref, "Update complete", 100);
    Ok("Shims, fonts, OneNote fixes, WebView2 and registry updated successfully.".to_string())
}

#[tauri::command]
pub async fn repair_installation(
    app: AppHandle,
    prefix_path: String,
    product_id: String,
) -> Result<String, String> {
    run_repair(Some(app), prefix_path, product_id).await
}

#[tauri::command]
pub async fn start_installation(
    app: AppHandle,
    state: State<'_, InstallerState>,
    config: InstallConfig,
) -> Result<String, String> {
    state.cancelled.store(false, Ordering::Relaxed);
    if let Ok(mut guard) = state.active_prefix.lock() {
        *guard = Some(config.prefix_path.clone());
    }
    let cancelled = state.cancelled.clone();
    run_installation(Some(app), cancelled, config).await
}

pub async fn run_installation(
    app: Option<AppHandle>,
    cancelled: Arc<AtomicBool>,
    config: InstallConfig,
) -> Result<String, String> {
    let app_ref = app.as_ref();

    emit_progress(app_ref, "Preparing umu-launcher and GE-Proton11-12...", 8);
    let runner = ensure_runner(app_ref).await?;
    let umu_str = runner.umu_run.to_string_lossy().to_string();
    let proton_str = runner.proton_dir.to_string_lossy().to_string();

    let compat_dir = PathBuf::from(&config.prefix_path);
    let pfx_dir = compat_dir.join("pfx");
    fs::create_dir_all(&compat_dir).map_err(|e| format!("Prefix directory error: {}", e))?;

    let work_dir = compat_dir.join("odt_work");
    let compat_str = compat_dir.to_string_lossy().to_string();
    let envs = [
        ("WINEPREFIX", compat_str.as_str()),
        ("PROTONPATH", proton_str.as_str()),
        ("PROTON_USE_XALIA", "0"),
    ];

    emit_progress(app_ref, "Preparing the Office Deployment Tool...", 18);
    emit_log(app_ref, ">>> Downloading/extracting the Office Deployment Tool (ODT)...");
    let setup_exe = prepare_odt_setup(&work_dir, config.custom_odt_path.as_deref()).await?;
    emit_log(app_ref, format!(">>> ODT ready: {}", setup_exe.display()));

    let xml_path = work_dir.join("configuration.xml");
    let xml_content = build_configuration_xml(&config);
    fs::write(&xml_path, &xml_content).map_err(|e| format!("XML write error: {}", e))?;
    emit_log(app_ref, format!(">>> configuration.xml written: {}", xml_path.display()));

    emit_progress(app_ref, "Initializing the GE-Proton prefix...", 28);
    emit_log(app_ref, format!(">>> Initializing the GE-Proton prefix at {}", compat_str));
    run_streamed(app_ref, &cancelled, &umu_str, &["cmd", "/c", "echo", "READY"], &envs, None).await?;

    emit_progress(app_ref, "Installing 64-bit Office shims and UI fixes...", 35);
    install_shims(app_ref, &pfx_dir, &runner.proton_ole32)?;
    let has_d2d1_fix = install_d2d1_fix(app_ref, &pfx_dir);
    install_selawik_fonts(app_ref, &pfx_dir, &work_dir).await;

    emit_log(app_ref, ">>> Installing verified 64-bit WWSAPI and XmlLite for OneNote/SharePoint...");
    crate::webservices::install(&pfx_dir, &work_dir).await?;

    let mut verbs = config.winetricks_verbs.clone();
    if !verbs.iter().any(|v| v.eq_ignore_ascii_case("msxml6")) {
        verbs.insert(0, "msxml6".to_string());
    }
    if !verbs.iter().any(|v| v.eq_ignore_ascii_case("webview2")) {
        verbs.push("webview2".to_string());
    }
    emit_progress(app_ref, "Installing Winetricks dependencies (msxml6, webview2)...", 45);
    emit_log(
        app_ref,
        format!(">>> Installing through umu-run winetricks: {}", verbs.join(" ")),
    );
    let mut wt_args = vec!["winetricks", "-q"];
    let verb_refs: Vec<&str> = verbs.iter().map(|s| s.as_str()).collect();
    wt_args.extend(verb_refs);
    let _ = run_streamed(app_ref, &cancelled, &umu_str, &wt_args, &envs, None).await;

    if config.apply_registry_fixes {
        emit_progress(app_ref, "Applying Office registry tweaks and DLL overrides...", 60);
        emit_log(app_ref, ">>> Applying Microsoft 365 identity, LicensingNext and shim registry keys...");
        let reg_content = build_office_fixes_reg(&config.product_id, has_d2d1_fix);
        let reg_path = work_dir.join("office_fixes.reg");
        fs::write(&reg_path, reg_content).map_err(|e| format!("Registry file error: {}", e))?;
        let reg_str = reg_path.to_string_lossy().to_string();
        run_streamed(app_ref, &cancelled, &umu_str, &["regedit", "/S", &reg_str], &envs, None).await?;
    }

    emit_progress(app_ref, "Microsoft 365 setup is running (download and extraction)...", 70);
    emit_log(app_ref, ">>> Starting the Office Deployment Tool through umu-run and GE-Proton11-12...");
    emit_log(app_ref, ">>> Note: The download (~4.3 GB) runs through OfficeClickToRun in the background.");

    let setup_dir = setup_exe.parent().unwrap_or(&work_dir);
    if setup_dir != work_dir {
        let _ = fs::copy(&xml_path, setup_dir.join("configuration.xml"));
    }

    let monitor_stop = Arc::new(AtomicBool::new(false));
    let monitor_stop_bg = monitor_stop.clone();
    let monitor_app = app.clone();
    let office_dir = pfx_dir.join("drive_c/Program Files/Microsoft Office");
    let c2r_dir = pfx_dir.join("drive_c/Program Files/Common Files/Microsoft Shared/ClickToRun");
    let monitor_proton = runner.proton_dir.clone();
    let monitor_pfx = pfx_dir.clone();
    let monitor_task = tokio::spawn(async move {
        let mut last_mb: u64 = 0;
        let mut saw_setup = false;
        let mut gone_ticks: u8 = 0;
        while !monitor_stop_bg.load(Ordering::Relaxed) {
            tokio::time::sleep(std::time::Duration::from_secs(4)).await;
            if monitor_stop_bg.load(Ordering::Relaxed) {
                break;
            }
            let mb = (dir_size_bytes(&office_dir) + dir_size_bytes(&c2r_dir)) / (1024 * 1024);
            if mb != last_mb && mb > 0 {
                last_mb = mb;
                let pct = (70 + ((mb.min(4500) * 22) / 4500)) as u8;
                emit_progress(
                    monitor_app.as_ref(),
                    format!("Installing Microsoft 365 ({} MB)...", mb),
                    pct,
                );
                emit_log(
                    monitor_app.as_ref(),
                    format!(">>> Click-to-Run progress: {} MB installed", mb),
                );
            }
            if is_odt_setup_running() {
                saw_setup = true;
                gone_ticks = 0;
            } else if saw_setup {
                gone_ticks += 1;
                if gone_ticks >= 3 {
                    emit_log(
                        monitor_app.as_ref(),
                        ">>> ODT setup.exe finished; stopping the Click-to-Run background service...",
                    );
                    kill_proton_wineserver(&monitor_proton, &monitor_pfx).await;
                    break;
                }
            }
        }
    });

    let setup_str = setup_exe.to_string_lossy().to_string();
    let setup_res = run_streamed(
        app_ref,
        &cancelled,
        &umu_str,
        &[&setup_str, "/configure", "configuration.xml"],
        &envs,
        Some(setup_dir),
    )
    .await;

    monitor_stop.store(true, Ordering::Relaxed);
    let _ = monitor_task.await;
    setup_res?;

    emit_progress(app_ref, "Post-installation fixes (VFS mirroring, C2R and VC++ DLLs)...", 95);
    kill_proton_wineserver(&runner.proton_dir, &pfx_dir).await;
    apply_post_install_fixes(app_ref, &pfx_dir);
    install_shims(app_ref, &pfx_dir, &runner.proton_ole32)?;

    emit_progress(app_ref, "Installation complete", 100);
    emit_log(app_ref, ">>> Done! Check desktop integration in the next step.");
    Ok("Installation complete".to_string())
}
