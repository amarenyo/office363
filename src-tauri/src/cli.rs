use crate::desktop::{create_desktop_entries, scan_installed_apps};
use crate::odt::InstallConfig;
use crate::system_check::check_system;
use crate::wine_installer::{run_installation, run_repair};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

fn print_help() {
    println!(
        "Office363 — Microsoft 365 Linux Installer (CLI and GUI)

Usage:
  office363                         Open the graphical installer (GUI)
  office363 --version                Show the application version
  office363 --check                  Check system dependencies
  office363 --install [OPTIONS]      Install Microsoft 365 from the terminal
  office363 --repair [--prefix PATH] Update shims, fonts, OneNote fixes and registry
  office363 --create-launchers       Create .desktop launchers for installed applications

Options (where applicable):
  --prefix <PATH>          GE-Proton prefix (default: ~/.local/share/office363/proton_prefix)
  --odt <PATH>             Local setup.exe or officedeploymenttool.exe (optional)
  --lang <CODE>            Office language, e.g. en-US or de-DE (default: en-US)
  --product <ID>           O365ProPlusRetail | O365BusinessRetail | O365HomePremRetail
  --channel <CHANNEL>     Current | MonthlyEnterprise | SemiAnnual (default: Current)
  --exclude <APPS>         Comma-separated exclusions (default: Access,Publisher,Teams,OneDrive,Groove)
  -h, --help               Show this help"
    );
}

fn expand_tilde(path: &str) -> String {
    if let Some(rest) = path.strip_prefix("~/") {
        if let Some(home) = dirs::home_dir() {
            return home.join(rest).to_string_lossy().to_string();
        }
    }
    path.to_string()
}

pub fn maybe_run_cli() -> Option<i32> {
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.is_empty() {
        return None;
    }

    if args.iter().any(|a| a == "-h" || a == "--help") {
        print_help();
        return Some(0);
    }

    if args.iter().any(|a| a == "-V" || a == "--version") {
        println!("Office363 {}", env!("CARGO_PKG_VERSION"));
        return Some(0);
    }

    let sys = check_system();
    let mut prefix = sys.default_prefix.clone();
    let mut custom_odt: Option<String> = None;
    let mut lang = "en-US".to_string();
    let mut product = "O365ProPlusRetail".to_string();
    let mut channel = "Current".to_string();
    let mut excluded = vec![
        "Access".to_string(),
        "Publisher".to_string(),
        "Teams".to_string(),
        "OneDrive".to_string(),
        "Groove".to_string(),
    ];

    let mut do_check = false;
    let mut do_install = false;
    let mut do_repair = false;
    let mut do_launchers = false;

    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--check" => do_check = true,
            "--install" => do_install = true,
            "--repair" => do_repair = true,
            "--create-launchers" => do_launchers = true,
            "--prefix" if i + 1 < args.len() => {
                i += 1;
                prefix = expand_tilde(&args[i]);
            }
            "--odt" if i + 1 < args.len() => {
                i += 1;
                custom_odt = Some(expand_tilde(&args[i]));
            }
            "--lang" if i + 1 < args.len() => {
                i += 1;
                lang = args[i].clone();
            }
            "--product" if i + 1 < args.len() => {
                i += 1;
                product = args[i].clone();
            }
            "--channel" if i + 1 < args.len() => {
                i += 1;
                channel = args[i].clone();
            }
            "--exclude" if i + 1 < args.len() => {
                i += 1;
                excluded = args[i]
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
            }
            other => {
                eprintln!("Unknown option: {}", other);
                print_help();
                return Some(2);
            }
        }
        i += 1;
    }

    if !do_check && !do_install && !do_repair && !do_launchers {
        return None;
    }

    if do_check {
        println!("System: {} (ready={})", sys.distro, sys.ready);
        for t in &sys.tools {
            println!(
                "  - {:12} : {} ({})",
                t.name,
                if t.installed { "OK" } else { "MISSING" },
                t.version.as_deref().unwrap_or("-")
            );
        }
        if !sys.ready {
            println!("Note: {}", sys.install_hint);
            return Some(1);
        }
    }

    if do_install {
        let config = InstallConfig {
            prefix_path: prefix.clone(),
            wine_arch: "win64".to_string(),
            product_id: product,
            channel,
            language: lang,
            excluded_apps: excluded,
            winetricks_verbs: Vec::new(),
            apply_registry_fixes: true,
            custom_odt_path: custom_odt,
        };
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Tokio runtime error: {}", e);
                return Some(1);
            }
        };
        let cancelled = Arc::new(AtomicBool::new(false));
        match rt.block_on(run_installation(None, cancelled, config)) {
            Ok(msg) => {
                println!(">>> {}", msg);
                match create_desktop_entries(prefix.clone()) {
                    Ok(n) => println!(">>> {} desktop launchers created.", n),
                    Err(e) => eprintln!(">>> Desktop launcher warning: {}", e),
                }
            }
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                return Some(1);
            }
        }
    } else if do_repair {
        let rt = match tokio::runtime::Runtime::new() {
            Ok(rt) => rt,
            Err(e) => {
                eprintln!("Tokio runtime error: {}", e);
                return Some(1);
            }
        };
        match rt.block_on(run_repair(None, prefix.clone(), product)) {
            Ok(msg) => {
                println!(">>> {}", msg);
                match create_desktop_entries(prefix) {
                    Ok(n) => println!(">>> {} desktop launchers updated.", n),
                    Err(e) => eprintln!(">>> Desktop launcher warning: {}", e),
                }
            }
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                return Some(1);
            }
        }
    } else if do_launchers {
        let apps = scan_installed_apps(prefix.clone());
        println!("Applications found: {}", apps.len());
        for a in &apps {
            println!("  - {} ({})", a.name, a.exe_path);
        }
        match create_desktop_entries(prefix) {
            Ok(n) => println!(">>> {} desktop launchers created.", n),
            Err(e) => {
                eprintln!("[ERROR] {}", e);
                return Some(1);
            }
        }
    }

    Some(0)
}
