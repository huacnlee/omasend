//! Redaction applied while facts are collected, before they reach a report.

/// Replaces the user-info part of every `scheme://user:pass@host` URL.
pub fn url_credentials(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(start) = rest.find("://") {
        let (head, tail) = rest.split_at(start + 3);
        out.push_str(head);
        let end = tail
            .find(|c: char| c == '/' || c.is_whitespace() || c == '"' || c == '\'')
            .unwrap_or(tail.len());
        let authority = &tail[..end];
        match authority.rfind('@') {
            Some(at) => {
                out.push_str("***");
                out.push_str(&authority[at..]);
            }
            None => out.push_str(authority),
        }
        rest = &tail[end..];
    }
    out.push_str(rest);
    out
}

/// Shortens hexadecimal runs of 32+ characters (fingerprints, hashes) to
/// their first eight characters.
pub fn fingerprints(text: &str) -> String {
    let mut out = String::with_capacity(text.len());
    let mut run = String::new();
    let flush = |run: &mut String, out: &mut String| {
        if run.len() >= 32 {
            out.push_str(&run[..8]);
            out.push('…');
        } else {
            out.push_str(run);
        }
        run.clear();
    };
    for c in text.chars() {
        if c.is_ascii_hexdigit() {
            run.push(c);
        } else {
            flush(&mut run, &mut out);
            out.push(c);
        }
    }
    flush(&mut run, &mut out);
    out
}

/// Every redaction that applies to free text (command output, logs).
pub fn text(text: &str) -> String {
    fingerprints(&url_credentials(text))
}

/// The first eight characters of a fingerprint.
pub fn short_fingerprint(fingerprint: &str) -> String {
    let short: String = fingerprint.chars().take(8).collect();
    if fingerprint.chars().count() > 8 {
        format!("{short}…")
    } else {
        short
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn removes_proxy_credentials() {
        assert_eq!(
            url_credentials("https_proxy=http://alice:s3cret@10.0.0.1:8080/x"),
            "https_proxy=http://***@10.0.0.1:8080/x"
        );
        assert_eq!(
            url_credentials("socks5://127.0.0.1:6152 and http://u@h"),
            "socks5://127.0.0.1:6152 and http://***@h"
        );
        assert_eq!(url_credentials("no urls here"), "no urls here");
    }

    #[test]
    fn shortens_fingerprints_only() {
        let fingerprint = "B97BE6652ED1394ECB1628232073159D0837223F0AA26890601951B3964C9E46";
        assert_eq!(
            fingerprints(&format!("peer {fingerprint} at 10.20.94.213")),
            "peer B97BE665… at 10.20.94.213"
        );
        assert_eq!(fingerprints("deadbeef cafe"), "deadbeef cafe");
    }

    #[test]
    fn short_fingerprint_keeps_eight() {
        assert_eq!(short_fingerprint("0123456789abcdef"), "01234567…");
        assert_eq!(short_fingerprint("abc"), "abc");
    }
}
