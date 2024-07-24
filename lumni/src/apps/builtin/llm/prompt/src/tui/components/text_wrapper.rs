use super::text_line::{TextLine, TextSegment};

pub struct TextWrapper {
    display_width: usize,
}

impl TextWrapper {
    pub fn new(display_width: usize) -> Self {
        Self { display_width }
    }

    pub fn wrap_text_styled(
        &self,
        line: &TextLine,
        first_line_max_width: Option<usize>,
    ) -> Vec<TextLine> {
        let mut wrapped_lines = Vec::new();
        let mut current_line = TextLine::new();
        let max_display_width = self.display_width.saturating_sub(2);

        let max_first_line_width = if let Some(max) = first_line_max_width {
            max
        } else {
            max_display_width
        };

        if max_display_width < 1 {
            // no space for text
            return wrapped_lines;
        }

        let mut is_first_line = true;

        for segment in line.segments() {
            self.wrap_segment(
                segment,
                &mut current_line,
                &mut wrapped_lines,
                max_display_width,
                max_first_line_width,
                &mut is_first_line,
            );
        }

        if !current_line.is_empty() {
            wrapped_lines.push(current_line);
        }

        // If the first line would be empty due to reserving too much space,
        // we ensure that an empty line is at the beginning of wrapped_lines.
        if max_first_line_width < 1 && !wrapped_lines.is_empty() {
            if !wrapped_lines[0].is_empty() {
                wrapped_lines.insert(0, TextLine::new());
            }
        }

        wrapped_lines
    }

    fn wrap_segment(
        &self,
        segment: &TextSegment,
        current_line: &mut TextLine,
        wrapped_lines: &mut Vec<TextLine>,
        max_display_width: usize,
        first_line_max_width: usize,
        is_first_line: &mut bool,
    ) {
        let mut current_text = String::new();
        let words = self.split_text_into_words(&segment.text);

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
                    is_first_line,
                );
                continue;
            }

            let space_len = if !current_text.is_empty() { 1 } else { 0 };

            let additional_length = leading_spaces.len() + word.len();
            let current_max_width = if *is_first_line == true {
                if additional_length > first_line_max_width {
                    // if leading spaces + single word is longer than width of first
                    // line, push existing contents immediately and move to next line
                    wrapped_lines.push(current_line.clone());
                    *current_line = TextLine::new();
                    *is_first_line = false;
                    max_display_width
                } else {
                    first_line_max_width
                }
            } else {
                max_display_width
            };

            let needs_wrapping = current_text.len()
                + space_len
                + additional_length
                + current_line.get_length()
                > current_max_width;

            if needs_wrapping {
                self.add_current_text_to_line(
                    &mut current_text,
                    segment,
                    current_line,
                );
                
                if additional_length > max_display_width {
                    self.handle_long_word(
                        &leading_spaces,
                        &word,
                        segment,
                        current_line,
                        wrapped_lines,
                        max_display_width,
                    );
                    continue;
                } else {
                    if !leading_spaces.is_empty() {
                        // Calculate the number of spaces that can be added
                        let available_spaces =
                            current_max_width.saturating_sub(current_line.get_length());
                        let spaces_to_add =
                            available_spaces.min(leading_spaces.len());
                        // Add the available leading spaces to the current line
                        current_line.add_segment(
                            leading_spaces[..spaces_to_add].to_string(),
                            segment.style.clone(),
                        );
                        // Update leading_spaces by slicing off the added spaces
                        leading_spaces =
                            leading_spaces[spaces_to_add..].to_string();
                    }
                    wrapped_lines.push(current_line.clone());
                    *current_line = TextLine::new();
                    *is_first_line = false;  // Set to false after pushing a line
                    current_text = format!("{}{}", leading_spaces, word);
                }
            } else {
                current_text.push_str(&format!("{}{}", leading_spaces, word));
            }
        }

        if !current_text.is_empty() {
            current_line
                .add_segment(current_text.to_string(), segment.style.clone());
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
            current_line
                .add_segment(current_text.to_string(), segment.style.clone());
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
        max_display_width: usize,
    ) {
        let mut current_text = leading_spaces.to_string();
        let mut start_index = 0;

        while start_index < word.len() {
            let end_index = std::cmp::min(
                (start_index + max_display_width).saturating_sub(current_text.len()),
                word.len(),
            );
            let slice = &word[start_index..end_index];

            if !current_line.is_empty() {
                wrapped_lines.push(current_line.clone());
                *current_line = TextLine::new();
            }

            current_line.add_segment(
                current_text.clone() + slice,
                segment.style.clone(),
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
        is_first_line: &mut bool,
    ) {
        let mut leading_spaces = leading_spaces.to_string();

        let parts: Vec<&str> = word.split("```").collect();
        for (i, part) in parts.iter().enumerate() {
            if !part.is_empty() {
                if i == 0 {
                    // if the first part is text, leading spaces should be kept
                    current_line.add_segment(
                        format!("{}{}", leading_spaces, part),
                        segment.style.clone(),
                    );
                    leading_spaces.clear(); // leading spaces are only added once
                } else {
                    // first part is triple-backticks, text is always added on a new line
                    current_line
                        .add_segment(part.to_string(), segment.style.clone());
                }
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
                            segment.style.clone(),
                        );
                    }
                    wrapped_lines.push(current_line.clone());
                    *current_line = TextLine::new();
                    *is_first_line = false;
                }
                current_line
                    .add_segment("```".to_string(), segment.style.clone());
                wrapped_lines.push(current_line.clone());
                *current_line = TextLine::new();
                *is_first_line = false;
            }
        }
    }
}
