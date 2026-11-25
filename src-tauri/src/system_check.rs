use serde::Serialize;
use std::fs;
use std::process::Command;

#[derive(Debug, Serialize)]
pub struct ToolStatus {
    pub name: String,
    pub installed: bool,
    pub version: Option<String>,
    pub required: bool,
}

#[derive(Debug, Serialize)]
pub struct SystemCheckResult {
    pub distro: String,
    pub install_hint: String,
    pub default_prefix: String,
    pub tools: Vec<ToolStatus>,
    pub ready: bool,
}

fn check_tool(cmd: &str, version_arg: &str, required: bool) -> ToolStatus {
    let output = Command::new(cmd).arg(version_arg).output();
    match output {
        Ok(out) if out.status.success() => {
            let stdout = String::from_utf8_lossy(&out.stdout);
            let stderr = String::from_utf8_lossy(&out.stderr);
            let combined = if stdout.trim().is_empty() { stderr } else { stdout };
            let first_line = combined.lines().next().unwrap_or("installed").trim().to_string();
            ToolStatus {
                name: cmd.to_string(),
                installed: true,
                version: Some(first_line),
                required,
            }
        }
        _ => ToolStatus {
            name: cmd.to_string(),
            installed: false,
            version: None,
            required,
        },
    }
}

fn detect_distro() -> (String, String) {
    let os_release = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let mut name = "Linux".to_string();
    let mut id = String::new();

    for line in os_release.lines() {
        if let Some(val) = line.strip_prefix("PRETTY_NAME=") {
            name = val.trim_matches('"').to_string();
        } else if let Some(val) = line.strip_prefix("ID=") {
            id = val.trim_matches('"').to_lowercase();
        }
    }

    let hint = match id.as_str() {
        "fedora" | "rhel" | "centos" => {
            "sudo dnf install python3 cabextract tar curl winetricks".to_string()
        }
        "ubuntu" | "debian" | "linuxmint" | "pop" => {
            "sudo apt update && sudo apt install python3 cabextract tar curl winetricks".to_string()
        }
        "arch" | "manjaro" | "endeavouros" => {
            "sudo pacman -S python cabextract tar curl winetricks".to_string()
        }
        _ => "Install: python3, cabextract, tar, curl, winetricks".to_string(),
    };

    (name, hint)
}

#[tauri::command]
pub fn check_system() -> SystemCheckResult {
    let (distro, install_hint) = detect_distro();
    let default_prefix = dirs::data_local_dir()
        .unwrap_or_else(|| dirs::home_dir().unwrap_or_default().join(".local/share"))
        .join("office363/proton_prefix")
        .to_string_lossy()
        .to_string();

    let tools = vec![
        check_tool("python3", "--version", true),
        check_tool("cabextract", "--version", true),
        check_tool("tar", "--version", true),
        check_tool("curl", "--version", false),
        check_tool("winetricks", "--version", false),
    ];

    let ready = tools.iter().filter(|t| t.required).all(|t| t.installed);

    SystemCheckResult {
        distro,
        install_hint,
        default_prefix,
        tools,
        ready,
    }
}
