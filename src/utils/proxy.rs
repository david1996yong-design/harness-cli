/// Mask credentials in a proxy URL for safe logging.
///
/// Replaces username and password with `***` so that credentials
/// are never printed to the console or written to log files.
pub fn mask_proxy_url(url: &str) -> String {
    // Try to parse as a URL with an authority section.
    // Format: scheme://[user[:pass]@]host[:port]/...
    if let Some(scheme_end) = url.find("://") {
        let scheme = &url[..scheme_end];
        let rest = &url[scheme_end + 3..];

        if let Some(at_pos) = rest.find('@') {
            // There are credentials -- mask them.
            let after_at = &rest[at_pos + 1..];
            return format!("{}://***:***@{}", scheme, after_at);
        }

        // No credentials in URL, return as-is.
        return url.to_string();
    }

    // Not a recognizable URL, mask entirely for safety.
    "***".to_string()
}

/// Check proxy environment variables and return the proxy URL if found.
///
/// Checks in order: `HTTPS_PROXY`, `https_proxy`, `HTTP_PROXY`, `http_proxy`,
/// `ALL_PROXY`, `all_proxy`.
///
/// Note: `reqwest` handles proxy configuration from environment variables
/// automatically. This function is mainly used for display/logging purposes.
pub fn setup_proxy() -> Option<String> {
    let candidates = [
        "HTTPS_PROXY",
        "https_proxy",
        "HTTP_PROXY",
        "http_proxy",
        "ALL_PROXY",
        "all_proxy",
    ];

    for var_name in &candidates {
        if let Ok(value) = std::env::var(var_name) {
            if !value.is_empty() {
                return Some(value);
            }
        }
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mask_proxy_url_with_credentials() {
        assert_eq!(
            mask_proxy_url("http://user:pass@proxy.example.com:8080"),
            "http://***:***@proxy.example.com:8080"
        );
    }

    #[test]
    fn test_mask_proxy_url_without_credentials() {
        assert_eq!(
            mask_proxy_url("http://proxy.example.com:8080"),
            "http://proxy.example.com:8080"
        );
    }

    #[test]
    fn test_mask_proxy_url_invalid() {
        assert_eq!(mask_proxy_url("not-a-url"), "***");
    }

    #[test]
    fn test_mask_proxy_url_with_user_only() {
        assert_eq!(
            mask_proxy_url("http://user@proxy.example.com"),
            "http://***:***@proxy.example.com"
        );
    }
}
