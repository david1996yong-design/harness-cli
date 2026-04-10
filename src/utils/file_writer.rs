use std::path::Path;
use std::sync::atomic::{AtomicU8, Ordering as AtomicOrdering};

use anyhow::Result;
use colored::Colorize;
use dialoguer::Select;

// ---------------------------------------------------------------------------
// WriteMode
// ---------------------------------------------------------------------------

/// How to handle file conflicts when writing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum WriteMode {
    Ask = 0,
    Force = 1,
    Skip = 2,
    Append = 3,
}

impl WriteMode {
    fn from_u8(v: u8) -> Self {
        match v {
            1 => Self::Force,
            2 => Self::Skip,
            3 => Self::Append,
            _ => Self::Ask,
        }
    }
}

/// Global write mode stored as an atomic for thread safety.
static WRITE_MODE: AtomicU8 = AtomicU8::new(0); // 0 = Ask

/// Set the global write mode.
pub fn set_write_mode(mode: WriteMode) {
    WRITE_MODE.store(mode as u8, AtomicOrdering::SeqCst);
}

/// Get the current global write mode.
pub fn get_write_mode() -> WriteMode {
    WriteMode::from_u8(WRITE_MODE.load(AtomicOrdering::SeqCst))
}

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Get a display path relative to the current working directory.
fn get_relative_path(file_path: &Path) -> String {
    if let Ok(cwd) = std::env::current_dir() {
        if let Ok(rel) = file_path.strip_prefix(&cwd) {
            let s = rel.to_string_lossy().to_string();
            if s.is_empty() {
                return file_path
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_default();
            }
            return s;
        }
    }
    file_path.to_string_lossy().to_string()
}

/// Append content to an existing file.
fn append_to_file(file_path: &Path, content: &str, executable: bool) -> Result<()> {
    let existing = std::fs::read_to_string(file_path)?;
    let new_content = if existing.ends_with('\n') {
        format!("{}{}", existing, content)
    } else {
        format!("{}\n{}", existing, content)
    };
    std::fs::write(file_path, new_content)?;
    if executable {
        set_executable(file_path);
    }
    Ok(())
}

/// Mark a file as executable on Unix platforms.
#[cfg(unix)]
fn set_executable(file_path: &Path) {
    use std::os::unix::fs::PermissionsExt;
    if let Ok(metadata) = std::fs::metadata(file_path) {
        let mut perms = metadata.permissions();
        perms.set_mode(0o755);
        let _ = std::fs::set_permissions(file_path, perms);
    }
}

#[cfg(not(unix))]
fn set_executable(_file_path: &Path) {
    // No-op on non-Unix platforms.
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Ensure a directory (and its parents) exists.
pub fn ensure_dir(dir_path: &Path) -> Result<()> {
    std::fs::create_dir_all(dir_path)?;
    Ok(())
}

/// Write a file with conflict handling.
///
/// - If the file doesn't exist: write directly, return `Ok(true)`.
/// - If the file exists with identical content: return `Ok(false)` silently.
/// - If the file exists with different content: behave according to the current
///   global [`WriteMode`].
///
/// When `executable` is true the file is given 0755 permissions (Unix only).
pub fn write_file(file_path: &Path, content: &str, executable: bool) -> Result<bool> {
    let display_path = get_relative_path(file_path);

    if !file_path.exists() {
        // File doesn't exist, write directly.
        std::fs::write(file_path, content)?;
        if executable {
            set_executable(file_path);
        }
        return Ok(true);
    }

    // File exists -- check if content is identical.
    let existing_content = std::fs::read_to_string(file_path)?;
    if existing_content == content {
        return Ok(false);
    }

    // File exists with different content -- handle based on mode.
    let mode = get_write_mode();

    match mode {
        WriteMode::Force => {
            std::fs::write(file_path, content)?;
            if executable {
                set_executable(file_path);
            }
            println!(
                "{}",
                format!("  \u{21bb} Overwritten: {}", display_path).yellow()
            );
            Ok(true)
        }
        WriteMode::Skip => {
            println!(
                "{}",
                format!("  \u{25cb} Skipped: {} (already exists)", display_path).dimmed()
            );
            Ok(false)
        }
        WriteMode::Append => {
            append_to_file(file_path, content, executable)?;
            println!("{}", format!("  + Appended: {}", display_path).blue());
            Ok(true)
        }
        WriteMode::Ask => {
            let choices = &[
                "Skip (keep existing)",
                "Overwrite",
                "Append to end",
                "Skip all remaining conflicts",
                "Overwrite all remaining conflicts",
                "Append all remaining conflicts",
            ];

            let selection = Select::new()
                .with_prompt(format!(
                    "File \"{}\" already exists. What would you like to do?",
                    display_path
                ))
                .items(choices)
                .default(0)
                .interact()?;

            match selection {
                0 => {
                    // Skip
                    println!(
                        "{}",
                        format!("  \u{25cb} Skipped: {}", display_path).dimmed()
                    );
                    Ok(false)
                }
                1 => {
                    // Overwrite
                    std::fs::write(file_path, content)?;
                    if executable {
                        set_executable(file_path);
                    }
                    println!(
                        "{}",
                        format!("  \u{21bb} Overwritten: {}", display_path).yellow()
                    );
                    Ok(true)
                }
                2 => {
                    // Append
                    append_to_file(file_path, content, executable)?;
                    println!("{}", format!("  + Appended: {}", display_path).blue());
                    Ok(true)
                }
                3 => {
                    // Skip all
                    set_write_mode(WriteMode::Skip);
                    println!(
                        "{}",
                        format!("  \u{25cb} Skipped: {}", display_path).dimmed()
                    );
                    Ok(false)
                }
                4 => {
                    // Overwrite all
                    set_write_mode(WriteMode::Force);
                    std::fs::write(file_path, content)?;
                    if executable {
                        set_executable(file_path);
                    }
                    println!(
                        "{}",
                        format!("  \u{21bb} Overwritten: {}", display_path).yellow()
                    );
                    Ok(true)
                }
                5 => {
                    // Append all
                    set_write_mode(WriteMode::Append);
                    append_to_file(file_path, content, executable)?;
                    println!("{}", format!("  + Appended: {}", display_path).blue());
                    Ok(true)
                }
                _ => Ok(false),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Mutex;

    // Tests that touch the global WRITE_MODE must not run in parallel.
    static MODE_LOCK: Mutex<()> = Mutex::new(());

    fn reset_mode() {
        WRITE_MODE.store(0, AtomicOrdering::SeqCst); // Ask
    }

    #[test]
    fn test_default_mode_is_ask() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        assert_eq!(get_write_mode(), WriteMode::Ask);
    }

    #[test]
    fn test_set_mode_force() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Force);
        assert_eq!(get_write_mode(), WriteMode::Force);
        reset_mode();
    }

    #[test]
    fn test_set_mode_skip() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Skip);
        assert_eq!(get_write_mode(), WriteMode::Skip);
        reset_mode();
    }

    #[test]
    fn test_set_mode_append() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Append);
        assert_eq!(get_write_mode(), WriteMode::Append);
        reset_mode();
    }

    #[test]
    fn test_ensure_dir_creates_new() {
        let tmp = tempfile::tempdir().unwrap();
        let new_dir = tmp.path().join("brand_new");
        assert!(!new_dir.exists());
        ensure_dir(&new_dir).unwrap();
        assert!(new_dir.is_dir());
    }

    #[test]
    fn test_ensure_dir_nested() {
        let tmp = tempfile::tempdir().unwrap();
        let nested = tmp.path().join("a").join("b").join("c");
        ensure_dir(&nested).unwrap();
        assert!(nested.is_dir());
    }

    #[test]
    fn test_ensure_dir_existing() {
        let tmp = tempfile::tempdir().unwrap();
        let dir = tmp.path().join("existing");
        std::fs::create_dir(&dir).unwrap();
        // Should not error on existing directory.
        ensure_dir(&dir).unwrap();
        assert!(dir.is_dir());
    }

    #[test]
    fn test_write_new_file() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("new.txt");
        let result = write_file(&file, "hello", false).unwrap();
        assert!(result);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "hello");
        reset_mode();
    }

    #[test]
    fn test_write_identical_content_returns_false() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("same.txt");
        std::fs::write(&file, "content").unwrap();
        let result = write_file(&file, "content", false).unwrap();
        assert!(!result);
        reset_mode();
    }

    #[test]
    fn test_write_force_mode_overwrites() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Force);
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("force.txt");
        std::fs::write(&file, "old content").unwrap();
        let result = write_file(&file, "new content", false).unwrap();
        assert!(result);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "new content");
        reset_mode();
    }

    #[test]
    fn test_write_skip_mode_preserves() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Skip);
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("skip.txt");
        std::fs::write(&file, "original").unwrap();
        let result = write_file(&file, "different", false).unwrap();
        assert!(!result);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "original");
        reset_mode();
    }

    #[test]
    fn test_write_append_mode() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Append);
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("append.txt");
        std::fs::write(&file, "line1\n").unwrap();
        let result = write_file(&file, "line2\n", false).unwrap();
        assert!(result);
        assert_eq!(std::fs::read_to_string(&file).unwrap(), "line1\nline2\n");
        reset_mode();
    }

    #[test]
    fn test_write_append_adds_newline() {
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        set_write_mode(WriteMode::Append);
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("append_nl.txt");
        // File content does NOT end with newline.
        std::fs::write(&file, "line1").unwrap();
        let result = write_file(&file, "line2", false).unwrap();
        assert!(result);
        let content = std::fs::read_to_string(&file).unwrap();
        assert_eq!(content, "line1\nline2");
        reset_mode();
    }

    #[cfg(unix)]
    #[test]
    fn test_write_executable() {
        use std::os::unix::fs::PermissionsExt;
        let _g = MODE_LOCK.lock().unwrap();
        reset_mode();
        let tmp = tempfile::tempdir().unwrap();
        let file = tmp.path().join("script.sh");
        write_file(&file, "#!/bin/sh\necho hi", true).unwrap();
        let mode = std::fs::metadata(&file).unwrap().permissions().mode();
        // Check that execute bit is set (at least user execute).
        assert_ne!(mode & 0o111, 0, "Execute bits should be set");
        reset_mode();
    }
}
