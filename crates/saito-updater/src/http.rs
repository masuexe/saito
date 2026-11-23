//! Shared HTTP(S) client and streaming download helpers for desktop and Android updaters, with system HTTP(S)
//! proxy resolution on Windows and macOS so desktop update checks work behind local proxy tools.

use futures_util::StreamExt;
use std::io::Write;
use std::path::Path;
use std::sync::Once;
use std::time::Duration;

/// How long connecting and reading may wait before failing cleanly.
pub const CONNECT_WAIT: Duration = Duration::from_secs(8);
pub const READ_WAIT: Duration = Duration::from_secs(30);

/// Normalize an HTTP(S) proxy entry into a URL accepted by `reqwest::Proxy::all`.
/// SOCKS schemes are skipped because our locked `reqwest` build enables HTTP/HTTPS proxies only.
pub(crate) fn normalize_http_proxy(raw: &str) -> Option<String> {
    let trimmed = raw.trim();
    if trimmed.is_empty() || trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("http://") || lower.starts_with("https://") {
        return Some(trimmed.to_string());
    }
    if trimmed.contains("://") {
        return None;
    }
    Some(format!("http://{trimmed}"))
}

/// Parse Windows `reg query "HKCU\...\Internet Settings"` output for an enabled HTTP(S) proxy.
#[cfg_attr(not(any(target_os = "windows", test)), allow(dead_code))]
pub(crate) fn parse_windows_proxy(output: &str) -> Option<String> {
    let mut enabled = false;
    let mut server: Option<&str> = None;
    for line in output.lines() {
        let mut parts = line.split_whitespace();
        let Some(name) = parts.next() else { continue };
        let Some(_ty) = parts.next() else { continue };
        let value = parts.collect::<Vec<_>>().join(" ");
        if name.eq_ignore_ascii_case("ProxyEnable") {
            let trimmed = value.trim();
            enabled = trimmed == "1" || trimmed.eq_ignore_ascii_case("0x1") || trimmed.eq_ignore_ascii_case("0x00000001");
        } else if name.eq_ignore_ascii_case("ProxyServer") {
            // Grab the raw rest of the line after REG_SZ so whitespace checks still catch malformed entries.
            if let Some((_, rest)) = line.split_once("REG_SZ") {
                server = Some(rest.trim());
            }
        }
    }
    if !enabled {
        return None;
    }
    let server = server?.trim();
    if server.is_empty() {
        return None;
    }
    if !server.contains('=') {
        return normalize_http_proxy(server);
    }
    let mut http_fallback = None;
    for entry in server.split(';') {
        let Some((scheme, addr)) = entry.split_once('=') else { continue };
        let scheme = scheme.trim();
        if scheme.eq_ignore_ascii_case("https")
            && let Some(url) = normalize_http_proxy(addr)
        {
            return Some(url);
        }
        if scheme.eq_ignore_ascii_case("http") && http_fallback.is_none() {
            http_fallback = normalize_http_proxy(addr);
        }
    }
    http_fallback
}

/// Parse macOS `scutil --proxy` output for an enabled HTTPS or HTTP proxy.
#[cfg_attr(not(any(target_os = "macos", test)), allow(dead_code))]
pub(crate) fn parse_macos_proxy(output: &str) -> Option<String> {
    let (mut https_on, mut https_host, mut https_port) = (false, None, None);
    let (mut http_on, mut http_host, mut http_port) = (false, None, None);
    for line in output.lines() {
        let Some((key, val)) = line.split_once(':') else { continue };
        let (key, val) = (key.trim(), val.trim());
        match key {
            "HTTPSEnable" => https_on = val == "1",
            "HTTPSProxy" => https_host = Some(val),
            "HTTPSPort" => https_port = Some(val),
            "HTTPEnable" => http_on = val == "1",
            "HTTPProxy" => http_host = Some(val),
            "HTTPPort" => http_port = Some(val),
            _ => {}
        }
    }
    if https_on
        && let Some(host) = https_host
        && !host.is_empty()
    {
        let raw = match https_port.filter(|p| !p.is_empty()) {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        if let Some(url) = normalize_http_proxy(&raw) {
            return Some(url);
        }
    }
    if http_on
        && let Some(host) = http_host
        && !host.is_empty()
    {
        let raw = match http_port.filter(|p| !p.is_empty()) {
            Some(port) => format!("{host}:{port}"),
            None => host.to_string(),
        };
        return normalize_http_proxy(&raw);
    }
    None
}

#[cfg(any(desktop, target_os = "android"))]
fn resolve_http_proxy() -> Option<String> {
    for var in ["HTTPS_PROXY", "https_proxy", "HTTP_PROXY", "http_proxy", "ALL_PROXY", "all_proxy"] {
        if let Ok(val) = std::env::var(var)
            && !val.trim().is_empty()
        {
            return normalize_http_proxy(&val);
        }
    }
    #[cfg(target_os = "windows")]
    {
        use std::os::windows::process::CommandExt;
        const CREATE_NO_WINDOW: u32 = 0x0800_0000;
        if let Ok(out) = std::process::Command::new("reg")
            .args(["query", r"HKCU\Software\Microsoft\Windows\CurrentVersion\Internet Settings"])
            .creation_flags(CREATE_NO_WINDOW)
            .output()
            && out.status.success()
        {
            return parse_windows_proxy(&String::from_utf8_lossy(&out.stdout));
        }
    }
    #[cfg(target_os = "macos")]
    {
        if let Ok(out) = std::process::Command::new("scutil").arg("--proxy").output()
            && out.status.success()
        {
            return parse_macos_proxy(&String::from_utf8_lossy(&out.stdout));
        }
    }
    #[cfg(target_os = "android")]
    {
        if let Some(url) = crate::platform::system::http_proxy() {
            return Some(url);
        }
    }
    None
}

fn install_tls() {
    static PROVIDER: Once = Once::new();
    PROVIDER.call_once(|| drop(rustls::crypto::ring::default_provider().install_default()));
}

fn build_client(proxy: Option<&str>) -> Result<reqwest::Client, String> {
    install_tls();
    let mut builder = reqwest::Client::builder().user_agent("saito").connect_timeout(CONNECT_WAIT).read_timeout(READ_WAIT);
    match proxy {
        Some(proxy_url) => match reqwest::Proxy::all(proxy_url) {
            Ok(p) => builder = builder.proxy(p),
            Err(why) => {
                log::warn!("ignoring invalid updater HTTP proxy {proxy_url:?}: {why}");
                builder = builder.no_proxy();
            }
        },
        None => builder = builder.no_proxy(),
    }
    builder.build().map_err(|e| format!("an HTTP client: {e}"))
}

/// HTTP client pair: tries any configured system HTTP(S) proxy first and automatically falls back
/// to a direct `.no_proxy()` client if the proxy fails at the transport layer (e.g. stale or unreachable proxy).
pub struct Clients {
    primary: reqwest::Client,
    direct: Option<reqwest::Client>,
}

/// HTTP status codes from a proxy request that warrant retrying directly (proxy auth/gateway errors or
/// shared-node rate limiting such as GitHub's 403/429).
pub(crate) fn should_fallback_status(status: reqwest::StatusCode) -> bool {
    matches!(status.as_u16(), 403 | 407 | 429 | 502 | 503 | 504)
}

impl Clients {
    pub fn new() -> Result<Clients, String> {
        #[cfg(any(desktop, target_os = "android"))]
        let proxy = resolve_http_proxy();
        #[cfg(not(any(desktop, target_os = "android")))]
        let proxy: Option<String> = None;

        if let Some(proxy_url) = proxy {
            let primary = build_client(Some(&proxy_url))?;
            let direct = build_client(None).ok();
            Ok(Clients { primary, direct })
        } else {
            Ok(Clients { primary: build_client(None)?, direct: None })
        }
    }

    #[cfg(test)]
    pub(crate) fn direct_for_test() -> Result<Clients, String> {
        Ok(Clients { primary: build_client(None)?, direct: None })
    }

    async fn send_get(&self, url: &str, accept: Option<&str>) -> Result<reqwest::Response, String> {
        let build = |c: &reqwest::Client| {
            let req = c.get(url);
            match accept {
                Some(header) => req.header(reqwest::header::ACCEPT, header),
                None => req,
            }
        };
        match build(&self.primary).send().await {
            Ok(resp) => {
                if let Some(direct) = &self.direct
                    && should_fallback_status(resp.status())
                {
                    log::warn!("updater request via proxy returned {}; retrying direct", resp.status());
                    if let Ok(direct_resp) = build(direct).send().await
                        && direct_resp.status().is_success()
                    {
                        return Ok(direct_resp);
                    }
                }
                Ok(resp)
            }
            Err(first) => {
                if let Some(direct) = &self.direct {
                    log::warn!("updater request via proxy failed ({first}); retrying direct");
                    build(direct).send().await.map_err(|e| format!("{url}: {e} (proxy: {first})"))
                } else {
                    Err(format!("{url}: {first}"))
                }
            }
        }
    }

    /// GET `url` with an optional `Accept` header, enforcing `limit` bytes and handing each chunk,
    /// cumulative bytes read, and response `Content-Length` (if known) to `take`.
    pub async fn get_streaming(
        &self,
        url: &str,
        accept: Option<&str>,
        limit: u64,
        mut take: impl FnMut(&[u8], u64, Option<u64>) -> Result<(), String>,
    ) -> Result<(), String> {
        let said = |e: reqwest::Error| format!("{url}: {e}");
        let response = self.send_get(url, accept).await?;
        let status = response.status();
        if !status.is_success() {
            return Err(format!("{url}: {status}"));
        }
        let total = response.content_length();
        if total.is_some_and(|n| n > limit) {
            return Err(format!("{url}: over {limit} bytes"));
        }
        let mut got = 0u64;
        let mut pieces = response.bytes_stream();
        while let Some(piece) = pieces.next().await {
            let piece = piece.map_err(said)?;
            got += piece.len() as u64;
            if got > limit {
                return Err(format!("{url}: over {limit} bytes"));
            }
            take(&piece, got, total)?;
        }
        Ok(())
    }

    /// GET `url` into memory, up to `limit` bytes.
    pub async fn get_bytes(&self, url: &str, limit: u64) -> Result<Vec<u8>, String> {
        self.get_bytes_with_accept(url, None, limit).await
    }

    /// GET `url` with an optional `Accept` header into memory, up to `limit` bytes.
    pub async fn get_bytes_with_accept(&self, url: &str, accept: Option<&str>, limit: u64) -> Result<Vec<u8>, String> {
        let mut body = Vec::new();
        self.get_streaming(url, accept, limit, |piece, _, _| {
            body.extend_from_slice(piece);
            Ok(())
        })
        .await?;
        Ok(body)
    }

    /// Best-effort `HEAD` request to obtain `Content-Length` without downloading the body.
    #[cfg_attr(not(target_os = "android"), allow(dead_code))]
    pub async fn head_content_length(&self, url: &str) -> Option<u64> {
        let resp = match self.primary.head(url).send().await {
            Ok(r) => r,
            Err(_) => self.direct.as_ref()?.head(url).send().await.ok()?,
        };
        if !resp.status().is_success() {
            return None;
        }
        resp.content_length()
    }

    /// Download `url` directly to `path`, reporting integer percentages (0..=100) using `total_hint` when set.
    #[cfg_attr(target_os = "android", allow(dead_code))]
    pub async fn download_to_file(&self, url: &str, path: &Path, total_hint: u64, mut progress_pct: impl FnMut(i16)) -> Result<(), String> {
        let written = |e: std::io::Error| format!("{}: {e}", path.display());
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| format!("{}: {e}", parent.display()))?;
        }
        let mut file = std::fs::File::create(path).map_err(written)?;
        let mut last_pct = 0i16;
        let limit = if total_hint > 0 { total_hint.saturating_mul(2).max(512 << 20) } else { 512 << 20 };
        self.get_streaming(url, None, limit, |piece, got, content_len| {
            file.write_all(piece).map_err(written)?;
            let total = if total_hint > 0 { total_hint } else { content_len.unwrap_or(0) };
            if total > 0 {
                let pct = ((got.saturating_mul(100) / total).min(100)) as i16;
                if pct != last_pct {
                    last_pct = pct;
                    progress_pct(pct);
                }
            }
            Ok(())
        })
        .await?;
        file.flush().map_err(written)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn normalizes_http_proxies_and_skips_socks() {
        assert_eq!(normalize_http_proxy("127.0.0.1:7890"), Some("http://127.0.0.1:7890".into()));
        assert_eq!(normalize_http_proxy(" http://127.0.0.1:10809 "), Some("http://127.0.0.1:10809".into()));
        assert_eq!(normalize_http_proxy("HTTPS://proxy.local:8443"), Some("HTTPS://proxy.local:8443".into()));
        assert_eq!(normalize_http_proxy("socks5://127.0.0.1:7891"), None);
        assert_eq!(normalize_http_proxy(""), None);
    }

    #[test]
    fn parses_windows_registry_proxy_output() {
        let enabled = "\
HKEY_CURRENT_USER\\Software\\Microsoft\\Windows\\CurrentVersion\\Internet Settings
    ProxyEnable    REG_DWORD    0x1
    ProxyServer    REG_SZ    127.0.0.1:7890
";
        assert_eq!(parse_windows_proxy(enabled), Some("http://127.0.0.1:7890".into()));

        let per_scheme = "\
    ProxyEnable    REG_DWORD    0x00000001
    ProxyServer    REG_SZ    http=127.0.0.1:8080;https=127.0.0.1:8443;socks=127.0.0.1:1080
";
        assert_eq!(parse_windows_proxy(per_scheme), Some("http://127.0.0.1:8443".into()));

        let disabled = "\
    ProxyEnable    REG_DWORD    0x0
    ProxyServer    REG_SZ    127.0.0.1:7890
";
        assert_eq!(parse_windows_proxy(disabled), None);
    }

    #[test]
    fn parses_macos_scutil_proxy_output() {
        let sample = "\
<dictionary> {
  HTTPEnable : 1
  HTTPPort : 7890
  HTTPProxy : 127.0.0.1
  HTTPSEnable : 1
  HTTPSPort : 7890
  HTTPSProxy : 127.0.0.1
}
";
        assert_eq!(parse_macos_proxy(sample), Some("http://127.0.0.1:7890".into()));

        let disabled = "\
<dictionary> {
  HTTPEnable : 0
  HTTPSEnable : 0
}
";
        assert_eq!(parse_macos_proxy(disabled), None);
    }

    #[test]
    fn proxy_fallback_triggers_on_rate_limit_and_gateway_statuses() {
        for code in [403, 407, 429, 502, 503, 504] {
            assert!(should_fallback_status(reqwest::StatusCode::from_u16(code).unwrap()), "{code}");
        }
        for code in [200, 302, 404, 500] {
            assert!(!should_fallback_status(reqwest::StatusCode::from_u16(code).unwrap()), "{code}");
        }
    }
}
