use std::path::Path;

use regex::Regex;

// =============================================================================
// Types
// =============================================================================

/// Project type detected by analyzing project files.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectType {
    Frontend,
    Backend,
    Fullstack,
    Unknown,
}

/// A detected package in a monorepo workspace.
#[derive(Debug, Clone)]
pub struct DetectedPackage {
    /// Package name (from package.json, Cargo.toml, go.mod, etc.)
    pub name: String,
    /// Relative path from cwd, normalized (no ./ prefix or trailing /)
    pub path: String,
    /// Project type detected via `detect_project_type()`
    pub type_: ProjectType,
    /// Whether this package is a git submodule
    pub is_submodule: bool,
}

// =============================================================================
// Indicator Lists
// =============================================================================

/// Files that indicate a frontend project.
const FRONTEND_INDICATORS: &[&str] = &[
    "package.json",
    "vite.config.ts",
    "vite.config.js",
    "next.config.js",
    "next.config.ts",
    "next.config.mjs",
    "nuxt.config.ts",
    "nuxt.config.js",
    "webpack.config.js",
    "rollup.config.js",
    "svelte.config.js",
    "astro.config.mjs",
    "angular.json",
    "vue.config.js",
    "src/App.tsx",
    "src/App.jsx",
    "src/App.vue",
    "src/app/page.tsx",
    "app/page.tsx",
    "pages/index.tsx",
    "pages/index.jsx",
];

/// Files that indicate a backend project.
const BACKEND_INDICATORS: &[&str] = &[
    "go.mod",
    "go.sum",
    "Cargo.toml",
    "Cargo.lock",
    "requirements.txt",
    "pyproject.toml",
    "setup.py",
    "Pipfile",
    "poetry.lock",
    "pom.xml",
    "build.gradle",
    "build.gradle.kts",
    "Gemfile",
    "composer.json",
    "*.csproj",
    "*.fsproj",
    "mix.exs",
    "server.ts",
    "server.js",
    "src/server.ts",
    "src/index.ts",
];

/// Frontend dependencies in package.json.
const FRONTEND_DEPS: &[&str] = &[
    "react",
    "vue",
    "svelte",
    "angular",
    "@angular/core",
    "next",
    "nuxt",
    "astro",
    "solid-js",
    "preact",
    "lit",
    "@remix-run/react",
];

/// Backend dependencies in package.json.
const BACKEND_DEPS: &[&str] = &[
    "express",
    "fastify",
    "hono",
    "koa",
    "hapi",
    "nest",
    "@nestjs/core",
    "fastapi",
    "flask",
    "django",
];

// =============================================================================
// Helpers
// =============================================================================

/// Check if a file exists in the project directory, with glob support for `*`.
fn file_exists(cwd: &Path, filename: &str) -> bool {
    if filename.contains('*') {
        let parent = Path::new(filename).parent().unwrap_or(Path::new("."));
        let pattern = Path::new(filename)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_default();
        let search_dir = if parent == Path::new(".") {
            cwd.to_path_buf()
        } else {
            cwd.join(parent)
        };

        if !search_dir.exists() {
            return false;
        }

        let regex_pattern = format!(
            "^{}$",
            pattern.replace('.', r"\.").replace('*', ".*")
        );
        let re = match Regex::new(&regex_pattern) {
            Ok(r) => r,
            Err(_) => return false,
        };

        match std::fs::read_dir(&search_dir) {
            Ok(entries) => entries
                .filter_map(|e| e.ok())
                .any(|e| re.is_match(&e.file_name().to_string_lossy())),
            Err(_) => false,
        }
    } else {
        cwd.join(filename).exists()
    }
}

/// Check package.json for frontend/backend dependencies.
fn check_package_json(cwd: &Path) -> (bool, bool) {
    let package_json_path = cwd.join("package.json");

    if !package_json_path.exists() {
        return (false, false);
    }

    let content = match std::fs::read_to_string(&package_json_path) {
        Ok(c) => c,
        Err(_) => return (false, false),
    };

    let pkg: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return (false, false),
    };

    let mut dep_names: Vec<String> = Vec::new();

    if let Some(deps) = pkg.get("dependencies").and_then(|v| v.as_object()) {
        dep_names.extend(deps.keys().cloned());
    }
    if let Some(deps) = pkg.get("devDependencies").and_then(|v| v.as_object()) {
        dep_names.extend(deps.keys().cloned());
    }

    let has_frontend = FRONTEND_DEPS
        .iter()
        .any(|dep| dep_names.iter().any(|d| d == dep));
    let has_backend = BACKEND_DEPS
        .iter()
        .any(|dep| dep_names.iter().any(|d| d == dep));

    (has_frontend, has_backend)
}

// =============================================================================
// Public API
// =============================================================================

/// Detect project type by analyzing project files.
pub fn detect_project_type(cwd: &Path) -> ProjectType {
    let has_frontend_files = FRONTEND_INDICATORS.iter().any(|f| file_exists(cwd, f));
    let has_backend_files = BACKEND_INDICATORS.iter().any(|f| file_exists(cwd, f));

    let (has_frontend_deps, has_backend_deps) = check_package_json(cwd);

    let is_frontend = has_frontend_files || has_frontend_deps;
    let is_backend = has_backend_files || has_backend_deps;

    if is_frontend && is_backend {
        ProjectType::Fullstack
    } else if is_frontend {
        ProjectType::Frontend
    } else if is_backend {
        ProjectType::Backend
    } else {
        ProjectType::Unknown
    }
}

/// Get a human-readable description of a project type.
pub fn get_project_type_description(type_: ProjectType) -> &'static str {
    match type_ {
        ProjectType::Frontend => "Frontend project (UI/client-side)",
        ProjectType::Backend => "Backend project (server-side/API)",
        ProjectType::Fullstack => "Fullstack project (frontend + backend)",
        ProjectType::Unknown => "Unknown project type (defaults to fullstack)",
    }
}

/// Sanitize a package name for use as a directory name.
/// Strips npm scope prefix (`@scope/`) so `@zhubao/desktop` becomes `desktop`.
pub fn sanitize_pkg_name(name: &str) -> String {
    let re = Regex::new(r"^@[^/]+/").unwrap();
    re.replace(name, "").to_string()
}

// =============================================================================
// Monorepo Detection
// =============================================================================

/// Normalize a package path: strip ./ prefix, trailing /, unify separators.
fn normalize_pkg_path(p: &str) -> String {
    p.replace('\\', "/")
        .trim_start_matches("./")
        .trim_end_matches('/')
        .to_string()
}

/// Check if a relative path is an existing directory.
fn dir_exists(cwd: &Path, rel_path: &str) -> bool {
    let full = cwd.join(rel_path);
    full.is_dir()
}

/// Recursively match glob segments against the filesystem.
/// Handles `*` as a single-level directory wildcard.
fn match_glob_segments(
    cwd: &Path,
    segments: &[&str],
    index: usize,
    current: &str,
) -> Vec<String> {
    if index >= segments.len() {
        return if dir_exists(cwd, current) {
            vec![current.to_string()]
        } else {
            vec![]
        };
    }

    let seg = segments[index];

    if seg != "*" {
        let next = if current.is_empty() {
            seg.to_string()
        } else {
            format!("{}/{}", current, seg)
        };
        return match_glob_segments(cwd, segments, index + 1, &next);
    }

    // Wildcard: match all direct subdirectories.
    let dir = if current.is_empty() {
        cwd.to_path_buf()
    } else {
        cwd.join(current)
    };

    let entries = match std::fs::read_dir(&dir) {
        Ok(e) => e,
        Err(_) => return vec![],
    };

    entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_type().map(|ft| ft.is_dir()).unwrap_or(false)
                && !e
                    .file_name()
                    .to_string_lossy()
                    .starts_with('.')
        })
        .flat_map(|e| {
            let name = e.file_name().to_string_lossy().to_string();
            let next = if current.is_empty() {
                name
            } else {
                format!("{}/{}", current, name)
            };
            match_glob_segments(cwd, segments, index + 1, &next)
        })
        .collect()
}

/// Expand workspace glob patterns (e.g. `packages/*`) into actual directory paths.
/// Supports `!` prefix for exclusion patterns.
fn expand_workspace_globs(cwd: &Path, patterns: &[String]) -> Vec<String> {
    let mut included: Vec<String> = Vec::new();
    let mut excluded: std::collections::HashSet<String> = std::collections::HashSet::new();

    for raw in patterns {
        let is_exclude = raw.starts_with('!');
        let pattern = normalize_pkg_path(if is_exclude { &raw[1..] } else { raw });

        let dirs = if pattern.contains('*') {
            let segments: Vec<&str> = pattern.split('/').collect();
            match_glob_segments(cwd, &segments, 0, "")
        } else if dir_exists(cwd, &pattern) {
            vec![pattern]
        } else {
            vec![]
        };

        for d in dirs {
            if is_exclude {
                excluded.insert(d);
            } else {
                included.push(d);
            }
        }
    }

    included
        .into_iter()
        .filter(|p| !excluded.contains(p))
        .collect()
}

// ---------------------------------------------------------------------------
// Package name reading
// ---------------------------------------------------------------------------

/// Read a package name from various config files in the package directory.
/// Falls back to directory name.
fn read_package_name(cwd: &Path, pkg_path: &str) -> String {
    let abs_path = cwd.join(pkg_path);
    let fallback = || {
        Path::new(pkg_path)
            .file_name()
            .map(|n| n.to_string_lossy().to_string())
            .unwrap_or_else(|| pkg_path.to_string())
    };

    // package.json
    if let Ok(content) = std::fs::read_to_string(abs_path.join("package.json")) {
        if let Ok(pkg) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(name) = pkg.get("name").and_then(|v| v.as_str()) {
                return name.to_string();
            }
        }
    }

    // Cargo.toml [package] name
    if let Ok(content) = std::fs::read_to_string(abs_path.join("Cargo.toml")) {
        let re = Regex::new(r#"\[package\][\s\S]*?name\s*=\s*"([^"]+)""#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return caps[1].to_string();
        }
    }

    // go.mod module name
    if let Ok(content) = std::fs::read_to_string(abs_path.join("go.mod")) {
        let re = Regex::new(r"(?m)^module\s+(\S+)").unwrap();
        if let Some(caps) = re.captures(&content) {
            let module = &caps[1];
            return module
                .rsplit('/')
                .next()
                .unwrap_or(&fallback())
                .to_string();
        }
    }

    // pyproject.toml [project] name
    if let Ok(content) = std::fs::read_to_string(abs_path.join("pyproject.toml")) {
        let re = Regex::new(r#"\[project\][\s\S]*?name\s*=\s*"([^"]+)""#).unwrap();
        if let Some(caps) = re.captures(&content) {
            return caps[1].to_string();
        }
    }

    fallback()
}

// ---------------------------------------------------------------------------
// Workspace format parsers
// ---------------------------------------------------------------------------

/// Parse pnpm-workspace.yaml packages list.
fn parse_pnpm_workspace(cwd: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(cwd.join("pnpm-workspace.yaml")).ok()?;
    let mut patterns: Vec<String> = Vec::new();
    let mut in_packages = false;
    let packages_re = Regex::new(r"^packages\s*:").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();
        if packages_re.is_match(trimmed) {
            in_packages = true;
            continue;
        }
        if in_packages {
            if let Some(rest) = trimmed.strip_prefix("- ") {
                let value = rest.trim().trim_matches('\'').trim_matches('"');
                patterns.push(value.to_string());
            } else if !trimmed.is_empty() && !trimmed.starts_with('#') {
                break;
            }
        }
    }

    if patterns.is_empty() {
        None
    } else {
        Some(patterns)
    }
}

/// Parse package.json workspaces (array or yarn v1 object form).
fn parse_npm_workspaces(cwd: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(cwd.join("package.json")).ok()?;
    let pkg: serde_json::Value = serde_json::from_str(&content).ok()?;
    let ws = pkg.get("workspaces")?;

    if let Some(arr) = ws.as_array() {
        let patterns: Vec<String> = arr.iter().filter_map(|v| v.as_str().map(String::from)).collect();
        if patterns.is_empty() {
            None
        } else {
            Some(patterns)
        }
    } else if let Some(obj) = ws.as_object() {
        if let Some(packages) = obj.get("packages").and_then(|v| v.as_array()) {
            let patterns: Vec<String> = packages
                .iter()
                .filter_map(|v| v.as_str().map(String::from))
                .collect();
            if patterns.is_empty() {
                None
            } else {
                Some(patterns)
            }
        } else {
            None
        }
    } else {
        None
    }
}

/// Parse a TOML inline/multiline array from a specific section.
fn parse_toml_array(content: &str, key: &str, section_header: &str) -> Option<Vec<String>> {
    let section_idx = content.find(section_header)?;
    let after_section = &content[section_idx + section_header.len()..];

    // Find the next section start (a line starting with [ but not [[)
    let re = Regex::new(r"(?m)^\[[^\[]").unwrap();
    let section_content = match re.find(after_section) {
        Some(m) => &after_section[..m.start()],
        None => after_section,
    };

    let key_pattern = Regex::new(&format!(r"{}\s*=\s*\[", regex::escape(key))).unwrap();
    let key_match = key_pattern.find(section_content)?;

    let start_idx = key_match.end();
    let end_idx = section_content[start_idx..].find(']')? + start_idx;

    let array_content = &section_content[start_idx..end_idx];

    let items: Vec<String> = array_content
        .split(&[',', '\n'][..])
        .map(|s| s.trim().trim_matches('"').trim_matches('\'').to_string())
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
        .collect();

    if items.is_empty() {
        None
    } else {
        Some(items)
    }
}

/// Parse Cargo.toml workspace members and exclude.
fn parse_cargo_workspace(cwd: &Path) -> Option<(Vec<String>, Vec<String>)> {
    let content = std::fs::read_to_string(cwd.join("Cargo.toml")).ok()?;

    let re = Regex::new(r"(?m)^\[workspace\]\s*$").unwrap();
    if !re.is_match(&content) {
        return None;
    }

    let members = parse_toml_array(&content, "members", "[workspace]")?;
    let exclude = parse_toml_array(&content, "exclude", "[workspace]").unwrap_or_default();

    Some((members, exclude))
}

/// Parse go.work use directives (block and single-line forms).
fn parse_go_work(cwd: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(cwd.join("go.work")).ok()?;
    let mut paths: Vec<String> = Vec::new();

    // Block form: use ( ... )
    let block_re = Regex::new(r"use\s*\(([\s\S]*?)\)").unwrap();
    if let Some(caps) = block_re.captures(&content) {
        for line in caps[1].lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with("//") {
                paths.push(trimmed.to_string());
            }
        }
    }

    // Single-line form: use ./path
    let single_re = Regex::new(r"(?m)^use\s+(\S+)\s*$").unwrap();
    for caps in single_re.captures_iter(&content) {
        let p = &caps[1];
        if !p.starts_with('(') {
            paths.push(p.to_string());
        }
    }

    if paths.is_empty() {
        None
    } else {
        Some(paths)
    }
}

/// Parse pyproject.toml [tool.uv.workspace] members.
fn parse_uv_workspace(cwd: &Path) -> Option<Vec<String>> {
    let content = std::fs::read_to_string(cwd.join("pyproject.toml")).ok()?;
    if !content.contains("[tool.uv.workspace]") {
        return None;
    }
    parse_toml_array(&content, "members", "[tool.uv.workspace]")
}

/// Parse .gitmodules for submodule names and paths.
fn parse_gitmodules(cwd: &Path) -> Option<Vec<(String, String)>> {
    let content = std::fs::read_to_string(cwd.join(".gitmodules")).ok()?;
    let mut modules: Vec<(String, String)> = Vec::new();
    let mut current_name = String::new();

    let header_re = Regex::new(r#"^\[submodule\s+"([^"]+)"\]"#).unwrap();
    let path_re = Regex::new(r"^path\s*=\s*(.+)").unwrap();

    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(caps) = header_re.captures(trimmed) {
            current_name = caps[1].to_string();
            continue;
        }
        if let Some(caps) = path_re.captures(trimmed) {
            if !current_name.is_empty() {
                modules.push((current_name.clone(), caps[1].trim().to_string()));
            }
        }
    }

    if modules.is_empty() {
        None
    } else {
        Some(modules)
    }
}

// ---------------------------------------------------------------------------
// Main monorepo detection
// ---------------------------------------------------------------------------

/// Detect monorepo workspace configuration and enumerate packages.
///
/// Checks workspace managers in priority order (pnpm -> npm/yarn/bun -> Cargo -> Go -> uv),
/// merges results, and marks git submodules. Returns `None` if no monorepo detected.
pub fn detect_monorepo(cwd: &Path) -> Option<Vec<DetectedPackage>> {
    let mut packages: std::collections::HashMap<String, DetectedPackage> =
        std::collections::HashMap::new();
    let mut detected = false;

    // 1. Parse .gitmodules first to build submodule path set.
    let mut submodule_paths: std::collections::HashMap<String, String> =
        std::collections::HashMap::new();
    if let Some(gitmodules) = parse_gitmodules(cwd) {
        detected = true;
        for (name, path) in gitmodules {
            let np = normalize_pkg_path(&path);
            if !np.is_empty() && np != "." {
                submodule_paths.insert(np, name);
            }
        }
    }

    // 2. Try workspace managers in priority order.
    let workspace_results: Vec<Option<Vec<String>>> = vec![
        // pnpm
        parse_pnpm_workspace(cwd).map(|p| expand_workspace_globs(cwd, &p)),
        // npm/yarn/bun
        parse_npm_workspaces(cwd).map(|p| expand_workspace_globs(cwd, &p)),
        // Cargo
        parse_cargo_workspace(cwd).map(|(members, exclude)| {
            let inc = expand_workspace_globs(cwd, &members);
            let exc: std::collections::HashSet<String> =
                expand_workspace_globs(cwd, &exclude).into_iter().collect();
            inc.into_iter().filter(|p| !exc.contains(p)).collect()
        }),
        // Go
        parse_go_work(cwd).map(|p| {
            p.iter()
                .map(|s| normalize_pkg_path(s))
                .filter(|d| !d.is_empty() && d != "." && dir_exists(cwd, d))
                .collect()
        }),
        // uv
        parse_uv_workspace(cwd).map(|p| expand_workspace_globs(cwd, &p)),
    ];

    for dirs_opt in workspace_results {
        let dirs = match dirs_opt {
            Some(d) => d,
            None => continue,
        };
        detected = true;
        for dir in dirs {
            let np = normalize_pkg_path(&dir);
            if np.is_empty() || np == "." {
                continue;
            }
            if packages.contains_key(&np) {
                continue;
            }

            let type_ = if dir_exists(cwd, &np) {
                detect_project_type(&cwd.join(&np))
            } else {
                ProjectType::Unknown
            };

            packages.insert(
                np.clone(),
                DetectedPackage {
                    name: read_package_name(cwd, &np),
                    path: np.clone(),
                    type_,
                    is_submodule: submodule_paths.contains_key(&np),
                },
            );
        }
    }

    // 3. Add submodule-only packages not already covered by workspace managers.
    for (np, name) in &submodule_paths {
        if packages.contains_key(np) {
            continue;
        }
        let type_ = if dir_exists(cwd, np) {
            detect_project_type(&cwd.join(np))
        } else {
            ProjectType::Unknown
        };

        packages.insert(
            np.clone(),
            DetectedPackage {
                name: name.clone(),
                path: np.clone(),
                type_,
                is_submodule: true,
            },
        );
    }

    if !detected {
        return None;
    }

    Some(packages.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ---------------------------------------------------------------
    // detect_project_type
    // ---------------------------------------------------------------

    #[test]
    fn test_unknown_empty_dir() {
        let tmp = tempfile::tempdir().unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Unknown);
    }

    #[test]
    fn test_frontend_vite_config() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("vite.config.ts"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Frontend);
    }

    #[test]
    fn test_frontend_next_config() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("next.config.js"), "").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Frontend);
    }

    #[test]
    fn test_frontend_react_dep() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"dependencies":{"react":"^18"}}"#,
        )
        .unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Frontend);
    }

    #[test]
    fn test_frontend_vue_devdep() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"devDependencies":{"vue":"^3"}}"#,
        )
        .unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Frontend);
    }

    #[test]
    fn test_backend_go_mod() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module example.com/foo").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Backend);
    }

    #[test]
    fn test_backend_cargo_toml() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[package]\nname = \"test\"",
        )
        .unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Backend);
    }

    #[test]
    fn test_backend_requirements() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("requirements.txt"), "flask==2.0").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Backend);
    }

    #[test]
    fn test_backend_pyproject() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("pyproject.toml"),
            "[project]\nname = \"app\"",
        )
        .unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Backend);
    }

    #[test]
    fn test_fullstack_express() {
        // package.json with express (a backend dep) plus package.json itself is
        // a frontend indicator, so Frontend+Backend = Fullstack.
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"dependencies":{"express":"^4"}}"#,
        )
        .unwrap();
        // package.json is in FRONTEND_INDICATORS so has_frontend_files = true,
        // and express is a backend dep, so result is Fullstack.
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Fullstack);
    }

    #[test]
    fn test_fullstack_both_indicators() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(tmp.path().join("vite.config.ts"), "").unwrap();
        std::fs::write(tmp.path().join("go.mod"), "module x").unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Fullstack);
    }

    #[test]
    fn test_fullstack_react_express() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"dependencies":{"react":"^18","express":"^4"}}"#,
        )
        .unwrap();
        assert_eq!(detect_project_type(tmp.path()), ProjectType::Fullstack);
    }

    // ---------------------------------------------------------------
    // get_project_type_description
    // ---------------------------------------------------------------

    #[test]
    fn test_description_frontend() {
        let desc = get_project_type_description(ProjectType::Frontend);
        assert!(
            desc.contains("Frontend"),
            "Expected 'Frontend' in: {}",
            desc
        );
    }

    #[test]
    fn test_description_backend() {
        let desc = get_project_type_description(ProjectType::Backend);
        assert!(
            desc.contains("Backend"),
            "Expected 'Backend' in: {}",
            desc
        );
    }

    #[test]
    fn test_description_fullstack() {
        let desc = get_project_type_description(ProjectType::Fullstack);
        assert!(
            desc.contains("Fullstack"),
            "Expected 'Fullstack' in: {}",
            desc
        );
    }

    #[test]
    fn test_description_unknown() {
        let desc = get_project_type_description(ProjectType::Unknown);
        assert!(
            desc.contains("Unknown"),
            "Expected 'Unknown' in: {}",
            desc
        );
    }

    // ---------------------------------------------------------------
    // sanitize_pkg_name
    // ---------------------------------------------------------------

    #[test]
    fn test_sanitize_scoped() {
        assert_eq!(sanitize_pkg_name("@scope/name"), "name");
    }

    #[test]
    fn test_sanitize_unscoped() {
        assert_eq!(sanitize_pkg_name("name"), "name");
    }

    #[test]
    fn test_sanitize_first_scope_only() {
        // Only the first @scope/ prefix is stripped.
        assert_eq!(sanitize_pkg_name("@a/@b/c"), "@b/c");
    }

    // ---------------------------------------------------------------
    // detect_monorepo
    // ---------------------------------------------------------------

    #[test]
    fn test_monorepo_null_empty() {
        let tmp = tempfile::tempdir().unwrap();
        assert!(detect_monorepo(tmp.path()).is_none());
    }

    #[test]
    fn test_monorepo_null_no_workspaces() {
        let tmp = tempfile::tempdir().unwrap();
        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"name":"no-ws"}"#,
        )
        .unwrap();
        assert!(detect_monorepo(tmp.path()).is_none());
    }

    #[test]
    fn test_monorepo_pnpm() {
        let tmp = tempfile::tempdir().unwrap();
        // Create actual package directories.
        let pkg_a = tmp.path().join("packages").join("a");
        std::fs::create_dir_all(&pkg_a).unwrap();
        std::fs::write(
            pkg_a.join("package.json"),
            r#"{"name":"pkg-a"}"#,
        )
        .unwrap();

        std::fs::write(
            tmp.path().join("pnpm-workspace.yaml"),
            "packages:\n  - 'packages/*'\n",
        )
        .unwrap();

        let result = detect_monorepo(tmp.path());
        assert!(result.is_some());
        let pkgs = result.unwrap();
        assert!(!pkgs.is_empty());
        assert!(pkgs.iter().any(|p| p.name == "pkg-a"));
    }

    #[test]
    fn test_monorepo_npm() {
        let tmp = tempfile::tempdir().unwrap();
        let pkg_a = tmp.path().join("packages").join("a");
        std::fs::create_dir_all(&pkg_a).unwrap();
        std::fs::write(
            pkg_a.join("package.json"),
            r#"{"name":"npm-a"}"#,
        )
        .unwrap();

        std::fs::write(
            tmp.path().join("package.json"),
            r#"{"workspaces":["packages/*"]}"#,
        )
        .unwrap();

        let result = detect_monorepo(tmp.path());
        assert!(result.is_some());
        let pkgs = result.unwrap();
        assert!(pkgs.iter().any(|p| p.name == "npm-a"));
    }

    #[test]
    fn test_monorepo_cargo() {
        let tmp = tempfile::tempdir().unwrap();
        let member = tmp.path().join("crates").join("core");
        std::fs::create_dir_all(&member).unwrap();
        std::fs::write(
            member.join("Cargo.toml"),
            "[package]\nname = \"core-crate\"",
        )
        .unwrap();

        std::fs::write(
            tmp.path().join("Cargo.toml"),
            "[workspace]\nmembers = [\"crates/*\"]\n",
        )
        .unwrap();

        let result = detect_monorepo(tmp.path());
        assert!(result.is_some());
        let pkgs = result.unwrap();
        assert!(pkgs.iter().any(|p| p.name == "core-crate"));
    }

    #[test]
    fn test_monorepo_go_work() {
        let tmp = tempfile::tempdir().unwrap();
        let svc = tmp.path().join("svc");
        std::fs::create_dir_all(&svc).unwrap();
        std::fs::write(svc.join("go.mod"), "module example.com/svc").unwrap();

        std::fs::write(
            tmp.path().join("go.work"),
            "go 1.21\n\nuse (\n    ./svc\n)\n",
        )
        .unwrap();

        let result = detect_monorepo(tmp.path());
        assert!(result.is_some());
        let pkgs = result.unwrap();
        assert!(pkgs.iter().any(|p| p.path == "svc"));
    }

    #[test]
    fn test_monorepo_gitmodules() {
        let tmp = tempfile::tempdir().unwrap();
        let sub = tmp.path().join("libs").join("shared");
        std::fs::create_dir_all(&sub).unwrap();

        std::fs::write(
            tmp.path().join(".gitmodules"),
            "[submodule \"shared\"]\n\tpath = libs/shared\n\turl = https://example.com/shared.git\n",
        )
        .unwrap();

        let result = detect_monorepo(tmp.path());
        assert!(result.is_some());
        let pkgs = result.unwrap();
        let submod = pkgs.iter().find(|p| p.path == "libs/shared");
        assert!(submod.is_some(), "Should detect submodule package");
        assert!(submod.unwrap().is_submodule);
    }
}
