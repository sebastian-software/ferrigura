//! Explicit host-side catalog fetching. Rendering only reads a verified local lock.
use std::cell::OnceCell;
use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use ferrigura::{FontStyle, FontWeight};
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use url::Url;

const BASE: &str = "https://assets.sebastian-software.com/ferrigura/";
const HASH: &str = "33a0a2fac7f3c0f43e2b38ef79d482e3cc4c75fc3730e241c28a73bef79eeaa8";
const MANIFEST: &str = "https://assets.sebastian-software.com/ferrigura/catalogs/fonts/1.0.0/33a0a2fac7f3c0f43e2b38ef79d482e3cc4c75fc3730e241c28a73bef79eeaa8.json";
pub(crate) const LIMIT: u64 = 16 * 1024 * 1024;
const MANIFEST_LIMIT: u64 = 2 * 1024 * 1024;

pub(crate) fn http_agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn digest(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
pub(crate) fn valid_hash(hash: &str) -> bool {
    hash.len() == 64
        && hash
            .bytes()
            .all(|c| c.is_ascii_digit() || (b'a'..=b'f').contains(&c))
}
fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("catalog needs {key}"))
}
fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("catalog needs {key} array"))
}

fn checked_url(raw: &str) -> Result<Url, String> {
    let url = Url::parse(raw).map_err(|e| format!("invalid catalog URL: {e}"))?;
    if url.scheme() != "https"
        || url.host_str().is_none()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err("catalog URLs must be HTTPS, without credentials, query or fragment".into());
    }
    Ok(url)
}
fn same_origin(a: &Url, b: &Url) -> bool {
    a.scheme() == b.scheme()
        && a.host_str() == b.host_str()
        && a.port_or_known_default() == b.port_or_known_default()
}
fn asset_url(base: &Url, path: &str) -> Result<Url, String> {
    if !path.starts_with("fonts/")
        || path.split('/').any(|p| {
            p.is_empty()
                || p == "."
                || p == ".."
                || !p
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
    {
        return Err(format!("invalid catalog asset path {path:?}"));
    }
    let url = base.join(path).map_err(|e| e.to_string())?;
    if !same_origin(base, &url) || !url.path().starts_with(base.path()) {
        return Err("catalog asset escapes base URL".into());
    }
    Ok(url)
}
pub(crate) fn verified(bytes: &[u8], hash: &str, size: Option<u64>) -> Result<(), String> {
    if !valid_hash(hash) || size.is_some_and(|n| n != bytes.len() as u64) || digest(bytes) != hash {
        return Err(format!("font catalog integrity check failed for {hash}"));
    }
    Ok(())
}
fn bounded_read_if_exists(path: &Path, limit: u64) -> Result<Option<Vec<u8>>, String> {
    #[cfg(unix)]
    let file = {
        use std::os::unix::fs::OpenOptionsExt;
        let mut options = fs::OpenOptions::new();
        options
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK);
        match options.open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) if error.raw_os_error() == Some(libc::ELOOP) => {
                return Err(format!("{} is not a regular file", path.display()));
            }
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        }
    };
    #[cfg(not(unix))]
    let file = {
        match fs::symlink_metadata(path) {
            Ok(metadata) if !metadata.file_type().is_file() => {
                return Err(format!("{} is not a regular file", path.display()));
            }
            Ok(_) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("cannot inspect {}: {error}", path.display())),
        }
        match fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(format!("cannot read {}: {error}", path.display())),
        }
    };
    let metadata = file.metadata().map_err(|e| e.to_string())?;
    if !metadata.is_file() {
        return Err(format!("{} is not a regular file", path.display()));
    }
    if metadata.len() > limit {
        return Err(format!("{} exceeds size limit", path.display()));
    }
    let mut bytes = Vec::new();
    file.take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err(format!("{} exceeds size limit", path.display()));
    }
    Ok(Some(bytes))
}

fn bounded_read(path: &Path, limit: u64) -> Result<Vec<u8>, String> {
    bounded_read_if_exists(path, limit)?
        .ok_or_else(|| format!("cannot read {}: no such file or directory", path.display()))
}

fn write_temporary(path: &Path, bytes: &[u8]) -> Result<PathBuf, String> {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    let dir = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let temp = dir.join(format!(
        ".ferrigura-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    let mut file = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        file.write_all(bytes).map_err(|e| e.to_string())?;
        file.sync_all().map_err(|e| e.to_string())?;
        drop(file);
        Ok(())
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result.map(|()| temp)
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    let temp = write_temporary(path, bytes)?;
    let result = fs::rename(&temp, path).map_err(|e| e.to_string());
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

fn publish_cache(
    path: &Path,
    bytes: &[u8],
    hash: &str,
    size: Option<u64>,
    limit: u64,
) -> Result<Vec<u8>, String> {
    let temp = write_temporary(path, bytes)?;
    let result = match fs::hard_link(&temp, path) {
        Ok(()) => Ok(bytes.to_vec()),
        Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => {
            bounded_read(path, limit).and_then(|winner| {
                verified(&winner, hash, size)?;
                Ok(winner)
            })
        }
        Err(error) => Err(format!("cannot publish {}: {error}", path.display())),
    };
    let cleanup =
        fs::remove_file(&temp).map_err(|e| format!("cannot remove {}: {e}", temp.display()));
    match result {
        Ok(winner) => {
            cleanup?;
            Ok(winner)
        }
        Err(error) => {
            let _ = cleanup;
            Err(error)
        }
    }
}

fn cached<F>(
    cache: &Path,
    hash: &str,
    size: Option<u64>,
    offline: bool,
    limit: u64,
    fetch: F,
) -> Result<(PathBuf, Vec<u8>), String>
where
    F: FnOnce() -> Result<Vec<u8>, String>,
{
    if !valid_hash(hash) {
        return Err("invalid SHA-256".into());
    }
    let path = cache.join("sha256").join(hash);
    if let Some(bytes) = bounded_read_if_exists(&path, limit)? {
        verified(&bytes, hash, size)?;
        return Ok((path, bytes));
    }
    if offline {
        return Err(format!("offline cache misses {hash}"));
    }
    let bytes = fetch()?;
    if bytes.len() as u64 > limit {
        return Err("catalog download exceeds size limit".into());
    }
    verified(&bytes, hash, size)?;
    let winner = publish_cache(&path, &bytes, hash, size, limit)?;
    Ok((path, winner))
}
fn download(agent: &ureq::Agent, url: &Url, limit: u64) -> Result<Vec<u8>, String> {
    let mut response = agent
        .get(url.as_str())
        .call()
        .map_err(|e| format!("fetch {url}: {e}"))?;
    if !response.status().is_success() {
        return Err(format!("fetch {url}: HTTP {}", response.status()));
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > limit {
        return Err("catalog download exceeds size limit".into());
    }
    Ok(bytes)
}

pub fn fetch(args: impl Iterator<Item = String>) -> Result<(), String> {
    let mut args = args;
    let mut manifest = MANIFEST.to_owned();
    let mut hash = HASH.to_owned();
    let mut base = BASE.to_owned();
    let mut cache = PathBuf::from(".ferrigura-font-cache");
    let mut lock = PathBuf::from("ferrigura-fonts.lock.json");
    let mut offline = false;
    let mut protected = false;
    let mut override_base = false;
    let mut families = Vec::new();
    let mut requested_fallbacks = Vec::new();
    let mut aliases = Vec::new();
    let mut override_manifest = false;
    let mut override_hash = false;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                println!(
                    "Usage: ferrigura fonts fetch [OPTIONS] FAMILY...\n\nFetch pinned catalog faces into a local cache and write a font lock. The first family is the default.\n\n  --fallback FAMILY       Add a selected family or alias to the fallback chain; repeat in order\n  --alias ALIAS=FAMILY    Register another name for a selected family; repeat as needed\n  --cache DIR             Cache directory (default: .ferrigura-font-cache)\n  --lock FILE             Output lock (default: ferrigura-fonts.lock.json)\n  --offline               Use only verified cached data\n  --protected             Authorize protected downloads with FERRIGURA_DOWNLOAD_TOKEN\n  --manifest URL          Immutable HTTPS catalog URL; requires --sha256\n  --sha256 HEX            Expected manifest SHA-256; requires --manifest\n  --base URL              HTTPS asset base on the catalog origin, ending in /\n\nRender without network access using --font-lock FILE."
                );
                return Ok(());
            }
            "--manifest" => {
                manifest = crate::value(&mut args, &arg)?;
                override_manifest = true;
            }
            "--sha256" => {
                hash = crate::value(&mut args, &arg)?;
                override_hash = true;
            }
            "--base" => {
                base = crate::value(&mut args, &arg)?;
                override_base = true;
            }
            "--cache" => cache = crate::value(&mut args, &arg)?.into(),
            "--lock" => lock = crate::value(&mut args, &arg)?.into(),
            "--offline" => offline = true,
            "--protected" => protected = true,
            "--fallback" => requested_fallbacks.push(crate::value(&mut args, &arg)?),
            "--alias" => {
                let raw = crate::value(&mut args, &arg)?;
                let (alias, family) = raw.split_once('=').ok_or("--alias needs ALIAS=FAMILY")?;
                if alias.trim().is_empty() || family.trim().is_empty() {
                    return Err("--alias needs ALIAS=FAMILY".into());
                }
                aliases.push((alias.to_owned(), family.to_owned()));
            }
            "--" => {
                families.extend(args);
                break;
            }
            _ if arg.starts_with('-') => return Err(format!("unknown font fetch option {arg}")),
            _ => families.push(arg),
        }
    }
    if override_manifest != override_hash {
        return Err("--manifest and --sha256 must be supplied together".into());
    }
    if protected && (override_manifest || override_base) {
        return Err("--protected cannot be used with a custom catalog or asset base".into());
    }
    if families.is_empty() {
        return Err("provide at least one font family".into());
    }
    if families.iter().any(|name| name.trim().is_empty()) {
        return Err("empty family name".into());
    }
    let mut requested_names = HashSet::new();
    if families
        .iter()
        .any(|name| !requested_names.insert(name.to_ascii_lowercase()))
    {
        return Err("duplicate requested family".into());
    }
    let base = checked_url(&base)?;
    let manifest = checked_url(&manifest)?;
    if !base.path().ends_with('/') || !same_origin(&base, &manifest) {
        return Err("manifest and asset base must share an origin, and base must end in /".into());
    }
    let agent = OnceCell::new();
    let (_, manifest_bytes) = cached(&cache, &hash, None, offline, MANIFEST_LIMIT, || {
        let agent = agent.get_or_init(http_agent);
        if protected {
            crate::protected::fetch(agent, manifest.path(), &hash, None, MANIFEST_LIMIT)
        } else {
            download(agent, &manifest, MANIFEST_LIMIT)
        }
    })?;
    let document: Value = serde_json::from_slice(&manifest_bytes)
        .map_err(|e| format!("invalid catalog JSON: {e}"))?;
    if document.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported font catalog schema".into());
    }
    let catalog = array(&document, "families")?;
    let mut seen = HashSet::new();
    let mut assets = Vec::new();
    let mut names = Vec::new();
    for requested in families {
        let matches: Vec<_> = catalog
            .iter()
            .filter(|f| {
                f.get("family")
                    .and_then(Value::as_str)
                    .is_some_and(|name| name.eq_ignore_ascii_case(&requested))
            })
            .collect();
        if matches.len() != 1 {
            return Err(format!("font family {requested:?} is missing or ambiguous"));
        }
        let family = string(matches[0], "family")?;
        if family.trim().is_empty() || string(matches[0], "version")?.trim().is_empty() {
            return Err("catalog family and version must be nonempty".into());
        }
        if !seen.insert(family.to_ascii_lowercase()) {
            return Err(format!("duplicate font family {family:?}"));
        }
        names.push(family.to_owned());
        let faces = array(matches[0], "faces")?;
        if faces.is_empty() {
            return Err(format!("font family {family:?} has no faces"));
        }
        let mut face_keys = HashSet::new();
        for face in faces {
            let weight = face
                .get("weight")
                .and_then(Value::as_u64)
                .filter(|w| (1..=1000).contains(w))
                .ok_or("invalid face weight")?;
            let style = string(face, "style")?;
            if !matches!(style, "normal" | "italic")
                || !face_keys.insert((weight, style.to_owned()))
            {
                return Err("invalid or duplicate font face".into());
            }
            let asset = face.get("renderer").ok_or("missing renderer face")?;
            let asset_hash = string(asset, "sha256")?;
            let size = asset
                .get("size")
                .and_then(Value::as_u64)
                .filter(|s| *s > 0 && *s <= LIMIT)
                .ok_or("invalid font size")?;
            let path = string(asset, "path")?;
            let filename = path.rsplit('/').next().unwrap_or("");
            if !valid_hash(asset_hash)
                || (filename != format!("{asset_hash}.ttf")
                    && filename != format!("{asset_hash}.otf"))
            {
                return Err("invalid renderer asset identity".into());
            }
            let url = asset_url(&base, path)?;
            assets.push((
                family.to_owned(),
                weight,
                style.to_owned(),
                asset_hash.to_owned(),
                size,
                url,
            ));
        }
    }
    let mut registered_names = names.clone();
    let mut normalized_aliases = Vec::new();
    for (alias, target) in aliases {
        let family = names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(&target))
            .ok_or_else(|| format!("alias target {target:?} was not selected"))?;
        if registered_names
            .iter()
            .any(|name| name.eq_ignore_ascii_case(&alias))
        {
            return Err(format!("duplicate alias {alias:?}"));
        }
        registered_names.push(alias.clone());
        normalized_aliases.push((alias, family.clone()));
    }
    let mut fallback_names = Vec::new();
    for requested in requested_fallbacks {
        let name = registered_names
            .iter()
            .find(|name| name.eq_ignore_ascii_case(&requested))
            .ok_or_else(|| format!("fallback {requested:?} was not selected"))?;
        if fallback_names
            .iter()
            .any(|existing: &String| existing.eq_ignore_ascii_case(name))
        {
            return Err(format!("duplicate fallback {name:?}"));
        }
        fallback_names.push(name.clone());
    }
    let mut selected = Vec::new();
    for (family, weight, style, asset_hash, size, url) in assets {
        let (file, _) = cached(&cache, &asset_hash, Some(size), offline, size, || {
            let agent = agent.get_or_init(http_agent);
            if protected {
                crate::protected::fetch(agent, url.path(), &asset_hash, Some(size), size)
            } else {
                download(agent, &url, size)
            }
        })?;
        let absolute = fs::canonicalize(file).map_err(|e| e.to_string())?;
        selected.push(json!({"family":family,"weight":weight,"style":style,"sha256":asset_hash,"size":size,"path":absolute}));
    }
    for (alias, family) in normalized_aliases {
        let duplicates: Vec<_> = selected
            .iter()
            .filter(|face| face["family"].as_str() == Some(&family))
            .cloned()
            .map(|mut face| {
                face["family"] = json!(alias);
                face
            })
            .collect();
        selected.extend(duplicates);
    }
    let output = json!({"schemaVersion":1,"families":registered_names,"fallbacks":fallback_names,"faces":selected});
    atomic_write(
        &lock,
        &serde_json::to_vec_pretty(&output).map_err(|e| e.to_string())?,
    )
    .map_err(|e| format!("cannot write {}: {e}", lock.display()))?;
    eprintln!("wrote {}", lock.display());
    Ok(())
}

type LockedFace = (String, Arc<Vec<u8>>, FontWeight, FontStyle);

pub fn read_lock(path: &Path) -> Result<(Vec<LockedFace>, Vec<String>), String> {
    let lock: Value =
        serde_json::from_slice(&bounded_read(path, MANIFEST_LIMIT)?).map_err(|e| e.to_string())?;
    if lock.get("schemaVersion").and_then(Value::as_u64) != Some(1) {
        return Err("unsupported font lock schema".into());
    }
    let families: Vec<String> = array(&lock, "families")?
        .iter()
        .map(|f| {
            f.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "invalid lock family".to_owned())
        })
        .collect::<Result<_, _>>()?;
    if families.is_empty() {
        return Err("font lock has no families".into());
    }
    let mut fonts = Vec::new();
    let mut loaded: HashMap<(PathBuf, String), Arc<Vec<u8>>> = HashMap::new();
    for face in array(&lock, "faces")? {
        let family = string(face, "family")?.to_owned();
        if !families.iter().any(|name| name == &family) {
            return Err("font lock contains unlisted family".into());
        }
        let weight = face
            .get("weight")
            .and_then(Value::as_u64)
            .filter(|w| (1..=1000).contains(w))
            .ok_or("invalid lock weight")? as u16;
        let style = match string(face, "style")? {
            "normal" => FontStyle::Normal,
            "italic" => FontStyle::Italic,
            _ => return Err("invalid lock style".into()),
        };
        let path = PathBuf::from(string(face, "path")?);
        let size = face
            .get("size")
            .and_then(Value::as_u64)
            .filter(|n| *n > 0 && *n <= LIMIT)
            .ok_or("invalid lock size")?;
        let sha256 = string(face, "sha256")?;
        let key = (path, sha256.to_owned());
        let bytes = if let Some(existing) = loaded.get(&key) {
            if existing.len() as u64 != size {
                return Err(format!("font catalog integrity check failed for {sha256}"));
            }
            Arc::clone(existing)
        } else {
            let bytes = bounded_read(&key.0, LIMIT)?;
            verified(&bytes, sha256, Some(size))?;
            let shared = Arc::new(bytes);
            loaded.insert(key, Arc::clone(&shared));
            shared
        };
        fonts.push((
            family,
            bytes,
            match weight {
                400 => FontWeight::Normal,
                700 => FontWeight::Bold,
                n => FontWeight::Number(n),
            },
            style,
        ));
    }
    if fonts.is_empty()
        || families
            .iter()
            .any(|family| !fonts.iter().any(|font| &font.0 == family))
    {
        return Err("font lock is missing a family face".into());
    }
    let fallbacks: Vec<String> = array(&lock, "fallbacks")?
        .iter()
        .map(|f| {
            f.as_str()
                .map(str::to_owned)
                .ok_or_else(|| "invalid lock fallback".to_owned())
        })
        .collect::<Result<_, _>>()?;
    if fallbacks.iter().any(|name| !families.contains(name)) {
        return Err("font lock contains unlisted fallback".into());
    }
    Ok((fonts, fallbacks))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn concurrent_cache_writers_share_the_verified_winner_without_clobbering() {
        use std::sync::{Arc, Barrier};
        let dir = std::env::temp_dir().join(format!("ferrigura-cache-race-{}", std::process::id()));
        let bytes = b"immutable font data";
        let hash = digest(bytes);
        let barrier = Arc::new(Barrier::new(8));
        std::thread::scope(|scope| {
            let mut handles = Vec::new();
            for _ in 0..8 {
                let barrier = Arc::clone(&barrier);
                let dir = &dir;
                let hash = &hash;
                handles.push(scope.spawn(move || {
                    cached(dir, hash, Some(bytes.len() as u64), false, LIMIT, || {
                        barrier.wait();
                        Ok(bytes.to_vec())
                    })
                    .unwrap()
                }));
            }
            for handle in handles {
                let (path, result) = handle.join().unwrap();
                assert_eq!(result, bytes);
                assert_eq!(path, dir.join("sha256").join(&hash));
            }
        });
        let path = dir.join("sha256").join(&hash);
        assert_eq!(fs::read(&path).unwrap(), bytes);
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::write(&path, b"corrupt winner").unwrap();
        assert!(publish_cache(&path, bytes, &hash, Some(bytes.len() as u64), LIMIT).is_err());
        assert_eq!(fs::read(&path).unwrap(), b"corrupt winner");
        assert_eq!(fs::read_dir(path.parent().unwrap()).unwrap().count(), 1);
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn offline_selection_lock_alias_fallback_and_corruption() {
        let dir =
            std::env::temp_dir().join(format!("ferrigura-catalog-test-{}", std::process::id()));
        let cache = dir.join("cache");
        fs::create_dir_all(cache.join("sha256")).unwrap();
        let mut families = Vec::new();
        for (name, filename) in [
            ("Fixture", "LexicaUltralegible-Regular.ttf"),
            ("Arrows", "NotoSansMath-Regular-Arrows.ttf"),
        ] {
            let bytes = fs::read(format!(
                "{}/../ferrigura-text/fonts/{filename}",
                env!("CARGO_MANIFEST_DIR")
            ))
            .unwrap();
            let hash = digest(&bytes);
            fs::write(cache.join("sha256").join(&hash), &bytes).unwrap();
            families.push(json!({"family":name,"version":"1.0", "faces":[{"weight":400,"style":"normal","renderer":{"path":format!("fonts/test/{hash}.ttf"),"sha256":hash,"size":bytes.len()}}]}));
        }
        let document = serde_json::to_vec(&json!({"schemaVersion":1,"families":families})).unwrap();
        let hash = digest(&document);
        fs::write(cache.join("sha256").join(&hash), document).unwrap();
        let lock = dir.join("fonts.json");
        let args = vec![
            "--offline".into(),
            "--manifest".into(),
            "https://example.invalid/catalog.json".into(),
            "--sha256".into(),
            hash.clone(),
            "--base".into(),
            "https://example.invalid/".into(),
            "--cache".into(),
            cache.to_string_lossy().into_owned(),
            "--lock".into(),
            lock.to_string_lossy().into_owned(),
            "--alias".into(),
            "monospace=Fixture".into(),
            "--fallback".into(),
            "arrows".into(),
            "fixture".into(),
            "ARROWS".into(),
        ];
        fetch(args.clone().into_iter()).unwrap();
        let (fonts, fallbacks) = read_lock(&lock).unwrap();
        assert_eq!(fonts.len(), 3);
        assert_eq!(fallbacks, ["Arrows"]);
        let fixture = fonts.iter().find(|face| face.0 == "Fixture").unwrap();
        let aliased = fonts.iter().find(|face| face.0 == "monospace").unwrap();
        assert!(
            Arc::ptr_eq(&fixture.1, &aliased.1),
            "aliased lock entries must share one verified buffer"
        );
        // Verify a mismatched size on a cached alias entry is still caught.
        let mut tampered_lock: Value = serde_json::from_slice(&fs::read(&lock).unwrap()).unwrap();
        for face in tampered_lock["faces"].as_array_mut().unwrap() {
            if face["family"].as_str() == Some("monospace") {
                face["size"] = json!(1);
            }
        }
        let tampered_path = dir.join("tampered-alias-size.json");
        fs::write(&tampered_path, serde_json::to_vec(&tampered_lock).unwrap()).unwrap();
        assert!(
            read_lock(&tampered_path)
                .unwrap_err()
                .contains("integrity check failed")
        );
        let mut without_fallback = args.clone();
        let index = without_fallback
            .iter()
            .position(|arg| arg == "--fallback")
            .unwrap();
        without_fallback.drain(index..=index + 1);
        fetch(without_fallback.into_iter()).unwrap();
        assert!(read_lock(&lock).unwrap().1.is_empty());
        fetch(args.into_iter()).unwrap();
        let bad = cache.join("sha256").join(hash);
        fs::write(&bad, b"tampered").unwrap();
        assert!(
            cached(
                &cache,
                bad.file_name().unwrap().to_str().unwrap(),
                None,
                true,
                MANIFEST_LIMIT,
                || unreachable!()
            )
            .is_err()
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn rejects_unsafe_sources_and_corrupt_data() {
        let base = checked_url("https://example.invalid/ferrigura/").unwrap();
        for path in [
            "../escape.ttf",
            "fonts/../escape.ttf",
            "fonts/%2e%2e/file.ttf",
            "https://other.invalid/file.ttf",
        ] {
            assert!(asset_url(&base, path).is_err(), "{path}");
        }
        assert!(checked_url("http://example.invalid/").is_err());
        assert!(verified(b"bad", &"0".repeat(64), Some(3)).is_err());
        assert!(verified(b"data", &digest(b"data"), Some(5)).is_err());
        let dir =
            std::env::temp_dir().join(format!("ferrigura-catalog-small-{}", std::process::id()));
        assert!(
            cached(&dir, &digest(b"data"), Some(1), false, 1, || Ok(
                b"data".to_vec()
            ))
            .is_err()
        );
        assert!(!dir.exists());
        let oversized = std::env::temp_dir().join(format!(
            "ferrigura-catalog-oversized-{}",
            std::process::id()
        ));
        let file = fs::File::create(&oversized).unwrap();
        file.set_len(MANIFEST_LIMIT + 1).unwrap();
        assert!(bounded_read(&oversized, MANIFEST_LIMIT).is_err());
        fs::remove_file(oversized).unwrap();
    }

    #[cfg(unix)]
    #[test]
    fn offline_cache_and_lock_reject_fifos_and_symlinks() {
        use std::os::unix::fs::symlink;
        let dir =
            std::env::temp_dir().join(format!("ferrigura-catalog-fifo-{}", std::process::id()));
        fs::create_dir_all(dir.join("sha256")).unwrap();
        let hash = "0".repeat(64);
        let fifo = dir.join("sha256").join(&hash);
        assert!(
            std::process::Command::new("mkfifo")
                .arg(&fifo)
                .status()
                .unwrap()
                .success()
        );
        let error = cached(&dir, &hash, None, true, MANIFEST_LIMIT, || unreachable!()).unwrap_err();
        assert!(error.contains("not a regular file"));
        assert!(read_lock(&fifo).unwrap_err().contains("not a regular file"));
        let link = dir.join("linked-lock");
        symlink(&fifo, &link).unwrap();
        assert!(read_lock(&link).unwrap_err().contains("not a regular file"));
        let regular = dir.join("regular-file");
        fs::write(&regular, b"{}").unwrap();
        let regular_link = dir.join("linked-regular");
        symlink(&regular, &regular_link).unwrap();
        assert!(
            bounded_read(&regular_link, MANIFEST_LIMIT)
                .unwrap_err()
                .contains("not a regular file")
        );
        assert!(
            bounded_read(&dir, MANIFEST_LIMIT)
                .unwrap_err()
                .contains("not a regular file")
        );
        fs::remove_dir_all(dir).unwrap();
    }

    #[test]
    fn validates_all_faces_before_fetching_any_face_or_replacing_lock() {
        let dir =
            std::env::temp_dir().join(format!("ferrigura-catalog-scope-{}", std::process::id()));
        let cache = dir.join("cache");
        fs::create_dir_all(cache.join("sha256")).unwrap();
        let face_hash = digest(b"not downloaded");
        let document = serde_json::to_vec(&json!({"schemaVersion":1,"families":[{"family":"Fixture","version":"1","faces":[
            {"weight":400,"style":"normal","renderer":{"path":format!("fonts/test/{face_hash}.ttf"),"sha256":face_hash,"size":14}},
            {"weight":700,"style":"normal","renderer":{"path":"fonts/../unsafe.ttf","sha256":"0".repeat(64),"size":1}}
        ]}]})).unwrap();
        let hash = digest(&document);
        fs::write(cache.join("sha256").join(&hash), document).unwrap();
        let lock = dir.join("fonts.json");
        fs::write(&lock, b"keep this lock").unwrap();
        let args = [
            "--offline",
            "--manifest",
            "https://example.invalid/catalog.json",
            "--sha256",
            &hash,
            "--base",
            "https://example.invalid/",
            "--cache",
            cache.to_str().unwrap(),
            "--lock",
            lock.to_str().unwrap(),
            "Fixture",
        ];
        let error = fetch(args.into_iter().map(str::to_owned)).unwrap_err();
        assert!(error.contains("invalid renderer asset"));
        assert_eq!(fs::read(&lock).unwrap(), b"keep this lock");
        assert!(!cache.join("sha256").join(face_hash).exists());
        fs::remove_dir_all(dir).unwrap();
    }
}
