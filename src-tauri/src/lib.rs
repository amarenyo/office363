mod cli;
mod desktop;
mod odt;
mod runner;
mod system_check;
mod webservices;
mod wine_installer;

use wine_installer::InstallerState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    if let Some(code) = cli::maybe_run_cli() {
        std::process::exit(code);
    }

    tauri::Builder::default()
        .manage(InstallerState::default())
        .invoke_handler(tauri::generate_handler![
            system_check::check_system,
            odt::generate_odt_xml,
            wine_installer::start_installation,
            wine_installer::repair_installation,
            wine_installer::cancel_installation,
            desktop::scan_installed_apps,
            desktop::create_desktop_entries,
            desktop::launch_wine_tool,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
