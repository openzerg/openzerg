pub const MAX_BYTES: usize = 50 * 1024;
pub const MAX_LINE_LENGTH: usize = 2000;
pub const MAX_METADATA_LENGTH: usize = 30_000;

pub fn truncate_output(output: &str, max_bytes: usize) -> (String, bool) {
    let bytes = output.as_bytes();

    if bytes.len() <= max_bytes {
        return (output.to_string(), false);
    }

    let mut truncated = String::new();
    let mut current_bytes = 0;

    for line in output.lines() {
        let line_bytes = line.len() + 1;
        if current_bytes + line_bytes > max_bytes {
            break;
        }
        if !truncated.is_empty() {
            truncated.push('\n');
        }
        truncated.push_str(line);
        current_bytes += line_bytes;
    }

    truncated.push_str(&format!(
        "\n\n... (output truncated at {} bytes)",
        max_bytes
    ));
    (truncated, true)
}

pub fn truncate_lines(output: &str, max_lines: usize, max_bytes: usize) -> (String, bool) {
    let lines: Vec<&str> = output.lines().collect();
    let total_lines = lines.len();

    if total_lines <= max_lines {
        return truncate_output(output, max_bytes);
    }

    let truncated_lines: Vec<&str> = lines.into_iter().take(max_lines).collect();
    let mut result = truncated_lines.join("\n");

    let total_bytes = result.as_bytes().len();
    if total_bytes > max_bytes {
        result = result.as_bytes()[..max_bytes]
            .iter()
            .map(|&c| c as char)
            .collect::<String>();
    }

    result.push_str(&format!(
        "\n\n... (showing {} of {} lines, truncated at {} bytes)",
        max_lines, total_lines, max_bytes
    ));

    (result, true)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_no_truncation_needed() {
        let output = "hello world";
        let (result, truncated) = truncate_output(output, 100);

        assert_eq!(result, "hello world");
        assert!(!truncated);
    }

    #[test]
    fn test_truncation_applied() {
        let output = &"a".repeat(1000);
        let (result, truncated) = truncate_output(output, 100);

        assert!(truncated);
        assert!(result.contains("truncated"));
        assert!(result.len() < output.len());
    }

    #[test]
    fn test_truncation_preserves_lines() {
        let output = "line1\nline2\nline3\nline4\nline5";
        let (result, truncated) = truncate_output(output, 20);

        assert!(truncated);
        assert!(result.contains("line1"));
    }

    #[test]
    fn test_truncate_lines_no_truncation() {
        let output = "line1\nline2\nline3";
        let (result, truncated) = truncate_lines(output, 10, 1000);

        assert_eq!(result, "line1\nline2\nline3");
        assert!(!truncated);
    }

    #[test]
    fn test_truncate_lines_applied() {
        let output = "line1\nline2\nline3\nline4\nline5";
        let (result, truncated) = truncate_lines(output, 2, 1000);

        assert!(truncated);
        assert!(result.contains("line1"));
        assert!(result.contains("line2"));
        assert!(!result.contains("line3"));
        assert!(result.contains("2 of 5 lines"));
    }

    #[test]
    fn test_empty_output() {
        let output = "";
        let (result, truncated) = truncate_output(output, 100);

        assert_eq!(result, "");
        assert!(!truncated);
    }

    #[test]
    fn test_single_long_line() {
        let output = &"a".repeat(2000);
        let (result, truncated) = truncate_output(output, 100);

        assert!(truncated);
    }
}
