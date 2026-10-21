//! Signed GitHub updates. Network and file work runs outside the native UI thread.
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use std::{
    fs::File,
    io::{Read, Write},
    path::PathBuf,
    time::Duration,
};

const ROOT: &str = "https://github.com/NitishMPD/PieAll/releases";
const MAX_INSTALLER: u64 = 256 * 1024 * 1024;
const PUBLIC_KEY: &str = match option_env!("PIEALL_UPDATE_PUBLIC_KEY") {
    Some(key) => key,
    None => "",
};

#[derive(Clone, Deserialize)]
pub struct Release {
    pub version: String,
    #[serde(default)]
    pub notes: String,
    pub url: String,
    pub signature: String,
}

pub struct Download {
    directory: tempfile::TempDir,
    installer: PathBuf,
    release: Release,
}

pub fn configured() -> bool {
    !cfg!(debug_assertions) && PublicKey::from_base64(PUBLIC_KEY.trim()).is_ok()
}

fn agent(timeout: u64) -> ureq::Agent {
    ureq::Agent::config_builder()
        .tls_config(
            ureq::tls::TlsConfig::builder()
                .provider(ureq::tls::TlsProvider::NativeTls)
                .build(),
        )
        .https_only(true)
        .timeout_global(Some(Duration::from_secs(timeout)))
        .timeout_connect(Some(Duration::from_secs(15)))
        .build()
        .into()
}

fn bounded(reader: impl Read, limit: u64) -> Result<Vec<u8>, String> {
    let mut bytes = Vec::new();
    reader
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("Update response exceeds its size limit.".into());
    }
    Ok(bytes)
}

fn get(url: &str, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent(30).get(url).call().map_err(|e| e.to_string())?;
    bounded(response.body_mut().as_reader(), limit)
}

fn verify(bytes: &[u8], signature: &str, key: &str) -> Result<(), String> {
    let key = PublicKey::from_base64(key.trim()).map_err(|e| e.to_string())?;
    let signature = Signature::decode(signature).map_err(|e| e.to_string())?;
    key.verify(bytes, &signature, false)
        .map_err(|e| format!("Update signature: {e}"))
}

fn validate(release: &Release, current: &str) -> Result<bool, String> {
    let version = semver::Version::parse(&release.version).map_err(|e| e.to_string())?;
    let current = semver::Version::parse(current).map_err(|e| e.to_string())?;
    if !version.pre.is_empty() || !version.build.is_empty() {
        return Err("Only stable updates are supported.".into());
    }
    let expected = format!("{ROOT}/download/v{version}/PieAll_{version}_x64-setup.exe");
    if release.url != expected || release.signature.len() > 4096 || release.notes.len() > 128 * 1024
    {
        return Err("Invalid Windows update metadata.".into());
    }
    Signature::decode(&release.signature).map_err(|e| e.to_string())?;
    Ok(version > current)
}

pub fn check() -> Result<Option<Release>, String> {
    if !configured() {
        return Err(
            "Updates require an installed release with a configured signing public key.".into(),
        );
    }
    let bytes = get(&format!("{ROOT}/latest/download/latest.json"), 256 * 1024)?;
    let signature = get(&format!("{ROOT}/latest/download/latest.json.minisig"), 4096)?;
    let signature = std::str::from_utf8(&signature).map_err(|e| e.to_string())?;
    // Authenticate version/URL as well as installer bytes to prevent metadata-driven downgrade.
    verify(&bytes, signature, PUBLIC_KEY)?;
    let release: Release = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
    Ok(validate(&release, env!("CARGO_PKG_VERSION"))?.then_some(release))
}

fn verify_file(file: &mut File, signature: &str) -> Result<(), String> {
    let key = PublicKey::from_base64(PUBLIC_KEY.trim()).map_err(|e| e.to_string())?;
    let signature = Signature::decode(signature).map_err(|e| e.to_string())?;
    let mut verifier = key.verify_stream(&signature).map_err(|e| e.to_string())?;
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        verifier.update(&buffer[..n]);
    }
    verifier
        .finalize()
        .map_err(|e| format!("Update signature: {e}"))
}

pub fn download(release: Release, mut progress: impl FnMut(u8)) -> Result<Download, String> {
    if !configured() || !validate(&release, env!("CARGO_PKG_VERSION"))? {
        return Err("No newer configured update is available.".into());
    }
    let directory = tempfile::Builder::new()
        .prefix("PieAll-update-")
        .tempdir()
        .map_err(|e| e.to_string())?;
    let installer = directory.path().join("setup.exe");
    let mut file = File::create_new(&installer).map_err(|e| e.to_string())?;
    let mut response = agent(600)
        .get(&release.url)
        .call()
        .map_err(|e| e.to_string())?;
    let total = response
        .headers()
        .get("content-length")
        .and_then(|v| v.to_str().ok())
        .and_then(|s| s.parse::<u64>().ok());
    if total.is_some_and(|n| n > MAX_INSTALLER) {
        return Err("Installer exceeds 256 MiB.".into());
    }
    let mut reader = response.body_mut().as_reader();
    let mut count = 0u64;
    let mut last = 255;
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = reader.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        count += n as u64;
        if count > MAX_INSTALLER {
            return Err("Installer exceeds 256 MiB.".into());
        }
        file.write_all(&buffer[..n]).map_err(|e| e.to_string())?;
        let percent = total
            .filter(|n| *n > 0)
            .map_or(0, |n| (count.saturating_mul(100) / n).min(99) as u8);
        if percent != last {
            progress(percent);
            last = percent;
        }
    }
    if count == 0 || total.is_some_and(|n| n != count) {
        return Err("Installer download was incomplete.".into());
    }
    file.sync_all().map_err(|e| e.to_string())?;
    drop(file);
    verify_file(
        &mut File::open(&installer).map_err(|e| e.to_string())?,
        &release.signature,
    )?;
    progress(100);
    Ok(Download {
        directory,
        installer,
        release,
    })
}

impl Download {
    pub fn version(&self) -> &str {
        &self.release.version
    }

    pub fn install(self) -> Result<(), String> {
        // Hold the file without write/delete sharing through process creation (TOCTOU protection).
        use std::os::windows::fs::OpenOptionsExt;
        let mut file = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&self.installer)
            .map_err(|e| e.to_string())?;
        verify_file(&mut file, &self.release.signature)?;
        std::process::Command::new(&self.installer)
            .arg("/UPDATE")
            .spawn()
            .map_err(|e| e.to_string())?;
        // NSIS must still be able to read its original executable after PieAll exits.
        let _ = self.directory.keep();
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    const KEY: &str = "RWQf6LRCGA9i53mlYecO4IzT51TGPpvWucNSCh1CBM0QTaLn73Y7GFO3";
    const SIG: &str = "untrusted comment: signature from minisign secret key\nRUQf6LRCGA9i559r3g7V1qNyJDApGip8MfqcadIgT9CuhV3EMhHoN1mGTkUidF/z7SrlQgXdy8ofjb7bNJJylDOocrCo8KLzZwo=\ntrusted comment: timestamp:1633700835\tfile:test\tprehashed\nwLMDjy9FLAuxZ3q4NlEvkgtyhrr0gtTu6KC4KBJdITbbOeAi1zBIYo0v4iTgt8jJpIidRJnp94ABQkJAgAooBQ==";

    fn release(version: &str) -> Release {
        Release {
            version: version.into(),
            notes: String::new(),
            url: format!("{ROOT}/download/v{version}/PieAll_{version}_x64-setup.exe"),
            signature: SIG.into(),
        }
    }

    #[test]
    fn newer_stable_owned_installer_only() {
        assert!(validate(&release("0.1.10"), "0.1.9").unwrap());
        assert!(!validate(&release("0.1.9"), "0.1.9").unwrap());
        assert!(!validate(&release("0.1.8"), "0.1.9").unwrap());
        for version in ["invalid", "0.1.10-beta", "0.1.10+local"] {
            assert!(validate(&release(version), "0.1.9").is_err());
        }
        let mut r = release("0.1.10");
        r.url = "https://evil.example/setup.exe".into();
        assert!(validate(&r, "0.1.9").is_err());
        r.url = format!("{ROOT}/download/v0.1.8/PieAll_0.1.8_x64-setup.exe");
        assert!(validate(&r, "0.1.9").is_err());
    }

    #[test]
    fn signatures_fail_closed_and_reads_are_bounded() {
        assert!(verify(b"test", SIG, KEY).is_ok());
        assert!(verify(b"tampered", SIG, KEY).is_err());
        assert!(verify(b"test", "invalid", KEY).is_err());
        assert!(verify(b"test", SIG, "").is_err());
        assert_eq!(bounded(&b"1234"[..], 4).unwrap(), b"1234");
        assert!(bounded(&b"12345"[..], 4).is_err());
    }
}
