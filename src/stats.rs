//! Text statistics for the status bar: words, characters and paragraphs.

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Stats {
    pub words: usize,
    /// Characters without line breaks.
    pub chars: usize,
    /// Blocks of non-blank lines separated by blank lines.
    pub paragraphs: usize,
}

pub fn count(text: &str) -> Stats {
    let mut stats = Stats::default();
    let mut in_paragraph = false;
    for line in text.lines() {
        stats.chars += line.chars().filter(|&c| c != '\r').count();
        // A word needs at least one letter or digit, so table rules (`|---|`),
        // list markers and lone punctuation are not counted.
        stats.words += line
            .split_whitespace()
            .filter(|w| w.chars().any(char::is_alphanumeric))
            .count();
        if line.trim().is_empty() {
            in_paragraph = false;
        } else if !in_paragraph {
            in_paragraph = true;
            stats.paragraphs += 1;
        }
    }
    stats
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text() {
        assert_eq!(count(""), Stats::default());
        assert_eq!(count("\n\n\n"), Stats::default());
    }

    #[test]
    fn counts_words_chars_paragraphs() {
        let s = count("Hello world.\nSecond line\n\n\nNew paragraph — here\n");
        assert_eq!(s.words, 7);
        assert_eq!(s.chars, 12 + 11 + 20);
        assert_eq!(s.paragraphs, 2);
    }

    #[test]
    fn ignores_markup_without_letters() {
        let s = count("| a | b |\n|---|:-:|\n- [ ] task\n");
        assert_eq!(s.words, 3);
        assert_eq!(s.paragraphs, 1);
    }

    #[test]
    fn crlf_line_breaks_are_not_characters() {
        assert_eq!(count("ab\r\ncd\r\n").chars, 4);
    }
}
