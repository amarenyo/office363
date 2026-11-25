import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import "./styles.css";
import { version } from "../package.json";
import type { DetectedApp, InstallConfig, SystemCheckResult } from "./types";

const ALL_APPS = [
  { id: "Word", label: "Word", excludedByDefault: false },
  { id: "Excel", label: "Excel", excludedByDefault: false },
  { id: "PowerPoint", label: "PowerPoint", excludedByDefault: false },
  { id: "Outlook", label: "Outlook", excludedByDefault: false },
  { id: "OneNote", label: "OneNote", excludedByDefault: false },
  { id: "Access", label: "Access", excludedByDefault: true },
  { id: "Publisher", label: "Publisher", excludedByDefault: true },
  { id: "Teams", label: "Teams", excludedByDefault: true },
  { id: "OneDrive", label: "OneDrive", excludedByDefault: true },
  { id: "Groove", label: "OneDrive for Business", excludedByDefault: true },
];

const WINETRICKS_DEFAULT: string[] = ["msxml6"];

let currentStep = 0;
let systemInfo: SystemCheckResult | null = null;
let detectedApps: DetectedApp[] = [];
let installing = false;
let progressPercent = 0;
let progressText = "Ready";
let logs: string[] = [];

const config: InstallConfig = {
  prefix_path: "~/.local/share/office363/proton_prefix",
  wine_arch: "win64",
  product_id: "O365ProPlusRetail",
  channel: "Current",
  language: "en-US",
  excluded_apps: ALL_APPS.filter((a) => a.excludedByDefault).map((a) => a.id),
  winetricks_verbs: [...WINETRICKS_DEFAULT],
  apply_registry_fixes: true,
  custom_odt_path: null,
};

const appEl = document.querySelector<HTMLDivElement>("#app")!;

async function runSystemCheck() {
  systemInfo = await invoke<SystemCheckResult>("check_system");
  if (systemInfo.default_prefix) {
    config.prefix_path = systemInfo.default_prefix;
  }
  render();
}

async function refreshXmlPreview() {
  const xml = await invoke<string>("generate_odt_xml", { config });
  const pre = document.querySelector<HTMLElement>("#xml-preview");
  if (pre) pre.textContent = xml;
}

async function startInstall() {
  if (installing) return;
  installing = true;
  logs = [];
  progressPercent = 2;
  progressText = "Starting installation...";
  render();

  try {
    await invoke<string>("start_installation", { config });
    installing = false;
    await scanApps();
  } catch (err) {
    installing = false;
    logs.push(`[ERROR] ${String(err)}`);
    progressText = "Failed";
  }
  render();
}

async function startRepair() {
  if (installing) return;
  currentStep = 2;
  installing = true;
  logs = [];
  progressPercent = 10;
  progressText = "Updating shims, fonts and registry...";
  render();

  try {
    await invoke<string>("repair_installation", {
      prefixPath: config.prefix_path,
      productId: config.product_id,
    });
    installing = false;
    await scanApps();
  } catch (err) {
    installing = false;
    logs.push(`[ERROR] ${String(err)}`);
    progressText = "Failed";
  }
  render();
}

async function cancelInstall() {
  await invoke("cancel_installation");
  logs.push(">>> Cancellation requested...");
  render();
}

async function scanApps() {
  detectedApps = await invoke<DetectedApp[]>("scan_installed_apps", {
    prefixPath: config.prefix_path,
  });
  render();
}

async function createLaunchers() {
  const count = await invoke<number>("create_desktop_entries", {
    prefixPath: config.prefix_path,
  });
  logs.push(`>>> ${count} desktop launchers created.`);
  await scanApps();
}

async function launchTool(tool: string) {
  await invoke("launch_wine_tool", {
    prefixPath: config.prefix_path,
    tool,
  });
}

function renderStep0(): string {
  if (!systemInfo) {
    return `<div class="card"><p>Checking system environment...</p></div>`;
  }
  const toolsHtml = systemInfo.tools
    .map(
      (t) => `
      <div class="tool-row">
        <div>
          <strong>${t.name}</strong>
          <div style="font-size:0.8rem;color:var(--muted)">
            ${t.version ?? (t.required ? "Missing (required)" : "Missing (optional)")}
          </div>
        </div>
        <span class="${t.installed ? "status-ok" : t.required ? "status-miss" : "status-opt"}">
          ${t.installed ? "OK" : t.required ? "Missing" : "Optional"}
        </span>
      </div>`
    )
    .join("");

  return `
    <div class="card">
      <h2>1. System check (${systemInfo.distro})</h2>
      ${toolsHtml}
      ${
        !systemInfo.ready
          ? `<p style="color:var(--warn);font-size:0.88rem">Install missing packages:</p>
             <div class="code-box">${systemInfo.install_hint}</div>`
          : `<p class="status-ok">All required system dependencies are available.</p>`
      }
      <div class="actions">
        <button class="secondary" id="btn-recheck">Check again</button>
        <button class="primary" id="btn-next-1">Continue to configuration</button>
      </div>
    </div>
  `;
}

function renderStep1(): string {
  const appChecks = ALL_APPS.map((a) => {
    const included = !config.excluded_apps.includes(a.id);
    return `
      <label class="check-pill">
        <input type="checkbox" data-app="${a.id}" ${included ? "checked" : ""} />
        ${a.label}
      </label>`;
  }).join("");

  return `
    <div class="card">
      <h2>2. Microsoft 365 configuration (ODT + GE-Proton)</h2>
      <div class="grid-2">
        <div>
          <label>GE-Proton prefix path</label>
          <input type="text" id="inp-prefix" value="${config.prefix_path}" />
        </div>
        <div>
          <label>Architecture</label>
          <select id="sel-arch" disabled>
            <option value="win64" selected>64-bit (win64)</option>
          </select>
        </div>
        <div>
          <label>Edition</label>
          <select id="sel-product">
            <option value="O365ProPlusRetail" ${config.product_id === "O365ProPlusRetail" ? "selected" : ""}>Microsoft 365 Apps for Enterprise</option>
            <option value="O365BusinessRetail" ${config.product_id === "O365BusinessRetail" ? "selected" : ""}>Microsoft 365 Apps for Business</option>
            <option value="O365HomePremRetail" ${config.product_id === "O365HomePremRetail" ? "selected" : ""}>Microsoft 365 Family / Personal</option>
          </select>
        </div>
        <div>
          <label>Language and channel</label>
          <div class="grid-2">
            <select id="sel-lang">
              <option value="de-DE" ${config.language === "de-DE" ? "selected" : ""}>German (de-DE)</option>
              <option value="en-US" ${config.language === "en-US" ? "selected" : ""}>English (en-US)</option>
            </select>
            <select id="sel-channel">
              <option value="Current" ${config.channel === "Current" ? "selected" : ""}>Current</option>
              <option value="MonthlyEnterprise" ${config.channel === "MonthlyEnterprise" ? "selected" : ""}>MonthlyEnterprise</option>
              <option value="SemiAnnual" ${config.channel === "SemiAnnual" ? "selected" : ""}>SemiAnnual</option>
            </select>
          </div>
        </div>
      </div>

      <div style="margin-top:14px">
        <label>Applications to install</label>
        <div class="checks">${appChecks}</div>
      </div>

      <div style="margin-top:14px" class="grid-2">
        <div>
          <label>Optional: local ODT path (setup.exe / officedeploymenttool.exe)</label>
          <input type="text" id="inp-odt" placeholder="Leave empty to download automatically" value="${config.custom_odt_path ?? ""}" />
        </div>
        <div style="display:flex;align-items:flex-end">
          <label class="check-pill" style="width:100%">
            <input type="checkbox" id="chk-reg" ${config.apply_registry_fixes ? "checked" : ""} />
            Apply Wine registry fixes for Office
          </label>
        </div>
      </div>

      <div style="margin-top:14px">
        <label>Generated configuration.xml</label>
        <pre class="code-box" id="xml-preview"></pre>
      </div>

      <div class="actions">
        <button class="secondary" id="btn-prev-1">Back</button>
        <button class="primary" id="btn-next-2">Continue to installation</button>
      </div>
    </div>
  `;
}

function renderStep2(): string {
  return `
    <div class="card">
      <h2>3. Installation and live log</h2>
      <div style="display:flex;justify-content:space-between;font-size:0.88rem">
        <span>${progressText}</span>
        <span>${progressPercent}%</span>
      </div>
      <div class="progress-bar">
        <div class="progress-fill" style="width:${progressPercent}%"></div>
      </div>
      <div class="log-box" id="log-box">${logs.join("\n") || "No logs yet. Click 'Start installation'."}</div>
      <div class="actions">
        <button class="secondary" id="btn-prev-2" ${installing ? "disabled" : ""}>Back</button>
        <div style="display:flex;gap:8px">
          ${installing ? `<button class="danger" id="btn-cancel">Cancel</button>` : ""}
          <button class="secondary" id="btn-repair" ${installing ? "disabled" : ""}>Repair shims and fixes</button>
          <button class="primary" id="btn-start" ${installing ? "disabled" : ""}>Start installation</button>
          <button class="secondary" id="btn-next-3" ${installing ? "disabled" : ""}>Next</button>
        </div>
      </div>
    </div>
  `;
}

function renderStep3(): string {
  const appsHtml =
    detectedApps.length > 0
      ? detectedApps
          .map(
            (a) => `
          <div class="tool-row">
            <div>
              <strong>${a.name}</strong>
              <div style="font-size:0.78rem;color:var(--muted)">${a.exe_path}</div>
            </div>
            <button class="secondary" data-launch="${a.exe_path}">Launch</button>
          </div>`
          )
          .join("")
      : `<p style="color:var(--muted)">No Office applications found in this prefix.</p>`;

  return `
    <div class="card">
      <h2>4. Desktop integration and management</h2>
      ${appsHtml}
      <div class="actions">
        <div style="display:flex;gap:8px">
          <button class="secondary" id="btn-scan">Scan again</button>
          <button class="secondary" id="btn-repair-4">Repair shims and fixes</button>
          <button class="secondary" id="btn-winecfg">Open winecfg</button>
          <button class="secondary" id="btn-kill">Stop wineserver</button>
        </div>
        <button class="primary" id="btn-desktop" ${detectedApps.length === 0 ? "disabled" : ""}>
          Create .desktop launchers
        </button>
      </div>
    </div>
  `;
}

function render() {
  const stepNames = [
    "1. System check",
    "2. Configuration",
    "3. Installation",
    "4. Desktop integration",
  ];

  const stepsHtml = stepNames
    .map(
      (name, idx) =>
        `<button class="step-btn ${idx === currentStep ? "active" : ""}" data-step="${idx}">${name}</button>`
    )
    .join("");

  let bodyHtml = "";
  if (currentStep === 0) bodyHtml = renderStep0();
  else if (currentStep === 1) bodyHtml = renderStep1();
  else if (currentStep === 2) bodyHtml = renderStep2();
  else bodyHtml = renderStep3();

  appEl.innerHTML = `
    <div class="layout">
      <div class="header">
        <h1>Office363 <small>v${version}</small></h1>
        <span class="badge">Microsoft 365 • GE-Proton + ODT</span>
      </div>
      <div class="steps">${stepsHtml}</div>
      ${bodyHtml}
    </div>
  `;

  bindEvents();
  if (currentStep === 1) refreshXmlPreview();
  const logBox = document.querySelector<HTMLElement>("#log-box");
  if (logBox) logBox.scrollTop = logBox.scrollHeight;
}

function bindEvents() {
  document.querySelectorAll<HTMLButtonElement>(".step-btn").forEach((btn) => {
    btn.onclick = () => {
      if (installing) return;
      currentStep = Number(btn.dataset.step);
      if (currentStep === 3) scanApps();
      render();
    };
  });

  document.querySelector<HTMLButtonElement>("#btn-recheck")?.addEventListener("click", runSystemCheck);
  document.querySelector<HTMLButtonElement>("#btn-next-1")?.addEventListener("click", () => {
    currentStep = 1;
    render();
  });
  document.querySelector<HTMLButtonElement>("#btn-prev-1")?.addEventListener("click", () => {
    currentStep = 0;
    render();
  });
  document.querySelector<HTMLButtonElement>("#btn-next-2")?.addEventListener("click", () => {
    currentStep = 2;
    render();
  });
  document.querySelector<HTMLButtonElement>("#btn-prev-2")?.addEventListener("click", () => {
    currentStep = 1;
    render();
  });
  document.querySelector<HTMLButtonElement>("#btn-next-3")?.addEventListener("click", () => {
    currentStep = 3;
    scanApps();
  });

  document.querySelector<HTMLInputElement>("#inp-prefix")?.addEventListener("input", (e) => {
    config.prefix_path = (e.target as HTMLInputElement).value;
  });
  document.querySelector<HTMLSelectElement>("#sel-arch")?.addEventListener("change", (e) => {
    config.wine_arch = (e.target as HTMLSelectElement).value as "win64" | "win32";
    refreshXmlPreview();
  });
  document.querySelector<HTMLSelectElement>("#sel-product")?.addEventListener("change", (e) => {
    config.product_id = (e.target as HTMLSelectElement).value;
    refreshXmlPreview();
  });
  document.querySelector<HTMLSelectElement>("#sel-lang")?.addEventListener("change", (e) => {
    config.language = (e.target as HTMLSelectElement).value;
    refreshXmlPreview();
  });
  document.querySelector<HTMLSelectElement>("#sel-channel")?.addEventListener("change", (e) => {
    config.channel = (e.target as HTMLSelectElement).value;
    refreshXmlPreview();
  });
  document.querySelector<HTMLInputElement>("#inp-odt")?.addEventListener("input", (e) => {
    const val = (e.target as HTMLInputElement).value.trim();
    config.custom_odt_path = val.length > 0 ? val : null;
  });
  document.querySelector<HTMLInputElement>("#chk-reg")?.addEventListener("change", (e) => {
    config.apply_registry_fixes = (e.target as HTMLInputElement).checked;
  });

  document.querySelectorAll<HTMLInputElement>("[data-app]").forEach((chk) => {
    chk.onchange = () => {
      const id = chk.dataset.app!;
      if (chk.checked) {
        config.excluded_apps = config.excluded_apps.filter((x) => x !== id);
      } else if (!config.excluded_apps.includes(id)) {
        config.excluded_apps.push(id);
      }
      refreshXmlPreview();
    };
  });

  document.querySelector<HTMLButtonElement>("#btn-start")?.addEventListener("click", startInstall);
  document.querySelector<HTMLButtonElement>("#btn-repair")?.addEventListener("click", startRepair);
  document.querySelector<HTMLButtonElement>("#btn-repair-4")?.addEventListener("click", startRepair);
  document.querySelector<HTMLButtonElement>("#btn-cancel")?.addEventListener("click", cancelInstall);
  document.querySelector<HTMLButtonElement>("#btn-scan")?.addEventListener("click", scanApps);
  document.querySelector<HTMLButtonElement>("#btn-desktop")?.addEventListener("click", createLaunchers);
  document.querySelector<HTMLButtonElement>("#btn-winecfg")?.addEventListener("click", () => launchTool("winecfg"));
  document.querySelector<HTMLButtonElement>("#btn-kill")?.addEventListener("click", () => launchTool("kill"));

  document.querySelectorAll<HTMLButtonElement>("[data-launch]").forEach((btn) => {
    btn.onclick = () => launchTool(btn.dataset.launch!);
  });
}

listen<{ line: string }>("install-log", (e) => {
  logs.push(e.payload.line);
  const logBox = document.querySelector<HTMLElement>("#log-box");
  if (logBox) {
    logBox.textContent = logs.join("\n");
    logBox.scrollTop = logBox.scrollHeight;
  }
});

listen<{ step: string; percent: number }>("install-progress", (e) => {
  progressText = e.payload.step;
  progressPercent = e.payload.percent;
  if (currentStep === 2) render();
});

runSystemCheck();
render();
