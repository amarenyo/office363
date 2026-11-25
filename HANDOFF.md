# Office363 maintainer handoff

Last updated: 2026-11-25.

## Scope and workflow

Office363 is an experimental Tauri v2 installer for original 64-bit Microsoft 365 on Linux through umu-launcher and GE-Proton. Maintain the GUI and CLI together. Use English for interface text, logs, documentation and new commit messages. Keep changes in small, reviewable commits. Use `gh` for GitHub operations; local working-tree checks and commits can use Git.

Only x64 Office is a supported target. Some Office packages contain x86 helper dependencies; copying those dependencies does not make 32-bit Office a supported installation mode.

Protect existing prefixes: they may contain signed-in accounts, cookies, unsynchronized notebooks and other user data. Do not delete account state or reset a prefix as a routine diagnostic step. Close Office normally before replacing compatibility components. Replace DLL files atomically or remove a runner symlink before writing; never overwrite the runner through a prefix symlink.

Bound command-line Wine diagnostics with a timeout. Launch interactive GUI applications detached from the terminal and avoid timers that can terminate a user's active session. Do not restart applications while the user is validating a fix unless the restart has been communicated.

Local machine paths, VM addresses, VM login details and account-specific SharePoint URLs are intentionally excluded from this public handoff. Earlier development history is retained locally and is not part of the initial public snapshot.

## Architecture

- Frontend: `src/main.ts`, `src/styles.css`, `src/types.ts`.
- Rust entry and Tauri commands: `src-tauri/src/lib.rs`.
- CLI: `src-tauri/src/cli.rs`.
- Managed runner downloads and child environment cleanup: `src-tauri/src/runner.rs`.
- ODT configuration and extraction: `src-tauri/src/odt.rs`.
- Installation, repair, registry and VFS fixes: `src-tauri/src/wine_installer.rs`.
- Desktop launchers: `src-tauri/src/desktop.rs`.
- Verified OneNote dependency downloads: `src-tauri/src/webservices.rs`.
- Embedded shims: `src-tauri/resources/`; source and Makefiles: `shims/`.

The pinned runner is umu-launcher 1.4.4 with GE-Proton11-12-x86_64. Office is installed using ODT rather than a repackaged Office distribution. The installer currently retrieves the ODT download link from Microsoft's download page, which may change independently of this project.

## Build and validation

```bash
npm ci
npm run build
cargo check --locked --manifest-path src-tauri/Cargo.toml
NO_STRIP=true npm run tauri build
```

Tauri requires Rust/Cargo and Linux GTK 3/WebKitGTK 4.1 development dependencies. The previous test environment used Fedora KDE. `NO_STRIP=true` avoids the AppImage stripping issue encountered with Fedora's ELF relocation format. Build products under `dist/`, `src-tauri/target/` and `dist-packages/` are excluded from Git.

Rebuild `shims/ole32` and `shims/sppc` using their Makefiles. Rebuild `shims/inkobj` and copy its DLL to the matching file in `src-tauri/resources/`. Preserve LGPL notices and attribution when updating the shim sources.

Regression probes in `shims/tests/` are x64 Windows programs. Run them with a compatible Wine runner and the intended shim installed. An already loaded DLL image can remain mapped after an atomic file replacement; use a fresh test process/prefix session when validating a replacement. Do not stop an active user's Wine server to refresh a test image.

## Compatibility decisions to preserve

- `PROTON_VERB=run` prevents application launch and repair registry imports from waiting for unrelated background Office services.
- Word launches with `/q` to avoid its splash-thread COM teardown problem.
- Launchers disable Xalia with `PROTON_USE_XALIA=0` and use `DXVK_CONFIG="dxgi.syncInterval = 0"`.
- Do not enable `PROTON_ENABLE_WAYLAND=1` by default: it previously broke Office Backstage interaction and AltGr input. XWayland was the working path.
- Do not restore the tested WebView2 GPU-disabling browser arguments: they caused account sign-in crashes. Default WebView2 worked for the observed sign-in flow.
- Child commands remove AppImage-specific Python/library environment variables to avoid breaking host Python and external tools.
- Native `msxml6` is needed for the ODT/AppX manifest handling path.
- The ink COM shim supplies the OneNote ink interfaces needed to pass the Desktop Experience startup check.
- The Direct2D fix and Selawik font setup address rendering; virtual GPUs without hardware Vulkan have additional display limitations.

## OneNote fixes and evidence

### SharePoint notebook opening and HTTP transport

Successful authentication did not guarantee successful notebook opening. Wine's WWSAPI failed on `WS_XML_BUFFER_TYPE` (type 21) while reading SharePoint list responses. Native x64 `webservices.dll` allowed the notebook list to be read.

Wine's XmlLite produced a duplicate `xmlns:s` declaration in a SOAP envelope. SharePoint rejected that request with HTTP 500 (`0xE4010646`). Native x64 `xmllite.dll` produced a valid envelope. The downloader verifies pinned SHA-256 hashes before installing either DLL; Microsoft DLLs are downloaded at runtime and are not shipped in the repository.

OneNote-specific DLL overrides are written under `HKCU\Software\Wine\AppDefaults\ONENOTE.EXE\DllOverrides`.

The native WWSAPI import `TpCallbackIndependent` is unavailable in the pinned runner. The shim uses `CallbackMayRunLong` as a scheduling fallback, which is not a complete implementation of Windows' independent-worker semantics.

Office also passed a NULL optional request buffer with a nonzero optional-buffer length to `WinHttpSendRequest`. Wine's TLS sender dereferenced NULL. The shim normalizes only that optional-buffer length to zero, preserving the declared total body length for the later `WinHttpWriteData` call. A local HTTP probe verified that the actual streamed body arrived intact.

### Text processing startup crash

An observed fail-fast (`0x1D7E1754`) originated in `mso20win32client`, with `0x80040154` from missing `Windows.Data.Text.WordsSegmenter` activation.

`shims/ole32/words_segmenter.h` provides the required factory, segmenter, word objects, immutable WinRT collections and iterators. Word boundaries use ICU already included in GE-Proton. The regression probe checks UTF-16 offsets, emoji, combining marks, token lookup, collection interfaces and reference-counted lifetime.

This implementation does not provide Windows-specific linguistic stemming or alternate word forms. Alternate forms are exposed as a valid empty iterable collection.

### LDAP startup crash

A second fail-fast (`0x1E044000`) originated in `mso98win32client` after an unexpected NULL LDAP session handle. Office supplied an empty server name and the Global Catalog port. Wine's empty-host-list handling underflowed an unsigned allocation size and passed an unterminated zero-sized allocation as a URL.

The shim normalizes empty/ASCII-whitespace host lists to NULL, the documented default-server input. Explicit host names, connection failures and security settings remain handled by the real LDAP implementation. Both name and ordinal imports are supported. A regression probe completed 2,000 initialization/unbind calls without opening a network connection.

### Validation limits

Notebook contents have been visible, and targeted regression probes have passed. The user subsequently reported additional crashes, which led to the text-processing and LDAP fixes above. The final observed normal startup remained open for at least three minutes with a visible OneNote window and no unhandled exceptions. A fresh user confirmation covering multiple page switches was still outstanding.

Do not describe this as complete stability, full account compatibility or validated synchronization of every notebook area. A separate `OfficeClickToRun` crash around `Windows.Management.Deployment.StagePackageOptions` has also been observed; distinguish its module name from a OneNote process crash.

## Remaining work

- Validate repeated page switching, editing, synchronization and long-running OneNote sessions.
- Investigate the separate Click-to-Run service crash if it affects application behavior.
- Verify minimize/restore rendering and keyboard layouts across desktop environments.
- Validate Outlook workflows end to end.
- A previous VM repair test completed the new DLL/registry stages but timed out during a separate missing-WebView2 installation. It is not evidence of a full repair end-to-end pass.
- Rebuild packages after source or embedded shim changes; locally cached binaries may predate the current source.

## CI cost and packaging policy

Full package builds are manual or version-tag only. Ordinary code pushes and pull requests run cached source checks; documentation-only changes skip CI. The Linux binary is compiled once and reused in the DEB, RPM, AppImage and Arch binary package. A native Arch source-build recipe remains available for manual use. Package artifacts expire after three days. Use `gh` for dispatching runs and all other GitHub operations.

The Fedora installation check exposed a real OpenSSL ABI mismatch; the downloader now uses rustls with system certificate roots. GUI smoke tests install software-rendering support and use X11 without portals in disposable containers. They still require a visible window owned by the exact process under test.
