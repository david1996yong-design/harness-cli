//! Code indexer -- produce a `.scan-cache.json` summarising the project.
//!
//! The cache is consumed by AI-side KB scan commands to skip their own
//! `find` + `cat` passes. It contains:
//!
//! - file clusters (grouped by top-level source directory)
//! - per-file language, line count, sha256, and detected entry points
//! - language stats and the current git commit (for freshness checks)
//!
//! Schema is versioned via [`SCHEMA_VERSION`]; bump when breaking the format.

use std::collections::BTreeMap;
use std::fs;
use std::io::Read;
use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;
use std::process::Command;

use anyhow::{Context, Result};
use ignore::WalkBuilder;
use regex::Regex;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

// =============================================================================
// Schema
// =============================================================================

/// Bump when making backwards-incompatible changes to the cache format.
pub const SCHEMA_VERSION: u32 = 1;

/// Files larger than this are flagged but still hashed/line-counted.
const LARGE_FILE_BYTES: u64 = 500 * 1024;
/// Files larger than this are listed with metadata only (no hash, no lines).
const SKIP_CONTENT_BYTES: u64 = 2 * 1024 * 1024;

/// Directory names that never belong to source clusters.
const HARD_EXCLUDE_DIRS: &[&str] = &[
    "target",
    "node_modules",
    "dist",
    "build",
    ".next",
    "__pycache__",
    ".harness-cli",
    ".git",
];

/// Top-level cache document written to `.harness-cli/kb/.scan-cache.json`.
#[derive(Debug, Serialize, Deserialize)]
pub struct ScanIndex {
    pub schema_version: u32,
    pub generated_at: String,
    pub git_commit: Option<String>,
    pub root: String,
    pub language_stats: BTreeMap<String, u32>,
    pub clusters: Vec<Cluster>,
    pub unclustered_files: Vec<FileEntry>,
}

/// A group of related source files, typically one top-level source directory.
#[derive(Debug, Serialize, Deserialize)]
pub struct Cluster {
    pub name: String,
    pub path: String,
    pub files: Vec<FileEntry>,
}

/// Metadata and detected entry points for a single source file.
#[derive(Debug, Serialize, Deserialize)]
pub struct FileEntry {
    pub path: String,
    pub language: String,
    pub lines: u32,
    pub sha256: Option<String>,
    pub entry_points: Vec<String>,
    #[serde(default, skip_serializing_if = "std::ops::Not::not")]
    pub large: bool,
}

// =============================================================================
// Public API
// =============================================================================

/// Build a [`ScanIndex`] for the project rooted at `root`.
pub fn build_index(root: &Path) -> Result<ScanIndex> {
    let root = root.canonicalize().context("Failed to canonicalize root")?;

    let walker = WalkBuilder::new(&root)
        .standard_filters(true)
        .hidden(false)
        .follow_links(false)
        .filter_entry(|entry| {
            entry
                .file_name()
                .to_str()
                .map(|name| !HARD_EXCLUDE_DIRS.contains(&name))
                .unwrap_or(true)
        })
        .build();

    let mut cluster_map: BTreeMap<String, Cluster> = BTreeMap::new();
    let mut unclustered: Vec<FileEntry> = Vec::new();
    let mut language_stats: BTreeMap<String, u32> = BTreeMap::new();

    for result in walker {
        let dent = match result {
            Ok(d) => d,
            Err(_) => continue,
        };
        let path = dent.path();
        if !path.is_file() {
            continue;
        }

        let rel = match path.strip_prefix(&root) {
            Ok(p) => p.to_path_buf(),
            Err(_) => continue,
        };

        // Skip HARD_EXCLUDE_DIRS even if deep in tree (filter_entry only sees dir entries).
        if rel.components().any(|c| {
            c.as_os_str()
                .to_str()
                .map(|s| HARD_EXCLUDE_DIRS.contains(&s))
                .unwrap_or(false)
        }) {
            continue;
        }

        let entry = match classify_file(path, &rel) {
            Some(e) => e,
            None => continue,
        };

        *language_stats.entry(entry.language.clone()).or_insert(0) += 1;

        let cluster_name = top_level_cluster(&rel);
        match cluster_name {
            Some(name) => {
                let cluster = cluster_map
                    .entry(name.clone())
                    .or_insert_with(|| Cluster {
                        name: name.clone(),
                        path: name.clone(),
                        files: Vec::new(),
                    });
                cluster.files.push(entry);
            }
            None => unclustered.push(entry),
        }
    }

    // Drop clusters that contain only non-source files.
    let clusters: Vec<Cluster> = cluster_map
        .into_values()
        .filter(|c| c.files.iter().any(|f| f.language != "other"))
        .collect();

    Ok(ScanIndex {
        schema_version: SCHEMA_VERSION,
        generated_at: current_timestamp(),
        git_commit: read_git_head(&root),
        root: root.to_string_lossy().to_string(),
        language_stats,
        clusters,
        unclustered_files: unclustered,
    })
}

/// Write a [`ScanIndex`] to `path` as pretty-printed JSON.
pub fn write_index(index: &ScanIndex, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create directory {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(index).context("Failed to serialize ScanIndex")?;
    fs::write(path, json).with_context(|| format!("Failed to write {}", path.display()))?;
    Ok(())
}

// =============================================================================
// File classification
// =============================================================================

fn classify_file(abs: &Path, rel: &Path) -> Option<FileEntry> {
    let metadata = fs::metadata(abs).ok()?;
    let size = metadata.len();

    let language = detect_language(rel);

    // Skip binary-likely files entirely (not listed).
    if language == "binary" {
        return None;
    }

    let rel_str = rel.to_string_lossy().replace('\\', "/");

    // For "other" (unknown text), we still include the entry but skip entry-point detection.
    let (lines, sha, entry_points, large) = if size > SKIP_CONTENT_BYTES {
        (0, None, Vec::new(), true)
    } else {
        let content = match fs::read_to_string(abs) {
            Ok(c) => c,
            Err(_) => return None, // non-UTF8 / unreadable
        };
        let lines = content.lines().count() as u32;
        let sha = sha256_hex(&content);
        let entries = if language == "other" {
            Vec::new()
        } else {
            detect_entry_points(&language, &content)
        };
        let large = size > LARGE_FILE_BYTES;
        (lines, Some(sha), entries, large)
    };

    Some(FileEntry {
        path: rel_str,
        language,
        lines,
        sha256: sha,
        entry_points,
        large,
    })
}

fn top_level_cluster(rel: &Path) -> Option<String> {
    let mut comps = rel.components();
    let first = comps.next()?.as_os_str().to_str()?;
    let second = comps.next().map(|c| c.as_os_str().to_str().unwrap_or(""));

    // Source roots we care about
    const SOURCE_ROOTS: &[&str] = &["src", "lib", "pkg", "internal", "cmd", "app", "packages"];

    if SOURCE_ROOTS.contains(&first) {
        // Inside src/, cluster at one level deeper if it's a subdir; otherwise cluster = first.
        match second {
            Some(sub) if !sub.is_empty() && rel_has_subdir(rel, first, sub) => {
                Some(format!("{}/{}", first, sub))
            }
            _ => Some(first.to_string()),
        }
    } else if first == "tests" || first == "examples" || first == "benches" {
        Some(first.to_string())
    } else {
        None
    }
}

/// True when `rel` is `{first}/{sub}/...` (at least 3 components starting with first/sub),
/// meaning `sub` is itself a directory under `first`.
fn rel_has_subdir(rel: &Path, first: &str, sub: &str) -> bool {
    let expected = Path::new(first).join(sub);
    rel.starts_with(&expected)
        && rel
            .components()
            .count()
            .checked_sub(expected.components().count())
            .map(|rest| rest >= 1)
            .unwrap_or(false)
}

// =============================================================================
// Language detection
// =============================================================================

fn detect_language(path: &Path) -> String {
    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_ascii_lowercase();

    match ext.as_str() {
        "rs" => "rust",
        "ts" | "tsx" => "typescript",
        "js" | "jsx" | "mjs" | "cjs" => "javascript",
        "py" => "python",
        "go" => "go",
        "java" => "java",
        "kt" | "kts" => "kotlin",
        "c" | "h" => "c",
        "cc" | "cpp" | "hpp" | "cxx" | "hxx" => "cpp",
        "rb" => "ruby",
        "php" => "php",
        "cs" => "csharp",
        "swift" => "swift",
        "md" | "markdown" => "markdown",
        "toml" => "toml",
        "yaml" | "yml" => "yaml",
        "json" => "json",
        "sh" | "bash" => "shell",
        "png" | "jpg" | "jpeg" | "gif" | "ico" | "pdf" | "zip" | "tar" | "gz" | "exe" | "bin"
        | "so" | "dylib" | "dll" | "o" | "a" | "class" | "jar" => "binary",
        _ => "other",
    }
    .to_string()
}

// =============================================================================
// Entry-point detection (MVP: Rust / TS/JS / Python / Go)
// =============================================================================

fn detect_entry_points(language: &str, content: &str) -> Vec<String> {
    let regexes: Vec<Regex> = match language {
        "rust" => vec![
            Regex::new(r"(?m)^pub\s+(?:async\s+)?(fn|struct|enum|trait|mod|type|const|static)\s+(\w+)").unwrap(),
            Regex::new(r"(?m)^pub\s+use\s+([\w:{}\s,*]+);").unwrap(),
            // Binary entry point (also used for `fn main()` in bins).
            Regex::new(r"(?m)^(?:async\s+)?fn\s+main\s*\(").unwrap(),
        ],
        "typescript" | "javascript" => vec![
            Regex::new(
                r"(?m)^export\s+(?:default\s+)?(?:async\s+)?(function|class|const|let|var|interface|type|enum)\s+(\w+)",
            )
            .unwrap(),
            Regex::new(r"(?m)^export\s+\{\s*([^}]+)\s*\}").unwrap(),
            Regex::new(r"(?m)^export\s+default\s+(\w+)").unwrap(),
        ],
        "python" => vec![
            // Module-level def/class (indent == 0)
            Regex::new(r"(?m)^(class|def)\s+(\w+)").unwrap(),
        ],
        "go" => vec![
            // Exported (capitalized) functions, types, vars, consts.
            Regex::new(r"(?m)^func\s+(?:\([^)]*\)\s+)?([A-Z]\w*)").unwrap(),
            Regex::new(r"(?m)^type\s+([A-Z]\w*)").unwrap(),
            Regex::new(r"(?m)^(?:var|const)\s+([A-Z]\w*)").unwrap(),
        ],
        _ => return Vec::new(),
    };

    let mut out: Vec<String> = Vec::new();
    for re in regexes {
        for cap in re.captures_iter(content) {
            let summary = cap
                .get(0)
                .map(|m| m.as_str().trim().to_string())
                .unwrap_or_default();
            if !summary.is_empty() && !out.contains(&summary) {
                out.push(summary);
            }
            if out.len() >= 50 {
                return out;
            }
        }
    }
    out
}

// =============================================================================
// Helpers
// =============================================================================

fn sha256_hex(s: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(s.as_bytes());
    format!("{:x}", hasher.finalize())
}

fn current_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let secs = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    format!("unix:{}", secs)
}

fn read_git_head(root: &Path) -> Option<String> {
    let output = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(root)
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let sha = String::from_utf8_lossy(&output.stdout).trim().to_string();
    if sha.is_empty() {
        None
    } else {
        Some(sha)
    }
}

// Retained for future use (content-based binary detection for files without known extensions).
#[allow(dead_code)]
fn looks_binary(path: &Path) -> bool {
    let mut file = match fs::File::open(path) {
        Ok(f) => f,
        Err(_) => return true,
    };
    let mut buf = [0u8; 8192];
    let n = file.read(&mut buf).unwrap_or(0);
    buf[..n].contains(&0)
}

// Convenience for tests: return a path inside a temp dir.
#[cfg(test)]
#[allow(dead_code)]
fn write_fixture(dir: &Path, rel: &str, body: &str) -> PathBuf {
    let full = dir.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).unwrap();
    }
    fs::write(&full, body).unwrap();
    full
}

// =============================================================================
// Tests
// =============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn detects_rust_entry_points() {
        let src = r#"
use std::io;

pub fn run() -> io::Result<()> { Ok(()) }

fn private_helper() {}

pub struct Config { pub name: String }

pub enum Mode { On, Off }

pub trait Greeter { fn hello(&self); }

fn main() { println!("hi"); }
"#;
        let eps = detect_entry_points("rust", src);
        let joined = eps.join(" || ");
        assert!(joined.contains("pub fn run"), "missing pub fn: {}", joined);
        assert!(joined.contains("pub struct Config"));
        assert!(joined.contains("pub enum Mode"));
        assert!(joined.contains("pub trait Greeter"));
        assert!(joined.contains("fn main"), "missing fn main: {}", joined);
        assert!(!joined.contains("private_helper"));
    }

    #[test]
    fn detects_typescript_entry_points() {
        let src = r#"
export function doThing() {}
export class Widget {}
export const FOO = 1;
function internal() {}
export default Widget;
"#;
        let eps = detect_entry_points("typescript", src);
        let joined = eps.join(" || ");
        assert!(joined.contains("export function doThing"));
        assert!(joined.contains("export class Widget"));
        assert!(joined.contains("export const FOO"));
        assert!(joined.contains("export default Widget"));
        assert!(!joined.contains("internal"));
    }

    #[test]
    fn detects_python_entry_points() {
        let src = r#"
import os

def top_level():
    pass

class Service:
    def method(self):
        pass

    def _private(self):
        pass
"#;
        let eps = detect_entry_points("python", src);
        let joined = eps.join(" || ");
        assert!(joined.contains("def top_level"));
        assert!(joined.contains("class Service"));
        // Indented def/method should NOT be picked up (they start with whitespace).
        assert!(!joined.contains("def method"));
        assert!(!joined.contains("def _private"));
    }

    #[test]
    fn detects_go_entry_points() {
        let src = r#"
package foo

func Public() {}
func private() {}
func (r *Receiver) Method() {}

type Widget struct{}
type internal struct{}

var Flag = 1
const Version = "x"
"#;
        let eps = detect_entry_points("go", src);
        let joined = eps.join(" || ");
        assert!(joined.contains("func Public"));
        assert!(joined.contains("Method"));
        assert!(joined.contains("type Widget"));
        assert!(joined.contains("var Flag") || joined.contains("const Version"));
        assert!(!joined.contains("private"));
        assert!(!joined.contains("type internal"));
    }

    #[test]
    fn unknown_language_returns_no_entry_points() {
        assert!(detect_entry_points("ruby", "class Foo; end").is_empty());
    }

    #[test]
    fn language_detection_covers_common_extensions() {
        assert_eq!(detect_language(Path::new("a.rs")), "rust");
        assert_eq!(detect_language(Path::new("a.ts")), "typescript");
        assert_eq!(detect_language(Path::new("a.tsx")), "typescript");
        assert_eq!(detect_language(Path::new("a.py")), "python");
        assert_eq!(detect_language(Path::new("a.go")), "go");
        assert_eq!(detect_language(Path::new("README.md")), "markdown");
        assert_eq!(detect_language(Path::new("logo.png")), "binary");
        assert_eq!(detect_language(Path::new("weird.xyz")), "other");
    }

    #[test]
    fn top_level_cluster_groups_by_src_subdir() {
        assert_eq!(
            top_level_cluster(Path::new("src/commands/init.rs")).as_deref(),
            Some("src/commands")
        );
        assert_eq!(
            top_level_cluster(Path::new("src/utils/foo.rs")).as_deref(),
            Some("src/utils")
        );
        // File directly under src/ falls into the "src" cluster.
        assert_eq!(
            top_level_cluster(Path::new("src/main.rs")).as_deref(),
            Some("src")
        );
        assert_eq!(
            top_level_cluster(Path::new("tests/integration.rs")).as_deref(),
            Some("tests")
        );
        // Top-level file (no source root) is unclustered.
        assert_eq!(top_level_cluster(Path::new("README.md")), None);
    }

    #[test]
    fn build_index_on_mini_project() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();

        fs::write(root.join("Cargo.toml"), "[package]\nname='x'\n").unwrap();
        fs::create_dir_all(root.join("src/commands")).unwrap();
        fs::write(root.join("src/main.rs"), "fn main() {}\npub fn run() {}\n").unwrap();
        fs::write(
            root.join("src/commands/init.rs"),
            "pub fn init() {}\npub struct Opts {}\n",
        )
        .unwrap();
        fs::create_dir_all(root.join("target/debug")).unwrap();
        fs::write(root.join("target/debug/ignored.rs"), "fn nope() {}").unwrap();
        fs::create_dir_all(root.join("node_modules/foo")).unwrap();
        fs::write(root.join("node_modules/foo/a.js"), "export const A=1").unwrap();

        let index = build_index(root).unwrap();

        assert_eq!(index.schema_version, SCHEMA_VERSION);
        assert!(
            index.clusters.iter().any(|c| c.name == "src/commands"),
            "clusters: {:?}",
            index.clusters.iter().map(|c| &c.name).collect::<Vec<_>>()
        );
        assert!(index.clusters.iter().any(|c| c.name == "src"));

        // Nothing from target/ or node_modules/ should be present.
        for c in &index.clusters {
            for f in &c.files {
                assert!(!f.path.starts_with("target/"), "leaked target: {}", f.path);
                assert!(
                    !f.path.starts_with("node_modules/"),
                    "leaked node_modules: {}",
                    f.path
                );
            }
        }

        // main.rs should have entry points detected.
        let src_cluster = index.clusters.iter().find(|c| c.name == "src").unwrap();
        let main_file = src_cluster
            .files
            .iter()
            .find(|f| f.path == "src/main.rs")
            .unwrap();
        assert!(main_file.entry_points.iter().any(|e| e.contains("pub fn run")));
    }

    #[test]
    fn write_and_read_back_index() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();

        let index = build_index(root).unwrap();
        let out = root.join(".scan-cache.json");
        write_index(&index, &out).unwrap();

        let text = fs::read_to_string(&out).unwrap();
        let parsed: ScanIndex = serde_json::from_str(&text).unwrap();
        assert_eq!(parsed.schema_version, SCHEMA_VERSION);
        assert!(!parsed.clusters.is_empty());
    }

    #[test]
    fn binary_files_are_skipped() {
        let tmp = TempDir::new().unwrap();
        let root = tmp.path();
        fs::write(root.join("logo.png"), b"\x89PNG\r\n").unwrap();
        fs::create_dir_all(root.join("src")).unwrap();
        fs::write(root.join("src/lib.rs"), "pub fn a() {}\n").unwrap();

        let index = build_index(root).unwrap();
        // PNG should not appear in unclustered or clusters.
        assert!(!index
            .unclustered_files
            .iter()
            .any(|f| f.path.ends_with("logo.png")));
        assert!(!index
            .clusters
            .iter()
            .flat_map(|c| c.files.iter())
            .any(|f| f.path.ends_with("logo.png")));
    }
}
