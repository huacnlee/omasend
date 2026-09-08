//! Stable release checks and verified in-app installation.
pub mod install;

pub static RESTART_REQUESTED: std::sync::atomic::AtomicBool =
    std::sync::atomic::AtomicBool::new(false);
use anyhow::{Context, Result, ensure};
use semver::Version;

pub const LATEST_API: &str = "https://api.github.com/repos/huacnlee/omasend/releases/latest";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub enum UpdateState {
    #[default]
    Idle,
    Checking,
    Current,
    NoRelease,
    Available {
        version: String,
        url: String,
    },
    Failed,
    Installing {
        downloaded: u64,
        total: Option<u64>,
    },
    Ready {
        version: String,
    },
    InstallFailed {
        version: String,
    },
}

fn client_builder() -> localsend::reqwest::ClientBuilder {
    // Update checks may start before the LocalSend server initializes TLS.
    let _ = rustls::crypto::ring::default_provider().install_default();
    localsend::reqwest::Client::builder()
}

pub async fn check() -> Result<UpdateState> {
    let client = client_builder()
        .user_agent(concat!("OmaSend/", env!("CARGO_PKG_VERSION")))
        .timeout(std::time::Duration::from_secs(15))
        .build()?;
    check_at(&client, LATEST_API, env!("CARGO_PKG_VERSION")).await
}

async fn check_at(
    client: &localsend::reqwest::Client,
    endpoint: &str,
    current: &str,
) -> Result<UpdateState> {
    let response = client
        .get(endpoint)
        .header("Accept", "application/vnd.github+json")
        .header("X-GitHub-Api-Version", "2022-11-28")
        .send()
        .await?;
    if response.status() == localsend::reqwest::StatusCode::NOT_FOUND {
        return Ok(UpdateState::NoRelease);
    }
    let mut response = response.error_for_status()?;
    let mut data = Vec::new();
    while let Some(chunk) = response.chunk().await? {
        ensure!(
            data.len() + chunk.len() <= 1024 * 1024,
            "Release response is too large"
        );
        data.extend_from_slice(&chunk);
    }
    evaluate(&serde_json::from_slice(&data)?, current)
}

fn evaluate(release: &serde_json::Value, current: &str) -> Result<UpdateState> {
    ensure!(
        release
            .get("draft")
            .and_then(|value| value.as_bool())
            .is_some(),
        "Missing release visibility"
    );
    ensure!(
        release
            .get("prerelease")
            .and_then(|value| value.as_bool())
            .is_some(),
        "Missing release channel"
    );
    if release["draft"] == true || release["prerelease"] == true {
        return Ok(UpdateState::NoRelease);
    }
    let tag = release["tag_name"]
        .as_str()
        .context("Missing release version")?;
    let version =
        Version::parse(tag.strip_prefix('v').unwrap_or(tag)).context("Invalid release version")?;
    if !version.pre.is_empty() {
        return Ok(UpdateState::NoRelease);
    }
    let current = Version::parse(current)?;
    if version.cmp_precedence(&current).is_le() {
        return Ok(UpdateState::Current);
    }
    Ok(UpdateState::Available {
        version: format!("v{version}"),
        url: format!("https://github.com/huacnlee/omasend/releases/tag/{tag}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    fn release(tag: &str) -> serde_json::Value {
        serde_json::json!({"tag_name": tag, "draft": false, "prerelease": false})
    }
    #[test]
    fn compares_semantic_versions_and_rejects_unstable_or_invalid_releases() {
        assert!(matches!(
            evaluate(&release("v0.10.0"), "0.9.0").unwrap(),
            UpdateState::Available { .. }
        ));
        for tag in ["v0.1.0", "v0.0.9", "v0.1.0+build"] {
            assert_eq!(
                evaluate(&release(tag), "0.1.0").unwrap(),
                UpdateState::Current
            );
        }
        assert_eq!(
            evaluate(&release("v0.2.0-beta.1"), "0.1.0").unwrap(),
            UpdateState::NoRelease
        );
        let mut draft = release("v0.2.0");
        draft["draft"] = true.into();
        assert_eq!(evaluate(&draft, "0.1.0").unwrap(), UpdateState::NoRelease);
        assert!(evaluate(&release("garbage"), "0.1.0").is_err());
        assert!(evaluate(&serde_json::json!({}), "0.1.0").is_err());
    }
    #[tokio::test]
    async fn handles_missing_release_rate_limit_and_valid_response() {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        for (status, body, expected) in [
            (404, "{}", Some(UpdateState::NoRelease)),
            (403, "{}", None),
            (200, "not json", None),
            (
                200,
                r#"{"tag_name":"v0.1.0","draft":false,"prerelease":false}"#,
                Some(UpdateState::Current),
            ),
        ] {
            let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
            let endpoint = format!("http://{}/latest", listener.local_addr().unwrap());
            let server = tokio::spawn(async move {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut buffer = [0; 4096];
                let mut request = Vec::new();
                while !request.windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                    let read = socket.read(&mut buffer).await.unwrap();
                    assert!(read > 0);
                    request.extend_from_slice(&buffer[..read]);
                    assert!(request.len() < 16384);
                }
                let request = String::from_utf8(request).unwrap().to_ascii_lowercase();
                assert!(request.contains("accept: application/vnd.github+json"));
                assert!(request.contains("x-github-api-version: 2022-11-28"));
                socket.write_all(format!("HTTP/1.1 {status} Test\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}", body.len()).as_bytes()).await.unwrap();
            });
            let client = client_builder().no_proxy().build().unwrap();
            let result = check_at(&client, &endpoint, "0.1.0").await;
            match expected {
                Some(expected) => assert_eq!(result.unwrap(), expected),
                None => assert!(result.is_err()),
            }
            server.await.unwrap();
        }
    }
}
