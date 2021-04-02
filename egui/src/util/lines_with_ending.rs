/// Use this iterator when [`str::lines()`] is not enough
/// For a given string yeald all lines including the new line byte(s)
///
///```rust
/// use egui::util::LinesWithEnding;
///
/// let mut iter = LinesWithEnding::new("AAAA\r\n\nBBBB\nCCCC");
/// assert_eq!(Some("AAAA\r\n"), iter.next());
/// assert_eq!(Some("\n"), iter.next());
/// assert_eq!(Some("BBBB\n"), iter.next());
/// assert_eq!(Some("CCCC"), iter.next());
/// assert_eq!(None, iter.next());
/// ```
pub struct LinesWithEnding<'a> {
    string: &'a str,
}

impl<'a> LinesWithEnding<'a> {
    pub fn new(string: &'a str) -> Self {
        Self { string }
    }
}

impl<'a> Iterator for LinesWithEnding<'a> {
    type Item = &'a str;

    #[inline]
    fn next(&mut self) -> Option<&'a str> {
        if self.string.is_empty() {
            return None;
        }

        let split_at = match self.string.find('\n') {
            Some(x) => x + 1,
            None => self.string.chars().count(),
        };

        let (line, rest) = self.string.split_at(split_at);
        self.string = rest;

        Some(line)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lines_with_ending() {
        let mut iter = LinesWithEnding::new("AAAA\r\n\nBBBB\nCCCC");

        assert_eq!(Some("AAAA\r\n"), iter.next());
        assert_eq!(Some("\n"), iter.next());
        assert_eq!(Some("BBBB\n"), iter.next());
        assert_eq!(Some("CCCC"), iter.next());
        assert_eq!(None, iter.next());
    }
}
