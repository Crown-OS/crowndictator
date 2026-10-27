//! The smallest edit that turns the text already injected into the focused
//! app into the latest text from the stream.

use unicode_segmentation::UnicodeSegmentation;

#[derive(Debug, PartialEq, Eq)]
pub struct TextEdit<'a> {
    /// Backspace presses: one per user-perceived character (grapheme cluster).
    pub backspaces: usize,
    /// The same span in UTF-8 bytes, as `delete_surrounding_text` wants it.
    pub delete_bytes: usize,
    pub insert: &'a str,
}

impl TextEdit<'_> {
    #[must_use]
    pub fn is_noop(&self) -> bool {
        self.delete_bytes == 0 && self.insert.is_empty()
    }
}

#[must_use]
pub fn diff<'a>(injected: &str, target: &'a str) -> TextEdit<'a> {
    let shared = shared_prefix_len(injected, target);
    let stale = &injected[shared..];
    TextEdit {
        backspaces: stale.graphemes(true).count(),
        delete_bytes: stale.len(),
        insert: &target[shared..],
    }
}

fn shared_prefix_len(left: &str, right: &str) -> usize {
    left.grapheme_indices(true)
        .zip(right.graphemes(true))
        .take_while(|((_, left_grapheme), right_grapheme)| left_grapheme == right_grapheme)
        .last()
        .map_or(0, |((start, grapheme), _)| start + grapheme.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edit(backspaces: usize, delete_bytes: usize, insert: &str) -> TextEdit<'_> {
        TextEdit {
            backspaces,
            delete_bytes,
            insert,
        }
    }

    #[test]
    fn diffs() {
        let cases = [
            ("", "", edit(0, 0, "")),
            ("hello", "hello", edit(0, 0, "")),
            ("", "hello", edit(0, 0, "hello")),
            ("hello", "", edit(5, 5, "")),
            ("hello", "hello world", edit(0, 0, " world")),
            ("hello world", "hello", edit(6, 6, "")),
            ("I want to go", "I wanted to go", edit(6, 6, "ed to go")),
            ("abc", "xyz", edit(3, 3, "xyz")),
            ("café", "cafe", edit(1, 2, "e")),
            ("naïve", "naïve!", edit(0, 0, "!")),
            ("hi 👋🏽", "hi 👋", edit(1, 8, "👋")),
            ("👨‍👩‍👧", "👨‍👩", edit(1, 18, "👨‍👩")),
            ("e\u{301}", "e", edit(1, 3, "e")),
            ("über", "Über", edit(4, 5, "Über")),
        ];
        for (injected, target, expected) in cases {
            assert_eq!(
                diff(injected, target),
                expected,
                "{injected:?} -> {target:?}"
            );
        }
    }

    #[test]
    fn noop_only_when_identical() {
        assert!(diff("same", "same").is_noop());
        assert!(!diff("same", "sam").is_noop());
        assert!(!diff("sam", "same").is_noop());
    }
}
