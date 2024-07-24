use ratatui::style::{Color, Style};


#[derive(Clone, Debug, PartialEq)]
pub struct TextLine {
    pub segments: Vec<TextSegment>,
    pub length: usize,
    pub background: Option<Color>,

}

impl TextLine {
    pub fn new() -> Self {
        TextLine {
            segments: Vec::new(),
            length: 0,
            background: None,
        }
    }

    pub fn add_segment(&mut self, text: String, style: Option<Style>) {
        self.length += text.len();

        if let Some(last) = self.segments.last_mut() {
            // update the length of the last segment
            if last.style == style {
                // Append text to the last segment if styles are the same
                last.text.push_str(&text);
                return;
            }
        }
        // Otherwise, create a new segment
        self.segments.push(TextSegment { text, style });
        if let Some(style) = style {
            self.background = style.bg;
        }
    }

    pub fn is_empty(&self) -> bool {
        self.segments.is_empty()
    }

    pub fn segments(&self) -> impl Iterator<Item = &TextSegment> {
        self.segments.iter()
    }

    pub fn get_length(&self) -> usize {
        self.length
    }

    pub fn get_background(&self) -> Option<Color> {
        self.background
    }

    pub fn to_string(&self) -> String {
        let mut content = String::new();
        for segment in &self.segments {
            content.push_str(&segment.text);
        }
        content
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TextSegment {
    pub text: String,
    pub style: Option<Style>,
}


