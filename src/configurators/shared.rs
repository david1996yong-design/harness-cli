//! Shared utilities for platform configurators.

/// Get the Python command based on platform.
///
/// Windows uses `python`, macOS/Linux use `python3`.
fn get_python_command() -> &'static str {
    if cfg!(windows) {
        "python"
    } else {
        "python3"
    }
}

/// Resolve platform-specific placeholders in template content.
///
/// Currently handles `{{PYTHON_CMD}}`.
pub fn resolve_placeholders(content: &str) -> String {
    content.replace("{{PYTHON_CMD}}", get_python_command())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_placeholders_python_cmd() {
        let result = resolve_placeholders("{{PYTHON_CMD}}");
        if cfg!(windows) {
            assert_eq!(result, "python");
        } else {
            assert_eq!(result, "python3");
        }
    }

    #[test]
    fn test_resolve_placeholders_no_match() {
        let input = "hello world";
        let result = resolve_placeholders(input);
        assert_eq!(result, "hello world");
    }

    #[test]
    fn test_resolve_placeholders_multiple() {
        let input = "run {{PYTHON_CMD}} script.py && {{PYTHON_CMD}} other.py";
        let result = resolve_placeholders(input);
        let expected_cmd = if cfg!(windows) { "python" } else { "python3" };
        let expected = format!("run {} script.py && {} other.py", expected_cmd, expected_cmd);
        assert_eq!(result, expected);
    }
}
