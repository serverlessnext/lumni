use super::piece_table::{TextLine, TextSegment};

pub struct TextWrapper {
    display_width: usize,
}

impl TextWrapper {
    pub fn new(display_width: usize) -> Self {
        Self { display_width }
    }

    pub fn wrap_text_styled(&self, line: &TextLine) -> Vec<TextLine> {
        let mut wrapped_lines = Vec::new();
        let mut current_line = TextLine::new();
        let max_width = self.display_width.saturating_sub(2);

        if max_width < 1 {
            return wrapped_lines;
        }

        for segment in line.segments() {
            self.wrap_segment(
                segment,
                &mut current_line,
                &mut wrapped_lines,
                max_width,
            );
        }

        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }

        wrapped_lines
    }

    fn wrap_segment(
        &self,
        segment: &TextSegment,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
        max_width: usize,
    ) {
        let text = segment.text();
        let mut current_text = String::new();
        let words = self.split_text_into_words(text);

        for (mut leading_spaces, word) in words {
           if word.contains("```") {
                // any existing current text should be added to the current line
                // before we process the triple backticks
                self.add_current_text_to_line(
                    &mut current_text,
                    segment,
                    current_line,
                );

                self.handle_triple_backticks(
                    &leading_spaces,
                    &word,
                    segment,
                    current_line,
                    wrapped_lines,
                );
                continue;
            }

            let space_len = if !current_text.is_empty() { 1 } else { 0 };

            let needs_wrapping = current_text.len()
                + space_len
                + leading_spaces.len()
                + word.len()
                + current_line.length()
                > max_width;

            if needs_wrapping {
                self.add_current_text_to_line(
                    &mut current_text,
                    segment,
                    current_line,
                );

                if leading_spaces.len() + word.len() > max_width {
                    // word is too long to fit on a single line
                    self.handle_long_word(
                        &leading_spaces,
                        &word,
                        segment,
                        current_line,
                        wrapped_lines,
                        max_width,
                    );
                    continue;
                } else {
                    if !leading_spaces.is_empty() {
                        leading_spaces.remove(0);
                    }
                    wrapped_lines.push(current_line.clone());
                    *current_line = TextLine::new();
                    current_text = format!("{}{}", leading_spaces, word);
                }
            } else {
                current_text.push_str(&format!("{}{}", leading_spaces, word));
            }
        }

        if !current_text.is_empty() {
            current_line.add_segment(
                current_text.trim_end().to_string(),
                segment.style().clone(),
            );
        }
    }

    fn split_text_into_words(&self, text: &str) -> Vec<(String, String)> {
        let re = regex::Regex::new(r"(\s*)(\S+)").unwrap();
        re.captures_iter(text)
            .map(|cap| (cap[1].to_string(), cap[2].to_string()))
            .collect()
    }

    fn add_current_text_to_line(
        &self,
        current_text: &mut String,
        segment: &TextSegment,
        current_line: &mut TextLine,
    ) {
        if !current_text.is_empty() {
            current_line.add_segment(
                current_text.trim_end().to_string(),
                segment.style().clone(),
            );
            current_text.clear();
        }
    }

    fn handle_long_word(
        &self,
        leading_spaces: &str,
        word: &str,
        segment: &TextSegment,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
        max_width: usize,
    ) {
        let mut current_text = leading_spaces.to_string();
        let mut start_index = 0;
        while start_index < word.len() {
            let end_index = std::cmp::min(
                (start_index + max_width).saturating_sub(current_text.len()),
                word.len(),
            );
            let slice = &word[start_index..end_index];

            if !current_line.is_empty() {
                wrapped_lines.push(current_line.clone());
                *current_line = TextLine::new();
            }

            current_line.add_segment(
                current_text.clone() + slice,
                segment.style().clone(),
            );
            wrapped_lines.push(current_line.clone());
            *current_line = TextLine::new();
            start_index = end_index;
            current_text.clear();
        }
    }

    fn handle_triple_backticks(
        &self,
        leading_spaces: &str,
        word: &str,
        segment: &TextSegment,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
    ) {
        let mut leading_spaces = leading_spaces.to_string();

        let parts: Vec<&str> = word.split("```").collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                if i == 0 {
                    // if the first part is text, leading spaces should be kept
                    current_line.add_segment(
                        format!("{}{}", leading_spaces, part),
                        segment.style().clone(),
                    );
                    leading_spaces.clear(); // leading spaces are only added once
                } else {
                    // first part is triple-backticks, text is always added on a new line
                    current_line.add_segment(part.to_string(), segment.style().clone());
                }                
                wrapped_lines.push(current_line.clone());
                *current_line = TextLine::new();
            }
            if i < parts.len() - 1 {
                // Add the triple-backticks on its own line
                // first ensure current line is empty
                if !current_line.is_empty() {
                    // leading spaces are preserved on the line, instead of being
                    // passed to the next line as is typical with wrapping. This is because
                    // triple-backticks are always on their own line with no leading spaces.
                    if leading_spaces.len() > 0 {
                        current_line.add_segment(
                            leading_spaces.clone(),
                            segment.style().clone(),
                        );
                    }
                    wrapped_lines.push(current_line.clone());
                    *current_line = TextLine::new();
                }
                current_line.add_segment("```".to_string(), segment.style().clone());
                wrapped_lines.push(current_line.clone());
                *current_line = TextLine::new();
            }
        }
    }
}

