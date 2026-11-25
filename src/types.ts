export interface ToolStatus {
  name: string;
  installed: boolean;
  version: string | null;
  required: boolean;
}

export interface SystemCheckResult {
  distro: string;
  install_hint: string;
  default_prefix: string;
  tools: ToolStatus[];
  ready: boolean;
}

export interface InstallConfig {
  prefix_path: string;
  wine_arch: "win64" | "win32";
  product_id: string;
  channel: string;
  language: string;
  excluded_apps: string[];
  winetricks_verbs: string[];
  apply_registry_fixes: boolean;
  custom_odt_path: string | null;
}

export interface DetectedApp {
  id: string;
  name: string;
  exe_path: string;
  desktop_installed: boolean;
}
