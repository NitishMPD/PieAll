//! Offline release gate. This CLI never initializes Tauri, a WebView, or an installer.
use base64::{engine::general_purpose::STANDARD, Engine};
use minisign_verify::{PublicKey, Signature};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use std::{collections::BTreeMap, fs, io::Read, path::Path};

type Result<T> = std::result::Result<T, String>;
#[derive(Deserialize)]
struct Manifest {
    version: String,
    #[serde(default)]
    notes: String,
    platforms: BTreeMap<String, Target>,
}
#[derive(Deserialize)]
struct Target {
    url: String,
    signature: String,
}

fn decode(value: &str) -> Result<String> {
    String::from_utf8(STANDARD.decode(value.trim()).map_err(|e| e.to_string())?)
        .map_err(|e| e.to_string())
}
fn signed_version(signature: &Signature, version: &str) -> Result<()> {
    let versions: Vec<_> = signature
        .trusted_comment()
        .split('\t')
        .filter_map(|f| f.strip_prefix("version:"))
        .collect();
    if versions.as_slice() != [version] {
        return Err("Missing, duplicate or mismatched signed release version.".into());
    }
    Ok(())
}
fn verify_file(
    file: &Path,
    encoded_signature: &str,
    encoded_key: &str,
    version: &str,
    limit: u64,
) -> Result<()> {
    let key = PublicKey::decode(&decode(encoded_key)?).map_err(|e| e.to_string())?;
    let signature = Signature::decode(&decode(encoded_signature)?).map_err(|e| e.to_string())?;
    let mut file = fs::File::open(file).map_err(|e| e.to_string())?;
    let length = file.metadata().map_err(|e| e.to_string())?.len();
    if length == 0 || length > limit {
        return Err("Empty or oversized release artifact.".into());
    }
    let mut verifier = key.verify_stream(&signature).map_err(|e| e.to_string())?;
    let mut read = 0u64;
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        read += n as u64;
        if read > limit {
            return Err("Artifact grew beyond its size limit.".into());
        }
        verifier.update(&buffer[..n]);
    }
    if read != length {
        return Err("Artifact length changed during verification.".into());
    }
    verifier.finalize().map_err(|e| e.to_string())?;
    // Do not trust metadata in the comment until both Minisign signatures pass.
    signed_version(&signature, version)
}
fn read_text(path: &Path, limit: u64) -> Result<String> {
    if fs::metadata(path).map_err(|e| e.to_string())?.len() > limit {
        return Err("Release metadata exceeds its size limit.".into());
    }
    fs::read_to_string(path).map_err(|e| e.to_string())
}
fn checksum(path: &Path) -> Result<String> {
    let mut file = fs::File::open(path).map_err(|e| e.to_string())?;
    let mut hash = Sha256::new();
    let mut buffer = [0; 64 * 1024];
    loop {
        let n = file.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        hash.update(&buffer[..n]);
    }
    Ok(format!("{:x}", hash.finalize()))
}
fn verify_directory(directory: &Path, key: &str, current: &str) -> Result<()> {
    let manifest: Manifest =
        serde_json::from_str(&read_text(&directory.join("latest.json"), 256 * 1024)?)
            .map_err(|e| e.to_string())?;
    let version = semver::Version::parse(&manifest.version).map_err(|e| e.to_string())?;
    if manifest.version != current
        || version.to_string() != manifest.version
        || !version.pre.is_empty()
        || !version.build.is_empty()
        || version.major > 255
        || version.minor > 255
        || version.patch > 65535
        || manifest.notes.len() > 128 * 1024
    {
        return Err("Invalid release version/notes.".into());
    }
    let installer = format!("PieAll_{version}_x64-setup.exe");
    let portable = format!("PieAll_{version}_x64-portable.zip");
    let target = manifest
        .platforms
        .get("windows-x86_64")
        .ok_or("Missing Windows x64 updater target")?;
    if manifest.platforms.len() != 1
        || target.url
            != format!(
                "https://github.com/NitishMPD/PieAll/releases/download/v{version}/{installer}"
            )
    {
        return Err("Invalid owned installer URL/target.".into());
    }
    let mut expected = vec![
        installer.clone(),
        format!("{installer}.sig"),
        portable.clone(),
        format!("{portable}.sig"),
        "latest.json".into(),
        "SHA256SUMS.txt".into(),
    ];
    expected.sort();
    let mut actual = Vec::new();
    for entry in fs::read_dir(directory).map_err(|e| e.to_string())? {
        let entry = entry.map_err(|e| e.to_string())?;
        if !entry.file_type().map_err(|e| e.to_string())?.is_file() {
            return Err("Release directory must contain plain files only.".into());
        }
        actual.push(entry.file_name().to_string_lossy().into_owned());
    }
    actual.sort();
    if actual != expected {
        return Err("Missing or unexpected release assets.".into());
    }
    let installer_sig = read_text(&directory.join(format!("{installer}.sig")), 16 * 1024)?;
    if installer_sig.trim() != target.signature {
        return Err("Installer asset/manifest signatures differ.".into());
    }
    verify_file(
        &directory.join(&installer),
        &installer_sig,
        key,
        current,
        256 * 1024 * 1024,
    )?;
    let portable_sig = read_text(&directory.join(format!("{portable}.sig")), 16 * 1024)?;
    verify_file(
        &directory.join(&portable),
        &portable_sig,
        key,
        current,
        512 * 1024 * 1024,
    )?;
    let mut sums = String::new();
    for name in expected.iter().filter(|n| n.as_str() != "SHA256SUMS.txt") {
        sums.push_str(&format!("{}  {name}\n", checksum(&directory.join(name))?));
    }
    if read_text(&directory.join("SHA256SUMS.txt"), 16 * 1024)? != sums {
        return Err("Release checksums do not match.".into());
    }
    Ok(())
}
fn main() {
    let result = (|| {
        let directory = std::env::args_os()
            .nth(1)
            .ok_or("Expected release assets directory")?;
        let key = option_env!("PIEALL_TAURI_PUBLIC_KEY")
            .filter(|s| !s.trim().is_empty())
            .ok_or("Compile release-verify with PIEALL_TAURI_PUBLIC_KEY")?;
        verify_directory(Path::new(&directory), key, env!("CARGO_PKG_VERSION"))
    })();
    match result {
        Ok(()) => println!("Installer, portable ZIP, signed versions and checksums verified."),
        Err(error) => {
            eprintln!("Release verification failed: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[derive(Deserialize)]
    struct Fixture {
        key: String,
        signature: String,
        data: String,
    }
    fn fixture() -> Fixture {
        serde_json::from_str(include_str!(
            "../../scripts/fixtures/release-signature.json"
        ))
        .unwrap()
    }
    fn file() -> std::path::PathBuf {
        let path = std::env::temp_dir().join(format!(
            "pieall-signature-test-{}-{:?}",
            std::process::id(),
            std::thread::current().id()
        ));
        fs::write(&path, fixture().data).unwrap();
        path
    }
    #[test]
    fn accepts_authenticated_version_and_rejects_tampered_bytes_or_version() {
        let f = fixture();
        let path = file();
        verify_file(&path, &f.signature, &f.key, "0.1.42", 1024).unwrap();
        assert!(verify_file(&path, &f.signature, &f.key, "0.1.43", 1024).is_err());
        assert!(verify_file(&path, &f.signature, &f.key, "0.1.42", 1).is_err());
        fs::write(&path, "tampered").unwrap();
        assert!(verify_file(&path, &f.signature, &f.key, "0.1.42", 1024).is_err());
        fs::remove_file(path).unwrap();
    }
    #[test]
    fn malformed_keys_signatures_and_unsigned_versions_fail_closed() {
        let f = fixture();
        let path = file();
        assert!(verify_file(&path, "invalid", &f.key, "0.1.42", 1024).is_err());
        assert!(verify_file(&path, &f.signature, "invalid", "0.1.42", 1024).is_err());
        let decoded = decode(&f.signature).unwrap();
        let changed = STANDARD.encode(decoded.replace("version:0.1.42", "version:0.1.43"));
        assert!(verify_file(&path, &changed, &f.key, "0.1.43", 1024).is_err());
        let unsigned = Signature::decode(&decoded.replace("\tversion:0.1.42", "")).unwrap();
        assert!(signed_version(&unsigned, "0.1.42").is_err());
        fs::remove_file(path).unwrap();
    }
}
