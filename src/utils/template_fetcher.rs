use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{anyhow, Result};
use serde::Deserialize;

// =============================================================================
// Constants
// =============================================================================

/// Default URL for the template index.
pub const TEMPLATE_INDEX_URL: &str =
    "https://raw.githubusercontent.com/mindfold-ai/harness-cli/main/marketplace/index.json";

/// Default giget-style repo source for templates.
const TEMPLATE_REPO: &str = "gh:mindfold-ai/harness-cli";

/// Timeout constants for network operations (milliseconds).
pub struct Timeouts;

impl Timeouts {
    /// Timeout for fetching the template index.
    pub const INDEX_FETCH_MS: u64 = 5_000;
    /// Timeout for downloading a template.
    pub const DOWNLOAD_MS: u64 = 30_000;
}

// =============================================================================
// Types
// =============================================================================

/// A template entry from the remote index.
#[derive(Debug, Clone, Deserialize)]
pub struct SpecTemplate {
    pub id: String,
    #[serde(rename = "type")]
    pub type_: String,
    pub name: String,
    pub description: Option<String>,
    pub path: String,
    pub tags: Option<Vec<String>>,
}

/// Internal structure for the full index response.
#[derive(Debug, Deserialize)]
struct TemplateIndex {
    #[allow(dead_code)]
    version: u32,
    templates: Vec<SpecTemplate>,
}

/// How to handle existing directories when downloading.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TemplateStrategy {
    Skip,
    Overwrite,
    Append,
}

/// Parsed components of a registry source string.
#[derive(Debug, Clone)]
pub struct RegistrySource {
    /// Original provider prefix (e.g., "gh", "gitlab", "bitbucket").
    pub provider: String,
    /// Repository path (e.g., "myorg/myrepo").
    pub repo: String,
    /// Subdirectory within the repo.
    pub subdir: String,
    /// Git ref / branch (default: "main").
    pub ref_: String,
    /// Base URL for fetching raw files (e.g., index.json).
    pub raw_base_url: String,
    /// Full giget source string for downloading.
    pub giget_source: String,
    /// Custom host for self-hosted instances. None for public providers.
    pub host: Option<String>,
}

// =============================================================================
// Install paths
// =============================================================================

/// Map template type to installation path.
fn install_paths() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("spec", ".harness-cli/spec");
    m.insert("skill", ".agents/skills");
    m.insert("command", ".claude/commands");
    m.insert("full", ".");
    m
}

// =============================================================================
// Raw URL patterns
// =============================================================================

/// Maps provider prefixes to raw file URL patterns.
fn raw_url_patterns() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("gh", "https://raw.githubusercontent.com/{repo}/{ref}/{subdir}");
    m.insert(
        "github",
        "https://raw.githubusercontent.com/{repo}/{ref}/{subdir}",
    );
    m.insert("gitlab", "https://gitlab.com/{repo}/-/raw/{ref}/{subdir}");
    m.insert(
        "bitbucket",
        "https://bitbucket.org/{repo}/raw/{ref}/{subdir}",
    );
    m
}

/// Known public domains mapped to their provider prefix.
fn public_domain_to_prefix() -> HashMap<&'static str, &'static str> {
    let mut m = HashMap::new();
    m.insert("github.com", "gh");
    m.insert("gitlab.com", "gitlab");
    m.insert("bitbucket.org", "bitbucket");
    m
}

const KNOWN_PUBLIC_DOMAINS: &[&str] = &["github.com", "gitlab.com", "bitbucket.org"];

// =============================================================================
// Registry Source Parsing
// =============================================================================

/// Convert an HTTPS URL to giget-style source format.
///
/// e.g. `https://github.com/user/repo` -> `gh:user/repo`
///      `https://github.com/user/repo/tree/branch/path` -> `gh:user/repo/path#branch`
///
/// Returns the original string if it's not a recognized HTTPS URL.
pub fn normalize_registry_source(source: &str) -> String {
    let patterns: Vec<(regex::Regex, &str)> = vec![
        (
            regex::Regex::new(r"^https?://github\.com/").unwrap(),
            "gh:",
        ),
        (
            regex::Regex::new(r"^https?://gitlab\.com/").unwrap(),
            "gitlab:",
        ),
        (
            regex::Regex::new(r"^https?://bitbucket\.org/").unwrap(),
            "bitbucket:",
        ),
    ];

    let tree_re = regex::Regex::new(
        r"^([^/]+/[^/]+)/tree/([^/]+)(?:/(.+?))?(?:\.git)?/?$",
    )
    .unwrap();
    let git_suffix_re = regex::Regex::new(r"\.git/?$").unwrap();

    for (re, prefix) in &patterns {
        if re.is_match(source) {
            let path = re.replace(source, "").to_string();
            // Handle /tree/<branch>/<subdir> format (GitHub browse URLs).
            if let Some(caps) = tree_re.captures(&path) {
                let repo = &caps[1];
                let ref_ = &caps[2];
                let subdir = caps.get(3).map(|m| m.as_str());
                return format!(
                    "{}{}{}#{}",
                    prefix,
                    repo,
                    subdir.map(|s| format!("/{}", s)).unwrap_or_default(),
                    ref_
                );
            }
            // Plain URL: strip trailing .git and /.
            let cleaned = git_suffix_re
                .replace(&path, "")
                .trim_end_matches('/')
                .to_string();
            return format!("{}{}", prefix, cleaned);
        }
    }

    source.to_string()
}

/// Parse a giget-style registry source into its components.
///
/// Supported input formats include giget prefix (e.g. `gh:org/repo`),
/// public HTTPS URLs, SSH URLs, and self-hosted instances.
pub fn parse_registry_source(source: &str) -> Result<RegistrySource> {
    let mut host: Option<String> = None;
    let mut normalized_input: Option<String> = None;

    // SSH URL: git@host:org/repo[.git] or ssh://git@host[:port]/org/repo[.git]
    let ssh_re1 = regex::Regex::new(r"^git@([^:]+):(.+?)(?:\.git)?/?$").unwrap();
    let ssh_re2 =
        regex::Regex::new(r"^ssh://git@([^/:]+)(?::\d+)?/(.+?)(?:\.git)?/?$").unwrap();

    let ssh_match = ssh_re1
        .captures(source)
        .or_else(|| ssh_re2.captures(source));

    if let Some(caps) = ssh_match {
        let ssh_domain = &caps[1];
        let ssh_path = &caps[2];
        let domain_map = public_domain_to_prefix();
        if let Some(prefix) = domain_map.get(ssh_domain) {
            normalized_input = Some(format!("{}:{}", prefix, ssh_path));
        } else {
            host = Some(ssh_domain.to_string());
            normalized_input = Some(format!("gitlab:{}", ssh_path));
        }
    }

    // HTTPS URL to unknown domain.
    if host.is_none() {
        let https_re =
            regex::Regex::new(r"^https?://([^/]+)/(.+?)(?:\.git)?/?$").unwrap();
        if let Some(caps) = https_re.captures(source) {
            let domain = &caps[1];
            if !KNOWN_PUBLIC_DOMAINS.contains(&domain) {
                host = Some(domain.to_string());
                let path_part = &caps[2];
                // Handle GitLab browse URLs: /org/repo/-/tree/branch/path
                let tree_re = regex::Regex::new(
                    r"^([^/]+/[^/]+)(?:/-)?/tree/([^/]+)(?:/(.+?))?$",
                )
                .unwrap();
                if let Some(tree_caps) = tree_re.captures(path_part) {
                    let repo_path = &tree_caps[1];
                    let ref_ = &tree_caps[2];
                    let subdir = tree_caps.get(3).map(|m| m.as_str());
                    normalized_input = Some(format!(
                        "gitlab:{}{}#{}",
                        repo_path,
                        subdir.map(|s| format!("/{}", s)).unwrap_or_default(),
                        ref_
                    ));
                } else {
                    normalized_input = Some(format!("gitlab:{}", path_part));
                }
            }
        }
    }

    let normalized = normalized_input
        .clone()
        .unwrap_or_else(|| normalize_registry_source(source));

    // Extract provider prefix.
    let colon_index = normalized
        .find(':')
        .ok_or_else(|| anyhow!(
            "Invalid registry source \"{}\". Expected format: gh:user/repo/path",
            source
        ))?;

    let provider = &normalized[..colon_index];
    let rest = &normalized[colon_index + 1..];

    // Check supported provider.
    let patterns = raw_url_patterns();
    let pattern = patterns.get(provider).ok_or_else(|| {
        let supported: Vec<&str> = patterns.keys().copied().collect();
        anyhow!(
            "Unsupported provider \"{}\". Supported: {}",
            provider,
            supported.join(", ")
        )
    })?;

    // Parse rest: user/repo/subdir#ref
    let ref_re = regex::Regex::new(r"^([^#]+?)(?:#(.+))?$").unwrap();
    let ref_match = ref_re.captures(rest).ok_or_else(|| {
        anyhow!(
            "Invalid registry source \"{}\". Expected format: {}:user/repo/path",
            normalized,
            provider
        )
    })?;

    let path_part = &ref_match[1];
    let ref_ = ref_match
        .get(2)
        .map(|m| m.as_str().to_string())
        .unwrap_or_else(|| "main".to_string());

    // Split into repo (first two segments) and subdir (rest).
    let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(anyhow!(
            "Invalid registry source \"{}\". Must include user/repo at minimum.",
            normalized
        ));
    }

    let repo = format!("{}/{}", segments[0], segments[1]);
    let subdir = segments[2..].join("/");

    // Build raw base URL.
    let mut raw_base_url = pattern
        .replace("{repo}", &repo)
        .replace("{ref}", &ref_)
        .replace("{subdir}", &subdir);

    // Replace public domain with self-hosted host in rawBaseUrl.
    if let Some(ref h) = host {
        if provider == "gitlab" {
            raw_base_url =
                raw_base_url.replace("https://gitlab.com", &format!("https://{}", h));
        }
    }

    let giget_source = normalized.clone();

    Ok(RegistrySource {
        provider: provider.to_string(),
        repo,
        subdir,
        ref_,
        raw_base_url,
        giget_source,
        host,
    })
}

// =============================================================================
// Fetch Template Index
// =============================================================================

/// Fetch available templates from the remote index.
/// Returns an empty vector on network error or timeout.
pub fn fetch_template_index(index_url: Option<&str>) -> Vec<SpecTemplate> {
    let url = index_url.unwrap_or(TEMPLATE_INDEX_URL);

    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(Timeouts::INDEX_FETCH_MS))
        .build()
    {
        Ok(c) => c,
        Err(_) => return Vec::new(),
    };

    let response = match client.get(url).send() {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    if !response.status().is_success() {
        return Vec::new();
    }

    match response.json::<TemplateIndex>() {
        Ok(index) => index.templates,
        Err(_) => Vec::new(),
    }
}

/// Probe a registry's index.json, distinguishing "not found" from transient errors.
///
/// - 404 -> `([], true)`
/// - Other HTTP error / network timeout -> `([], false)`
/// - 200 + valid JSON -> `(templates, false)`
pub fn probe_registry_index(index_url: &str) -> (Vec<SpecTemplate>, bool) {
    let client = match reqwest::blocking::Client::builder()
        .timeout(Duration::from_millis(Timeouts::INDEX_FETCH_MS))
        .build()
    {
        Ok(c) => c,
        Err(_) => return (Vec::new(), false),
    };

    let response = match client.get(index_url).send() {
        Ok(r) => r,
        Err(_) => return (Vec::new(), false),
    };

    if response.status().as_u16() == 404 {
        return (Vec::new(), true);
    }

    if !response.status().is_success() {
        return (Vec::new(), false);
    }

    match response.json::<TemplateIndex>() {
        Ok(index) => (index.templates, false),
        Err(_) => (Vec::new(), false),
    }
}

/// Find a template by ID from the index.
pub fn find_template(template_id: &str, index_url: Option<&str>) -> Option<SpecTemplate> {
    let templates = fetch_template_index(index_url);
    templates.into_iter().find(|t| t.id == template_id)
}

// =============================================================================
// Download Template
// =============================================================================

/// Get the installation path for a template type.
pub fn get_install_path(cwd: &Path, template_type: &str) -> PathBuf {
    let paths = install_paths();
    let relative_path = paths.get(template_type).copied().unwrap_or(".harness-cli/spec");
    cwd.join(relative_path)
}

/// Parse a giget-style source string into (provider, repo, subdir, ref).
fn parse_giget_source(source: &str) -> Result<(String, String, String, String)> {
    let colon_idx = source.find(':').ok_or_else(|| {
        anyhow!("Invalid giget source: {}", source)
    })?;

    let provider = &source[..colon_idx];
    let rest = &source[colon_idx + 1..];

    // Split on #ref
    let (path_part, ref_) = if let Some(hash_idx) = rest.find('#') {
        (&rest[..hash_idx], &rest[hash_idx + 1..])
    } else {
        (rest, "main")
    };

    let segments: Vec<&str> = path_part.split('/').filter(|s| !s.is_empty()).collect();
    if segments.len() < 2 {
        return Err(anyhow!("Invalid giget source: must include user/repo"));
    }

    let repo = format!("{}/{}", segments[0], segments[1]);
    let subdir = segments[2..].join("/");

    // Build git clone URL from provider.
    let clone_url = match provider {
        "gh" | "github" => format!("https://github.com/{}.git", repo),
        "gitlab" => format!("https://gitlab.com/{}.git", repo),
        "bitbucket" => format!("https://bitbucket.org/{}.git", repo),
        _ => return Err(anyhow!("Unsupported provider: {}", provider)),
    };

    Ok((clone_url, repo, subdir, ref_.to_string()))
}

/// Copy only files that don't exist in the destination (for append strategy).
fn copy_missing(src: &Path, dest: &Path) -> Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_missing(&src_path, &dest_path)?;
        } else if !dest_path.exists() {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Recursively copy a directory tree.
fn copy_dir_recursive(src: &Path, dest: &Path) -> Result<()> {
    if !dest.exists() {
        std::fs::create_dir_all(dest)?;
    }

    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let src_path = entry.path();
        let dest_path = dest.join(entry.file_name());

        if entry.file_type()?.is_dir() {
            copy_dir_recursive(&src_path, &dest_path)?;
        } else {
            std::fs::copy(&src_path, &dest_path)?;
        }
    }

    Ok(())
}

/// Download a template using git clone.
///
/// Since we don't have giget in Rust, this uses `git clone --depth 1` as a
/// simpler alternative:
///
/// 1. Parse the giget source string to get provider, repo, ref, subdir
/// 2. Use `git clone --depth 1 --branch <ref> <url> <temp_dir>`
/// 3. Copy the relevant subdirectory to the destination
///
/// `repo_source` - optional giget repo source override. When `None`, uses
/// `TEMPLATE_REPO`. When `Some(None)`, `template_path` is a full giget source.
pub fn download_with_strategy(
    template_path: &str,
    dest_dir: &Path,
    strategy: TemplateStrategy,
    repo_source: Option<Option<&str>>,
) -> Result<bool> {
    // Build the giget download source.
    let giget_source = match repo_source {
        Some(None) => template_path.to_string(), // Already a full giget source.
        Some(Some(src)) => format!("{}/{}", src, template_path),
        None => format!("{}/{}", TEMPLATE_REPO, template_path),
    };

    let exists = dest_dir.exists();

    // skip: Directory exists, don't download.
    if strategy == TemplateStrategy::Skip && exists {
        return Ok(false);
    }

    // overwrite: Delete existing directory first.
    if strategy == TemplateStrategy::Overwrite && exists {
        std::fs::remove_dir_all(dest_dir)?;
    }

    // append: Download to temp dir, then merge missing files.
    if strategy == TemplateStrategy::Append && exists {
        let temp_dir = tempfile::tempdir()?;
        download_via_git(&giget_source, temp_dir.path())?;
        copy_missing(temp_dir.path(), dest_dir)?;
        return Ok(true);
    }

    // Default: direct download (for new directory or after overwrite).
    download_via_git(&giget_source, dest_dir)?;
    Ok(true)
}

/// Perform the actual git clone and subdirectory extraction.
fn download_via_git(giget_source: &str, dest_dir: &Path) -> Result<()> {
    let (clone_url, _repo, subdir, ref_) = parse_giget_source(giget_source)?;

    let temp_dir = tempfile::tempdir()?;
    let temp_path = temp_dir.path();

    // Run git clone.
    let mut cmd = std::process::Command::new("git");
    cmd.args(["clone", "--depth", "1", "--branch", &ref_, &clone_url])
        .arg(temp_path)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null());

    let status = cmd.status().map_err(|e| anyhow!("Failed to run git: {}", e))?;

    if !status.success() {
        return Err(anyhow!(
            "git clone failed for {} (branch: {})",
            clone_url,
            ref_
        ));
    }

    // Copy the relevant subdirectory (or root) to destination.
    let source_dir = if subdir.is_empty() {
        temp_path.to_path_buf()
    } else {
        temp_path.join(&subdir)
    };

    if !source_dir.exists() {
        return Err(anyhow!(
            "Subdirectory \"{}\" not found in repository",
            subdir
        ));
    }

    // Ensure destination parent exists.
    if let Some(parent) = dest_dir.parent() {
        std::fs::create_dir_all(parent)?;
    }

    copy_dir_recursive(&source_dir, dest_dir)?;

    // Remove .git directory from destination if it was copied.
    let git_dir = dest_dir.join(".git");
    if git_dir.exists() {
        let _ = std::fs::remove_dir_all(&git_dir);
    }

    Ok(())
}

/// Download a template by ID.
///
/// Returns a result with success status, message, and optional skipped flag.
pub fn download_template_by_id(
    cwd: &Path,
    template_id: &str,
    strategy: TemplateStrategy,
    template: Option<&SpecTemplate>,
    registry: Option<&RegistrySource>,
    dest_dir_override: Option<&Path>,
) -> DownloadResult {
    // Use pre-fetched template or find from index.
    let resolved: Option<SpecTemplate> = if let Some(t) = template {
        Some(t.clone())
    } else {
        let index_url = registry.map(|r| format!("{}/index.json", r.raw_base_url));
        if let Some(_reg) = registry {
            if let Some(ref url) = index_url {
                let (templates, is_not_found) = probe_registry_index(url);
                if templates.is_empty() && !is_not_found {
                    return DownloadResult {
                        success: false,
                        message: "Could not reach registry. Check your network connection and try again.".to_string(),
                        skipped: false,
                    };
                }
                if is_not_found {
                    return DownloadResult {
                        success: false,
                        message: "Registry has no index.json. Remove --template to use direct download mode.".to_string(),
                        skipped: false,
                    };
                }
                templates.into_iter().find(|t| t.id == template_id)
            } else {
                find_template(template_id, None)
            }
        } else {
            find_template(template_id, index_url.as_deref())
        }
    };

    let resolved = match resolved {
        Some(t) => t,
        None => {
            return DownloadResult {
                success: false,
                message: format!("Template \"{}\" not found", template_id),
                skipped: false,
            };
        }
    };

    // Only support spec type in MVP.
    if resolved.type_ != "spec" {
        return DownloadResult {
            success: false,
            message: format!(
                "Template type \"{}\" is not supported yet (only \"spec\" is supported)",
                resolved.type_
            ),
            skipped: false,
        };
    }

    // Get destination path.
    let dest_dir = dest_dir_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| get_install_path(cwd, &resolved.type_));

    // Check if directory exists for skip strategy.
    if strategy == TemplateStrategy::Skip && dest_dir.exists() {
        return DownloadResult {
            success: true,
            message: format!("Skipped: {} already exists", dest_dir.display()),
            skipped: true,
        };
    }

    // Download template.
    let result = if let Some(reg) = registry {
        let full_source = format!(
            "{}:{}/{}#{}",
            reg.provider, reg.repo, resolved.path, reg.ref_
        );
        download_with_strategy(&full_source, &dest_dir, strategy, Some(None))
    } else {
        download_with_strategy(&resolved.path, &dest_dir, strategy, None)
    };

    match result {
        Ok(_) => DownloadResult {
            success: true,
            message: format!(
                "Downloaded template \"{}\" to {}",
                template_id,
                dest_dir.display()
            ),
            skipped: false,
        },
        Err(e) => {
            let error_message = e.to_string();
            if error_message.contains("timed out") {
                DownloadResult {
                    success: false,
                    message: "Download timed out. Check your network connection and try again."
                        .to_string(),
                    skipped: false,
                }
            } else if error_message.contains("Failed to download")
                || error_message.contains("Failed to fetch")
            {
                DownloadResult {
                    success: false,
                    message:
                        "Could not reach template server. Check your network connection."
                            .to_string(),
                    skipped: false,
                }
            } else {
                DownloadResult {
                    success: false,
                    message: format!("Download failed: {}", error_message),
                    skipped: false,
                }
            }
        }
    }
}

/// Download a registry source directly to the spec directory (no index.json).
/// Used when the registry source points to a spec directory, not a marketplace.
pub fn download_registry_direct(
    cwd: &Path,
    registry: &RegistrySource,
    strategy: TemplateStrategy,
    dest_dir_override: Option<&Path>,
) -> DownloadResult {
    let dest_dir = dest_dir_override
        .map(|p| p.to_path_buf())
        .unwrap_or_else(|| get_install_path(cwd, "spec"));

    if strategy == TemplateStrategy::Skip && dest_dir.exists() {
        return DownloadResult {
            success: true,
            message: format!("Skipped: {} already exists", dest_dir.display()),
            skipped: true,
        };
    }

    let result = download_with_strategy(
        &registry.giget_source,
        &dest_dir,
        strategy,
        Some(None), // giget_source is already a full source.
    );

    match result {
        Ok(_) => DownloadResult {
            success: true,
            message: format!(
                "Downloaded spec from {} to {}",
                registry.giget_source,
                dest_dir.display()
            ),
            skipped: false,
        },
        Err(e) => {
            let error_message = e.to_string();
            if error_message.contains("timed out") {
                DownloadResult {
                    success: false,
                    message: "Download timed out. Check your network connection and try again."
                        .to_string(),
                    skipped: false,
                }
            } else if error_message.contains("Failed to download")
                || error_message.contains("Failed to fetch")
            {
                DownloadResult {
                    success: false,
                    message:
                        "Could not reach template server. Check your network connection."
                            .to_string(),
                    skipped: false,
                }
            } else {
                DownloadResult {
                    success: false,
                    message: format!("Download failed: {}", error_message),
                    skipped: false,
                }
            }
        }
    }
}

/// Result of a download operation.
#[derive(Debug, Clone)]
pub struct DownloadResult {
    pub success: bool,
    pub message: String,
    pub skipped: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // normalize_registry_source
    // ---------------------------------------------------------------

    #[test]
    fn test_normalize_github_https() {
        let result = normalize_registry_source("https://github.com/user/repo");
        assert_eq!(result, "gh:user/repo");
    }

    #[test]
    fn test_normalize_github_tree() {
        let result =
            normalize_registry_source("https://github.com/user/repo/tree/dev/some/path");
        assert_eq!(result, "gh:user/repo/some/path#dev");
    }

    #[test]
    fn test_normalize_strip_git() {
        let result = normalize_registry_source("https://github.com/user/repo.git");
        assert_eq!(result, "gh:user/repo");
    }

    #[test]
    fn test_normalize_gitlab() {
        let result = normalize_registry_source("https://gitlab.com/org/project");
        assert_eq!(result, "gitlab:org/project");
    }

    #[test]
    fn test_normalize_passthrough() {
        // Already giget-style, should pass through unchanged.
        let result = normalize_registry_source("gh:user/repo/subdir#main");
        assert_eq!(result, "gh:user/repo/subdir#main");
    }

    // ---------------------------------------------------------------
    // parse_registry_source
    // ---------------------------------------------------------------

    #[test]
    fn test_parse_gh_repo_subdir() {
        let src = parse_registry_source("gh:user/repo/subdir").unwrap();
        assert_eq!(src.provider, "gh");
        assert_eq!(src.repo, "user/repo");
        assert_eq!(src.subdir, "subdir");
        assert_eq!(src.ref_, "main");
    }

    #[test]
    fn test_parse_gh_repo_ref() {
        let src = parse_registry_source("gh:user/repo/path#v2").unwrap();
        assert_eq!(src.provider, "gh");
        assert_eq!(src.repo, "user/repo");
        assert_eq!(src.subdir, "path");
        assert_eq!(src.ref_, "v2");
    }

    #[test]
    fn test_parse_gh_no_subdir() {
        let src = parse_registry_source("gh:user/repo").unwrap();
        assert_eq!(src.provider, "gh");
        assert_eq!(src.repo, "user/repo");
        assert!(src.subdir.is_empty());
    }

    #[test]
    fn test_parse_gitlab_with_ref() {
        let src = parse_registry_source("gitlab:user/repo/path#dev").unwrap();
        assert_eq!(src.provider, "gitlab");
        assert_eq!(src.repo, "user/repo");
        assert_eq!(src.subdir, "path");
        assert_eq!(src.ref_, "dev");
    }

    #[test]
    fn test_parse_error_no_colon() {
        let result = parse_registry_source("userrepo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_unsupported() {
        let result = parse_registry_source("svn:user/repo");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_error_no_repo() {
        let result = parse_registry_source("gh:user");
        assert!(result.is_err());
    }

    #[test]
    fn test_parse_ssh_github() {
        let src = parse_registry_source("git@github.com:user/repo").unwrap();
        assert_eq!(src.provider, "gh");
        assert_eq!(src.repo, "user/repo");
        assert!(src.host.is_none());
    }

    #[test]
    fn test_parse_ssh_self_hosted() {
        let src = parse_registry_source("git@git.corp.com:org/repo").unwrap();
        assert_eq!(src.provider, "gitlab");
        assert_eq!(src.repo, "org/repo");
        assert_eq!(src.host.as_deref(), Some("git.corp.com"));
    }

    #[test]
    fn test_parse_https_self_hosted() {
        let src = parse_registry_source("https://git.corp.com/org/repo").unwrap();
        assert_eq!(src.provider, "gitlab");
        assert_eq!(src.repo, "org/repo");
        assert_eq!(src.host.as_deref(), Some("git.corp.com"));
    }

    // ---------------------------------------------------------------
    // get_install_path
    // ---------------------------------------------------------------

    #[test]
    fn test_get_install_path_spec() {
        let tmp = tempfile::tempdir().unwrap();
        let path = get_install_path(tmp.path(), "spec");
        assert_eq!(path, tmp.path().join(".harness-cli/spec"));
    }

    #[test]
    fn test_get_install_path_skill() {
        let tmp = tempfile::tempdir().unwrap();
        let path = get_install_path(tmp.path(), "skill");
        assert_eq!(path, tmp.path().join(".agents/skills"));
    }

    #[test]
    fn test_get_install_path_unknown() {
        let tmp = tempfile::tempdir().unwrap();
        let path = get_install_path(tmp.path(), "nonexistent_type");
        // Defaults to spec path.
        assert_eq!(path, tmp.path().join(".harness-cli/spec"));
    }
}
