//! OneNote's SharePoint SOAP responses contain WS_XML_BUFFER_TYPE fields, which
//! GE-Proton 11's builtin WWSAPI cannot deserialize (get_type_size: type 21).
//! Wine's XmlLite also duplicates SOAP namespace declarations, causing HTTP 500.
//! Fetch the x64 Microsoft implementations at runtime; do not redistribute them.
use sha2::{Digest, Sha256};
use std::{fs, path::Path, time::Duration};

// Symbol-server payloads, without the original files' Authenticode overlays.
const DLLS: &[(&str, &str, &str)] = &[
    (
        "webservices",
        "2C403A49153000",
        "a2804b37c0ca94164cf406eed81a0bdd05df7c95eb9efa4518b47111e4389810",
    ),
    (
        "xmllite",
        "94A0132E36000",
        "72d75e550d5b9ce12afbfc78b22fb7a36f1dfb081be51017c85d59cfe386ca4a",
    ),
];

fn verified(bytes: &[u8], sha256: &str) -> bool {
    format!("{:x}", Sha256::digest(bytes)) == sha256
}

pub async fn install(pfx: &Path, work_dir: &Path) -> Result<(), String> {
    for &(name, version, sha256) in DLLS {
        install_dll(pfx, work_dir, name, version, sha256)
            .await
            .map_err(|e| format!("{name}.dll: {e}"))?;
    }
    Ok(())
}

async fn install_dll(
    pfx: &Path,
    work_dir: &Path,
    name: &str,
    version: &str,
    sha256: &str,
) -> Result<(), String> {
    let target = pfx.join(format!("drive_c/windows/system32/{name}.dll"));
    if fs::read(&target).is_ok_and(|bytes| verified(&bytes, sha256)) {
        return Ok(());
    }
    let cache = work_dir.join(format!("{name}-x64.dll"));
    let bytes = match fs::read(&cache) {
        Ok(bytes) if verified(&bytes, sha256) => bytes,
        _ => {
            let client = reqwest::Client::builder()
                .timeout(Duration::from_secs(60))
                .build()
                .map_err(|e| format!("WWSAPI HTTP client: {e}"))?;
            let url = format!("https://msdl.microsoft.com/download/symbols/{name}.dll/{version}/{name}.dll");
            let bytes = client
                .get(url)
                .send()
                .await
                .map_err(|e| format!("WWSAPI Download: {e}"))?
                .error_for_status()
                .map_err(|e| format!("WWSAPI Download: {e}"))?
                .bytes()
                .await
                .map_err(|e| format!("WWSAPI Download: {e}"))?
                .to_vec();
            if !verified(&bytes, sha256) {
                return Err("WWSAPI: SHA-256 verification failed".into());
            }
            fs::write(&cache, &bytes).map_err(|e| format!("WWSAPI cache: {e}"))?;
            bytes
        }
    };
    // Rename replaces GE-Proton's symlink itself, never its shared runner DLL.
    let temporary = target.with_extension("dll.office363-tmp");
    if fs::symlink_metadata(&temporary).is_ok() {
        fs::remove_file(&temporary).map_err(|e| format!("WWSAPI temporary file: {e}"))?;
    }
    fs::write(&temporary, bytes).map_err(|e| format!("WWSAPI write error: {e}"))?;
    fs::rename(&temporary, &target).map_err(|e| format!("WWSAPI installation: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    struct TestDirectory(std::path::PathBuf);
    impl Drop for TestDirectory {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[tokio::test]
    #[ignore = "Downloads the pinned Microsoft dependencies over HTTPS"]
    async fn download_and_verify_dependencies_without_overwriting_runner() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let temp = TestDirectory(std::env::temp_dir().join(format!(
            "office363-https-test-{}-{nonce}", std::process::id()
        )));
        let pfx = temp.0.join("pfx");
        let system32 = pfx.join("drive_c/windows/system32");
        let work = temp.0.join("work");
        fs::create_dir_all(&system32).unwrap();
        fs::create_dir_all(&work).unwrap();
        let runner_dll = temp.0.join("shared-runner.dll");
        fs::write(&runner_dll, b"shared runner must remain unchanged").unwrap();
        for &(name, _, _) in DLLS {
            symlink(&runner_dll, system32.join(format!("{name}.dll"))).unwrap();
            fs::write(work.join(format!("{name}-x64.dll")), b"invalid cached download").unwrap();
        }

        install(&pfx, &work).await.unwrap();
        for &(name, _, sha256) in DLLS {
            let target = system32.join(format!("{name}.dll"));
            assert!(!target.is_symlink());
            assert!(verified(&fs::read(target).unwrap(), sha256));
        }
        assert_eq!(fs::read(runner_dll).unwrap(), b"shared runner must remain unchanged");
        // A valid installation must also work after the download cache is removed.
        fs::remove_dir_all(&work).unwrap();
        install(&pfx, &work).await.unwrap();
    }
}
