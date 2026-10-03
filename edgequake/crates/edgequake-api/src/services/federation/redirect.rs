//! Post-login redirect hygiene (SPEC-158 LAW-158-4 / EC-158-30).

/// Return `candidate` only when it is a same-origin absolute *path*; otherwise `/`.
///
/// Rejects scheme-relative (`//evil`), absolute URLs, backslash tricks, control characters and
/// anything not starting with a single `/`.
pub fn safe_redirect_path(candidate: Option<&str>) -> String {
    let Some(path) = candidate.map(str::trim) else {
        return "/".to_string();
    };
    let bad = path.is_empty()
        || !path.starts_with('/')
        || path.starts_with("//")
        || path.contains('\\')
        || path.contains("://")
        || path.chars().any(char::is_control)
        // `/\evil` and `/%5Cevil` normalise to `//evil` in some browsers.
        || path.to_ascii_lowercase().starts_with("/%5c")
        || path.to_ascii_lowercase().starts_with("/%2f");
    if bad {
        "/".to_string()
    } else {
        path.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_plain_paths() {
        assert_eq!(safe_redirect_path(Some("/documents?x=1")), "/documents?x=1");
    }

    #[test]
    fn rejects_open_redirect_vectors() {
        for bad in [
            "//evil.test",
            "https://evil.test",
            "/\\evil.test",
            "javascript:alert(1)",
            "/%5Cevil.test",
            "/%2fevil.test",
            "evil",
            "",
            "/a\nb",
            "/x://y",
        ] {
            assert_eq!(safe_redirect_path(Some(bad)), "/", "{bad:?}");
        }
        assert_eq!(safe_redirect_path(None), "/");
    }
}
