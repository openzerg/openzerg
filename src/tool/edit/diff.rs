pub fn create_diff(filename: &str, old: &str, new: &str) -> String {
    let old_lines: Vec<&str> = old.lines().collect();
    let new_lines: Vec<&str> = new.lines().collect();

    let mut result = format!("--- {}\n+++ {}\n", filename, filename);

    let mut old_idx = 0;
    let mut new_idx = 0;

    while old_idx < old_lines.len() || new_idx < new_lines.len() {
        if old_idx >= old_lines.len() {
            result.push_str(&format!("+{}\n", new_lines[new_idx]));
            new_idx += 1;
        } else if new_idx >= new_lines.len() {
            result.push_str(&format!("-{}\n", old_lines[old_idx]));
            old_idx += 1;
        } else if old_lines[old_idx] == new_lines[new_idx] {
            result.push_str(&format!(" {}\n", old_lines[old_idx]));
            old_idx += 1;
            new_idx += 1;
        } else {
            result.push_str(&format!("-{}\n", old_lines[old_idx]));
            result.push_str(&format!("+{}\n", new_lines[new_idx]));
            old_idx += 1;
            new_idx += 1;
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_addition() {
        let old = "hello";
        let new = "hello\nworld";
        let diff = create_diff("test.txt", old, new);

        assert!(diff.contains(" hello"));
        assert!(diff.contains("+world"));
    }

    #[test]
    fn test_removal() {
        let old = "hello\nworld";
        let new = "hello";
        let diff = create_diff("test.txt", old, new);

        assert!(diff.contains(" hello"));
        assert!(diff.contains("-world"));
    }

    #[test]
    fn test_modification() {
        let old = "hello";
        let new = "world";
        let diff = create_diff("test.txt", old, new);

        assert!(diff.contains("-hello"));
        assert!(diff.contains("+world"));
    }
}
