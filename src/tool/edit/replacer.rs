use super::levenshtein::levenshtein;

pub trait Replacer: Send + Sync {
    fn find(&self, content: &str, pattern: &str) -> Vec<String>;
}

pub fn get_replacers() -> Vec<Box<dyn Replacer>> {
    vec![
        Box::new(SimpleReplacer),
        Box::new(LineTrimmedReplacer),
        Box::new(BlockAnchorReplacer),
        Box::new(WhitespaceNormalizedReplacer),
        Box::new(IndentationFlexibleReplacer),
        Box::new(EscapeNormalizedReplacer),
        Box::new(TrimmedBoundaryReplacer),
        Box::new(ContextAwareReplacer),
        Box::new(MultiOccurrenceReplacer),
    ]
}

pub struct SimpleReplacer;
impl Replacer for SimpleReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        if content.contains(pattern) {
            vec![pattern.to_string()]
        } else {
            vec![]
        }
    }
}

pub struct LineTrimmedReplacer;
impl Replacer for LineTrimmedReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let original_lines: Vec<&str> = content.split('\n').collect();
        let mut search_lines: Vec<&str> = pattern.split('\n').collect();

        if search_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            search_lines.pop();
        }

        let mut matches = Vec::new();

        for i in 0..=original_lines.len().saturating_sub(search_lines.len()) {
            let mut matches_block = true;

            for j in 0..search_lines.len() {
                if original_lines[i + j].trim() != search_lines[j].trim() {
                    matches_block = false;
                    break;
                }
            }

            if matches_block {
                let match_start = original_lines[..i]
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>();
                let match_end = match_start
                    + original_lines[i..i + search_lines.len()]
                        .iter()
                        .map(|l| l.len() + 1)
                        .sum::<usize>()
                    - 1;
                matches.push(content[match_start..match_end].to_string());
            }
        }

        matches
    }
}

pub struct BlockAnchorReplacer;
impl Replacer for BlockAnchorReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let original_lines: Vec<&str> = content.split('\n').collect();
        let mut search_lines: Vec<&str> = pattern.split('\n').collect();

        if search_lines.len() < 3 {
            return vec![];
        }

        if search_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            search_lines.pop();
        }

        let first_line = search_lines[0].trim();
        let last_line = search_lines[search_lines.len() - 1].trim();

        let mut candidates = Vec::new();

        for i in 0..original_lines.len() {
            if original_lines[i].trim() != first_line {
                continue;
            }

            for j in (i + 2)..original_lines.len() {
                if original_lines[j].trim() == last_line {
                    candidates.push((i, j));
                    break;
                }
            }
        }

        if candidates.is_empty() {
            return vec![];
        }

        let threshold = if candidates.len() == 1 { 0.0 } else { 0.3 };

        let mut best_match: Option<(usize, usize, f64)> = None;

        for (start, end) in candidates {
            let actual_block_size = end - start + 1;
            let lines_to_check = std::cmp::min(search_lines.len() - 2, actual_block_size - 2);

            let similarity = if lines_to_check > 0 {
                let mut sim = 0.0;
                for j in 1..search_lines.len() - 1 {
                    if start + j >= original_lines.len() {
                        break;
                    }
                    let original_line = original_lines[start + j].trim();
                    let search_line = search_lines[j].trim();
                    let max_len = std::cmp::max(original_line.len(), search_line.len()) as f64;
                    if max_len > 0.0 {
                        let dist = levenshtein(original_line, search_line) as f64;
                        sim += 1.0 - dist / max_len;
                    }
                }
                sim / lines_to_check as f64
            } else {
                1.0
            };

            if similarity >= threshold {
                if best_match.map(|(_, _, s)| similarity > s).unwrap_or(true) {
                    best_match = Some((start, end, similarity));
                }
            }
        }

        if let Some((start, end, _)) = best_match {
            let match_start: usize = original_lines[..start].iter().map(|l| l.len() + 1).sum();
            let match_end: usize = match_start
                + original_lines[start..=end]
                    .iter()
                    .map(|l| l.len() + 1)
                    .sum::<usize>()
                - 1;
            return vec![content[match_start..match_end].to_string()];
        }

        vec![]
    }
}

pub struct WhitespaceNormalizedReplacer;
impl Replacer for WhitespaceNormalizedReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let normalize = |s: &str| s.split_whitespace().collect::<Vec<_>>().join(" ");
        let normalized_pattern = normalize(pattern);

        let mut matches = Vec::new();
        let lines: Vec<&str> = content.split('\n').collect();
        let pattern_lines: Vec<&str> = pattern.split('\n').collect();

        for line in &lines {
            if normalize(line) == normalized_pattern {
                matches.push(line.to_string());
            }
        }

        if pattern_lines.len() > 1 {
            for i in 0..=lines.len().saturating_sub(pattern_lines.len()) {
                let block = lines[i..i + pattern_lines.len()].join("\n");
                if normalize(&block) == normalized_pattern {
                    matches.push(block);
                }
            }
        }

        matches
    }
}

pub struct IndentationFlexibleReplacer;
impl Replacer for IndentationFlexibleReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let remove_indent = |s: &str| {
            let lines: Vec<&str> = s.split('\n').collect();
            let non_empty: Vec<&str> = lines
                .iter()
                .filter(|l| !l.trim().is_empty())
                .copied()
                .collect();
            if non_empty.is_empty() {
                return s.to_string();
            }
            let min_indent = non_empty
                .iter()
                .map(|l| l.chars().take_while(|c| c.is_whitespace()).count())
                .min()
                .unwrap_or(0);
            lines
                .iter()
                .map(|l| {
                    if l.trim().is_empty() {
                        l
                    } else {
                        &l[min_indent..]
                    }
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let normalized_pattern = remove_indent(pattern);
        let content_lines: Vec<&str> = content.split('\n').collect();
        let pattern_lines: Vec<&str> = pattern.split('\n').collect();

        let mut matches = Vec::new();

        for i in 0..=content_lines.len().saturating_sub(pattern_lines.len()) {
            let block = content_lines[i..i + pattern_lines.len()].join("\n");
            if remove_indent(&block) == normalized_pattern {
                matches.push(block);
            }
        }

        matches
    }
}

pub struct EscapeNormalizedReplacer;
impl Replacer for EscapeNormalizedReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let unescape = |s: &str| {
            s.replace("\\n", "\n")
                .replace("\\t", "\t")
                .replace("\\r", "\r")
                .replace("\\'", "'")
                .replace("\\\"", "\"")
                .replace("\\`", "`")
                .replace("\\\\", "\\")
        };

        let unescaped_pattern = unescape(pattern);

        let mut matches = Vec::new();

        if content.contains(&unescaped_pattern) {
            matches.push(unescaped_pattern.clone());
        }

        let lines: Vec<&str> = content.split('\n').collect();
        let pattern_lines: Vec<&str> = unescaped_pattern.split('\n').collect();

        for i in 0..=lines.len().saturating_sub(pattern_lines.len()) {
            let block = lines[i..i + pattern_lines.len()].join("\n");
            if unescape(&block) == unescaped_pattern {
                matches.push(block);
            }
        }

        matches
    }
}

pub struct TrimmedBoundaryReplacer;
impl Replacer for TrimmedBoundaryReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let trimmed_pattern = pattern.trim();

        if trimmed_pattern == pattern {
            return vec![];
        }

        let mut matches = Vec::new();

        if content.contains(trimmed_pattern) {
            matches.push(trimmed_pattern.to_string());
        }

        let lines: Vec<&str> = content.split('\n').collect();
        let pattern_lines: Vec<&str> = pattern.split('\n').collect();

        for i in 0..=lines.len().saturating_sub(pattern_lines.len()) {
            let block = lines[i..i + pattern_lines.len()].join("\n");
            if block.trim() == trimmed_pattern {
                matches.push(block);
            }
        }

        matches
    }
}

pub struct ContextAwareReplacer;
impl Replacer for ContextAwareReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let mut pattern_lines: Vec<&str> = pattern.split('\n').collect();

        if pattern_lines.len() < 3 {
            return vec![];
        }

        if pattern_lines.last().map(|l| l.is_empty()).unwrap_or(false) {
            pattern_lines.pop();
        }

        let content_lines: Vec<&str> = content.split('\n').collect();
        let first_line = pattern_lines[0].trim();
        let last_line = pattern_lines[pattern_lines.len() - 1].trim();

        for i in 0..content_lines.len() {
            if content_lines[i].trim() != first_line {
                continue;
            }

            for j in (i + 2)..content_lines.len() {
                if content_lines[j].trim() != last_line {
                    continue;
                }

                if j - i + 1 != pattern_lines.len() {
                    continue;
                }

                let block_lines = &content_lines[i..=j];
                let mut matching_lines = 0;
                let mut total_non_empty = 0;

                for k in 1..block_lines.len() - 1 {
                    let block_line = block_lines[k].trim();
                    let search_line = pattern_lines[k].trim();

                    if !block_line.is_empty() || !search_line.is_empty() {
                        total_non_empty += 1;
                        if block_line == search_line {
                            matching_lines += 1;
                        }
                    }
                }

                if total_non_empty == 0 || matching_lines as f64 / total_non_empty as f64 >= 0.5 {
                    let match_start: usize = content_lines[..i].iter().map(|l| l.len() + 1).sum();
                    let match_end: usize = match_start
                        + content_lines[i..=j]
                            .iter()
                            .map(|l| l.len() + 1)
                            .sum::<usize>()
                        - 1;
                    return vec![content[match_start..match_end].to_string()];
                }

                break;
            }
        }

        vec![]
    }
}

pub struct MultiOccurrenceReplacer;
impl Replacer for MultiOccurrenceReplacer {
    fn find(&self, content: &str, pattern: &str) -> Vec<String> {
        let mut matches = Vec::new();
        let mut start = 0;

        while let Some(idx) = content[start..].find(pattern) {
            matches.push(pattern.to_string());
            start += idx + pattern.len();
        }

        matches
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_replacer() {
        let replacer = SimpleReplacer;
        let content = "hello world";

        let matches = replacer.find(content, "world");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0], "world");
    }

    #[test]
    fn test_simple_replacer_no_match() {
        let replacer = SimpleReplacer;
        let content = "hello world";

        let matches = replacer.find(content, "foo");
        assert_eq!(matches.len(), 0);
    }

    #[test]
    fn test_multi_occurrence_replacer() {
        let replacer = MultiOccurrenceReplacer;
        let content = "hello world hello";

        let matches = replacer.find(content, "hello");
        assert_eq!(matches.len(), 2);
    }

    #[test]
    fn test_line_trimmed_replacer() {
        let replacer = LineTrimmedReplacer;
        let content = "  hello world  \n  foo bar  ";

        let matches = replacer.find(content, "hello world\nfoo bar");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_whitespace_normalized_replacer() {
        let replacer = WhitespaceNormalizedReplacer;
        let content = "hello    world";

        let matches = replacer.find(content, "hello world");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_escape_normalized_replacer() {
        let replacer = EscapeNormalizedReplacer;
        let content = "hello\nworld";

        let matches = replacer.find(content, "hello\\nworld");
        assert!(matches.len() >= 1);
    }

    #[test]
    fn test_indentation_flexible_replacer() {
        let replacer = IndentationFlexibleReplacer;
        let content = "    hello\n    world";

        let matches = replacer.find(content, "hello\nworld");
        assert_eq!(matches.len(), 1);
    }

    #[test]
    fn test_trimmed_boundary_replacer() {
        let replacer = TrimmedBoundaryReplacer;
        let content = "  hello world  ";

        let matches = replacer.find(content, "  hello world  ");
        assert!(matches.len() >= 1);
    }

    #[test]
    fn test_get_replacers_count() {
        let replacers = get_replacers();
        assert_eq!(replacers.len(), 9);
    }
}
