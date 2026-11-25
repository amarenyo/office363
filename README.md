> **Vibecoded:** This project was built with AI-assisted coding.

# Office363

An experimental Linux desktop installer for the original **64-bit Microsoft 365** applications, using **umu-launcher**, **GE-Proton** and Microsoft's **Office Deployment Tool (ODT)**.

Office363 provides a Tauri desktop wizard and a command-line interface for installation, compatibility repairs and desktop launchers. Microsoft Office runs through Wine/Proton without a virtual machine.

## Status

This is a work in progress. Installation and application startup have been tested on Fedora KDE, including Word, Excel, PowerPoint and OneNote. Outlook is included in the installer and launcher detection, but its full workflow has not been validated.

OneNote SharePoint notebook loading has required several compatibility fixes. Notebook contents have loaded successfully, but repeated crashes were subsequently reported. Additional text segmentation and LDAP fixes passed targeted regression tests and a short startup observation. Extended page switching, editing and synchronization still need validation. Do not treat a successful installation as proof that every Office feature works.

Known areas requiring further testing include minimize/restore rendering, keyboard layouts under KDE Wayland/XWayland and school or organization account workflows.

## Features

- Four-step wizard: system checks, configuration, installation logs and desktop integration.
- CLI installation and repair commands.
- Configurable Office edition, update channel, language and excluded applications.
- Automatic downloads of umu-launcher, GE-Proton and ODT.
- Embedded x64 compatibility shims, registry fixes, Selawik fonts and WebView2 setup.
- OneNote/SharePoint compatibility dependencies downloaded from Microsoft and verified with pinned SHA-256 hashes.
- Desktop launchers for installed Word, Excel, PowerPoint, Outlook and OneNote applications.
- Linux package targets: AppImage, RPM and DEB.

The interface and project documentation are in English. New installations default to `en-US`; German Office (`de-DE`) remains selectable.

## Requirements

- An x86-64 Linux desktop with X11 or XWayland support.
- `python3`, `cabextract` and `tar`; `curl` and host `winetricks` are also checked.
- Internet access and sufficient disk space for the runner, Office download and Wine prefix. The Office payload is several gigabytes.
- Your own valid Microsoft 365 license and any account permissions required to access your documents.

A system Wine installation is not required for the runner. The optional Direct2D fix currently uses a compatible system Wine `d2d1.dll` when one is available.

Microsoft Office and Microsoft runtime DLLs are not bundled with this repository. Required downloads remain subject to their respective licenses. Office363 is an independent project and is not affiliated with Microsoft.

## Build and run

Install Node.js/npm, Rust/Cargo and the Linux build dependencies for Tauri v2, including WebKitGTK 4.1 and GTK 3 development libraries.

```bash
git clone https://github.com/amarenyo/office363.git
cd office363
npm ci
npm run tauri dev
```

Build the frontend:

```bash
npm run build
```

Build the application and Linux packages:

```bash
NO_STRIP=true npm run tauri build
```

`NO_STRIP=true` was required for AppImage packaging on the Fedora test environment. Packages are generated under `src-tauri/target/release/bundle/`.

The compiled CLI is `src-tauri/target/release/office363`:

```bash
./src-tauri/target/release/office363 --help
./src-tauri/target/release/office363 --check
./src-tauri/target/release/office363 --install --lang en-US
./src-tauri/target/release/office363 --repair
./src-tauri/target/release/office363 --create-launchers
```

Supported configuration flags include `--prefix`, `--odt`, `--lang`, `--product`, `--channel` and `--exclude`. Run `--help` for the available values. The GUI supports 64-bit installations; 32-bit installations are not a supported target.

Repair updates compatibility components in the selected prefix. Close Office before repairing; repair stops that prefix's Wine server. Back up important notebooks and documents before experimenting with compatibility changes.

## Data locations

Default locations use the current user's XDG local data directory, normally:

| Location | Contents |
| --- | --- |
| `~/.local/share/office363/proton_prefix/` | Proton compatibility directory |
| `~/.local/share/office363/proton_prefix/pfx/` | Wine prefix, Office installation and account state |
| `~/.local/share/office363/runner/` | Managed umu-launcher and GE-Proton downloads |
| `~/.local/share/office363/install.log` | Persistent installation log |
| `~/.local/share/applications/office363-*.desktop` | Application launchers |

Prefixes and diagnostic traces may contain account information or document data. Keep them out of Git and redact logs before sharing them.

## Development

| Path | Purpose |
| --- | --- |
| `src/` | TypeScript installer interface |
| `src-tauri/src/` | Rust installer, CLI, runner and desktop integration |
| `src-tauri/resources/` | Embedded, project-built compatibility DLLs |
| `shims/ole32/` | COM/WinRT, rendering, HTTP and LDAP compatibility |
| `shims/inkobj/` | OneNote ink COM compatibility |
| `shims/sppc/` | Licensing API compatibility shim |
| `shims/tests/` | Targeted x64 Windows regression probes |
| [HANDOFF.md](HANDOFF.md) | Maintainer notes, validation scope and known pitfalls |

Rebuilding the shims requires an x64 MinGW compiler (`x86_64-w64-mingw32-gcc`). The existing Makefiles build the shim DLLs; the ink shim output must be copied into `src-tauri/resources/` after rebuilding it. Each regression probe documents its compilation command and required test environment in its source header.

Useful checks:

```bash
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
git diff --check
```

## License and acknowledgments

The project uses [LGPL-2.1](LICENSE). The `sppc` and `ole32` compatibility work is based in part on [Tombert/office365_flake](https://github.com/Tombert/office365_flake). See the shim sources and [shim license](shims/LICENSE.LGPL-2.1).

Office363 also relies on the work of Wine, GE-Proton, umu-launcher and Winetricks. Their components retain their own licenses. Microsoft 365, ODT, WebView2, fonts and Microsoft runtime downloads retain Microsoft's applicable license terms.
