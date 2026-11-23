//! Explicit catalog download authorization. Credentials and signed URLs are never logged.
use std::io::Read;
use std::time::Duration;

use serde_json::{Value, json};
use url::Url;

use crate::font_catalog::verified;

const ISSUER: &str = "https://downloads.ferrigura.dev/v1/downloads";
const HOST: &str = "protected-assets.ferrigura.dev";
const ISSUER_LIMIT: u64 = 64 * 1024;

fn agent() -> ureq::Agent {
    ureq::Agent::config_builder()
        .max_redirects(0)
        .timeout_global(Some(Duration::from_secs(30)))
        .build()
        .into()
}

fn read_response(
    mut response: ureq::http::Response<ureq::Body>,
    limit: u64,
) -> Result<Vec<u8>, String> {
    if !response.status().is_success() {
        return Err("protected download request failed".into());
    }
    let mut bytes = Vec::new();
    response
        .body_mut()
        .as_reader()
        .take(limit + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| "protected download response failed".to_owned())?;
    if bytes.len() as u64 > limit {
        return Err("protected download exceeds size limit".into());
    }
    Ok(bytes)
}

fn signed_url(
    response: &Value,
    path: &str,
    hash: &str,
    size: Option<u64>,
    limit: u64,
) -> Result<(Url, u64), String> {
    let invalid = || "invalid protected download authorization".to_owned();
    let bytes = response
        .get("bytes")
        .and_then(Value::as_u64)
        .filter(|n| *n > 0 && *n <= limit)
        .ok_or_else(invalid)?;
    if size.is_some_and(|expected| expected != bytes)
        || response.get("sha256").and_then(Value::as_str) != Some(hash)
    {
        return Err(invalid());
    }
    let raw = response
        .get("url")
        .and_then(Value::as_str)
        .ok_or_else(invalid)?;
    let url = Url::parse(raw).map_err(|_| invalid())?;
    if url.scheme() != "https"
        || url.host_str() != Some(HOST)
        || url.port().is_some()
        || !url.username().is_empty()
        || url.password().is_some()
        || url.fragment().is_some()
        || url.path() != path
    {
        return Err(invalid());
    }
    let mut token = None;
    let mut expires = None;
    for (key, value) in url.query_pairs() {
        match key.as_ref() {
            "token" if token.is_none() && !value.is_empty() => token = Some(value.into_owned()),
            "expires"
                if expires.is_none()
                    && !value.is_empty()
                    && value.bytes().all(|b| b.is_ascii_digit()) =>
            {
                expires = Some(value.into_owned())
            }
            _ => return Err(invalid()),
        }
    }
    if token.is_none() || expires.is_none() {
        return Err(invalid());
    }
    Ok((url, bytes))
}

fn fetch_with<I, D>(
    path: &str,
    hash: &str,
    size: Option<u64>,
    limit: u64,
    token: &str,
    issue: I,
    download: D,
) -> Result<Vec<u8>, String>
where
    I: FnOnce(&str, &str, &str) -> Result<Value, String>,
    D: FnOnce(&Url, u64) -> Result<Vec<u8>, String>,
{
    if token.is_empty() || token.chars().any(char::is_control) {
        return Err("FERRIGURA_DOWNLOAD_TOKEN is missing or invalid".into());
    }
    if !(path.starts_with("/ferrigura/fonts/") || path.starts_with("/ferrigura/catalogs/"))
        || !crate::font_catalog::valid_hash(hash)
        || path[1..].split('/').any(|part| {
            part.is_empty()
                || part == "."
                || part == ".."
                || !part
                    .bytes()
                    .all(|c| c.is_ascii_alphanumeric() || b"-_.".contains(&c))
        })
        || ![
            format!("{hash}.json"),
            format!("{hash}.ttf"),
            format!("{hash}.otf"),
        ]
        .iter()
        .any(|name| path.rsplit('/').next() == Some(name.as_str()))
    {
        return Err("invalid protected catalog request".into());
    }
    let response = issue(path, hash, token)
        .map_err(|_| "protected download authorization failed".to_owned())?;
    let (url, expected_size) = signed_url(&response, path, hash, size, limit)?;
    let bytes =
        download(&url, expected_size).map_err(|_| "protected download failed".to_owned())?;
    verified(&bytes, hash, Some(expected_size))
        .map_err(|_| "protected download integrity check failed".to_owned())?;
    Ok(bytes)
}

fn issue(issuer: &str, path: &str, hash: &str, token: &str) -> Result<Value, String> {
    let payload = json!({"path":path,"sha256":hash}).to_string();
    let response = agent()
        .post(issuer)
        .header("Authorization", format!("Bearer {token}"))
        .header("Content-Type", "application/json")
        .send(payload.as_str())
        .map_err(|_| "authorization failed".to_owned())?;
    let bytes = read_response(response, ISSUER_LIMIT)?;
    serde_json::from_slice(&bytes).map_err(|_| "invalid authorization response".to_owned())
}

pub fn fetch(path: &str, hash: &str, size: Option<u64>, limit: u64) -> Result<Vec<u8>, String> {
    let token = std::env::var("FERRIGURA_DOWNLOAD_TOKEN").map_err(|_| {
        "FERRIGURA_DOWNLOAD_TOKEN is required for a protected cache miss".to_owned()
    })?;
    fetch_with(
        path,
        hash,
        size,
        limit,
        &token,
        |path, hash, token| issue(ISSUER, path, hash, token),
        |url, limit| {
            let response = agent()
                .get(url.as_str())
                .call()
                .map_err(|_| "download failed".to_owned())?;
            read_response(response, limit)
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use sha2::{Digest, Sha256};
    use std::cell::Cell;

    fn hash(bytes: &[u8]) -> String {
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect()
    }

    #[test]
    fn issuer_request_sends_only_the_expected_path_and_hash() {
        use std::io::{Read as _, Write as _};
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let address = listener.local_addr().unwrap();
        let server = std::thread::spawn(move || {
            let (mut stream, _) = listener.accept().unwrap();
            stream
                .set_read_timeout(Some(Duration::from_secs(5)))
                .unwrap();
            let mut request = Vec::new();
            let mut chunk = [0; 4096];
            loop {
                let n = stream.read(&mut chunk).unwrap();
                assert!(n > 0);
                request.extend_from_slice(&chunk[..n]);
                if let Some(end) = request.windows(4).position(|window| window == b"\r\n\r\n") {
                    let head = std::str::from_utf8(&request[..end]).unwrap();
                    let length: usize = head
                        .lines()
                        .find_map(|line| {
                            line.to_ascii_lowercase()
                                .strip_prefix("content-length:")
                                .map(str::trim)
                                .map(str::to_owned)
                        })
                        .unwrap()
                        .parse()
                        .unwrap();
                    if request.len() >= end + 4 + length {
                        break;
                    }
                }
            }
            stream
                .write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}")
                .unwrap();
            request
        });
        let path = "/ferrigura/fonts/test/file.ttf";
        assert_eq!(
            issue(
                &format!("http://{address}/v1/downloads"),
                path,
                "sha",
                "synthetic-token"
            )
            .unwrap(),
            json!({})
        );
        let request = server.join().unwrap();
        let split = request
            .windows(4)
            .position(|window| window == b"\r\n\r\n")
            .unwrap();
        let headers = std::str::from_utf8(&request[..split])
            .unwrap()
            .to_ascii_lowercase();
        assert!(headers.starts_with("post /v1/downloads http/1.1"));
        assert!(headers.contains("authorization: bearer synthetic-token"));
        assert_eq!(
            serde_json::from_slice::<Value>(&request[split + 4..]).unwrap(),
            json!({"path":path,"sha256":"sha"})
        );
    }

    #[test]
    fn issues_only_known_path_and_verifies_result() {
        assert_eq!(agent().config().max_redirects(), 0);
        let body = b"face data";
        let sha = hash(body);
        let path = format!("/ferrigura/fonts/test/{sha}.ttf");
        let signed = format!("https://{HOST}{path}?token=signedsecret&expires=12345");
        let bytes = fetch_with(
            &path,
            &sha,
            Some(body.len() as u64),
            100,
            "bearersecret",
            |p, h, token| {
                assert_eq!(p, path);
                assert_eq!(h, sha);
                assert_eq!(token, "bearersecret");
                Ok(json!({"url":signed,"bytes":body.len(),"sha256":sha}))
            },
            |url, limit| {
                assert_eq!(url.path(), path);
                assert_eq!(limit, body.len() as u64);
                Ok(body.to_vec())
            },
        )
        .unwrap();
        assert_eq!(bytes, body);
    }

    #[test]
    fn rejects_bad_signed_urls_or_metadata_without_downloading_or_exposing_secrets() {
        let body = b"face data";
        let sha = hash(body);
        let path = format!("/ferrigura/fonts/test/{sha}.ttf");
        for suffix in [
            format!("http://{HOST}{path}?token=secret&expires=1"),
            format!("https://other.invalid{path}?token=secret&expires=1"),
            format!("https://user@{HOST}{path}?token=secret&expires=1"),
            format!("https://{HOST}{path}x?token=secret&expires=1"),
            format!("https://{HOST}{path}?token=secret&expires=1&extra=1"),
            format!("https://{HOST}{path}?token=secret&token=secret&expires=1"),
            format!("https://{HOST}{path}?token=secret&expires=no"),
            format!("https://{HOST}{path}?token=secret&expires=1#fragment"),
        ] {
            let called = Cell::new(false);
            let result = fetch_with(
                &path,
                &sha,
                Some(body.len() as u64),
                100,
                "bearersecret",
                |_, _, _| Ok(json!({"url":suffix,"bytes":body.len(),"sha256":sha})),
                |_, _| {
                    called.set(true);
                    Ok(body.to_vec())
                },
            );
            let error = result.unwrap_err();
            assert!(!called.get());
            assert!(!error.contains("secret"));
            assert!(!error.contains(HOST));
        }
        let response = json!({"url":format!("https://{HOST}{path}?token=secret&expires=1"),"bytes":body.len()+1,"sha256":sha});
        assert!(signed_url(&response, &path, &sha, Some(body.len() as u64), 100).is_err());
        let response = json!({"url":format!("https://{HOST}{path}?token=secret&expires=1"),"bytes":body.len(),"sha256":"0".repeat(64)});
        assert!(signed_url(&response, &path, &sha, None, 100).is_err());
        let failure = fetch_with(
            &path,
            &sha,
            None,
            100,
            "bearersecret",
            |_, _, _| Err("bearersecret signedsecret".into()),
            |_, _| unreachable!(),
        )
        .unwrap_err();
        assert!(!failure.contains("secret"));
        let failure = fetch_with(&path, &sha, None, 100, "bearersecret", |_,_,_| Ok(json!({"url":format!("https://{HOST}{path}?token=secret&expires=1"),"bytes":body.len(),"sha256":sha})), |_,_| Ok(b"bad".to_vec())).unwrap_err();
        assert!(failure.contains("integrity"));
    }

    #[test]
    fn refuses_non_catalog_paths_before_calling_the_issuer() {
        let sha = hash(b"face data");
        for path in [
            format!("/ferrigura/fonts/../{sha}.ttf"),
            format!("/ferrigura/fonts/test/not-{sha}.ttf"),
            format!("/outside/{sha}.ttf"),
        ] {
            let result = fetch_with(
                &path,
                &sha,
                None,
                100,
                "token",
                |_, _, _| unreachable!(),
                |_, _| unreachable!(),
            );
            assert!(result.is_err());
        }
    }
}
