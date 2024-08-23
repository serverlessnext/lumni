use ratatui::style::Style;

use super::simple_string::SimpleString;
use super::text_line::TextLine;

pub struct TextWrapper {
    display_width: usize,
}

impl TextWrapper {
    pub fn new(display_width: usize) -> Self {
        Self { display_width }
    }
    // TODO: wrap_text_styled_with_delim

    pub fn wrap_text_styled(
        &self,
        line: &TextLine,
        first_line_max_width: Option<usize>,
    ) -> Vec<TextLine> {
        let max_display_width = self.display_width.saturating_sub(2);
        let max_first_line_width =
            first_line_max_width.unwrap_or(max_display_width);

        if max_display_width < 1 {
            return Vec::new();
        }

        let estimated_lines = line.get_length() / max_display_width + 1;
        let mut wrapped_lines = Vec::with_capacity(estimated_lines);
        let mut current_line = TextLine::new();
        let mut remaining_width = max_first_line_width;

        for segment in line.segments() {
            let mut word_iter = WordIterator::new(segment.text.as_str());

            while let Some((word, spaces)) = word_iter.next() {
                if word == "```" {
                    self.handle_code_block(
                        &mut current_line,
                        &mut wrapped_lines,
                        segment.style.as_ref(),
                        &mut remaining_width,
                        max_display_width,
                    );
                    continue;
                }

                if !spaces.is_empty() && remaining_width > 0 {
                    let space_width = spaces.len().min(remaining_width);
                    current_line.add_segment(
                        SimpleString::from_str(&spaces[..space_width]),
                        segment.style.clone(),
                    );
                    remaining_width -= space_width;
                }

                if word.len() > remaining_width {
                    if !current_line.is_empty() {
                        wrapped_lines.push(std::mem::replace(
                            &mut current_line,
                            TextLine::new(),
                        ));
                        remaining_width = max_display_width;
                    }
                    self.handle_long_word(
                        word,
                        segment.style.as_ref(),
                        &mut current_line,
                        &mut wrapped_lines,
                        &mut remaining_width,
                        max_display_width,
                    );
                } else {
                    current_line.add_segment(
                        SimpleString::from_str(word),
                        segment.style.clone(),
                    );
                    remaining_width -= word.len();
                }
            }
        }

        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }

        wrapped_lines
    }

    fn handle_code_block(
        &self,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
        style: Option<&Style>,
        remaining_width: &mut usize,
        max_display_width: usize,
    ) {
        if !current_line.is_empty() {
            wrapped_lines
                .push(std::mem::replace(current_line, TextLine::new()));
        }
        current_line.add_segment(SimpleString::from("```"), style.cloned());
        wrapped_lines.push(std::mem::replace(current_line, TextLine::new()));
        *remaining_width = max_display_width;
    }

    fn handle_long_word(
        &self,
        word: &str,
        style: Option<&Style>,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
        remaining_width: &mut usize,
        max_display_width: usize,
    ) {
        let mut start = 0;
        while start < word.len() {
            let end = (start + *remaining_width).min(word.len());
            let slice = &word[start..end];
            current_line
                .add_segment(SimpleString::from_str(slice), style.cloned());
            *remaining_width -= slice.len();

            if *remaining_width == 0 && end < word.len() {
                wrapped_lines
                    .push(std::mem::replace(current_line, TextLine::new()));
                *remaining_width = max_display_width;
            }

            start = end;
        }
    }
}

struct WordIterator<'a> {
    text: &'a str,
    position: usize,
}

impl<'a> WordIterator<'a> {
    fn new(text: &'a str) -> Self {
        Self { text, position: 0 }
    }
}

impl<'a> Iterator for WordIterator<'a> {
    type Item = (&'a str, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        if self.position >= self.text.len() {
            return None;
        }

        let remaining = &self.text[self.position..];
        let spaces_end = remaining
            .find(|c: char| !c.is_whitespace())
            .unwrap_or(remaining.len());
        let spaces = &remaining[..spaces_end];

        let word_start = self.position + spaces_end;
        if word_start >= self.text.len() {
            self.position = self.text.len();
            return Some((spaces, ""));
        }

        let word_end = self.text[word_start..]
            .find(char::is_whitespace)
            .map(|i| word_start + i)
            .unwrap_or(self.text.len());

        let word = &self.text[word_start..word_end];
        self.position = word_end;

        Some((word, spaces))
    }
}
