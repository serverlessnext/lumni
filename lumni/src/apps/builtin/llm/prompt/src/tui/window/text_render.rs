use ratatui::style::{Color, Style};
use ratatui::text::{Line, Masked, Span};

use super::text_display::{
    CodeBlockLine, CodeBlockLineType, LineSegment, LineType,
};

pub struct DisplayWindowRenderer<'a> {
    wrap_lines: &'a [LineSegment<'a>],
    window_width: usize,
}

impl<'a> DisplayWindowRenderer<'a> {
    pub fn new(wrap_lines: &'a [LineSegment<'a>], window_width: usize) -> Self {
        Self {
            wrap_lines,
            window_width,
        }
    }

    pub fn render_lines(&self, start: usize, end: usize) -> Vec<Line<'a>> {
        let length = self.wrap_lines.len();

        if start >= length {
            return Vec::new(); // out of bounds
        }

        // Convert inclusive end to exclusive end for slicing
        let exclusive_end = (end + 1).min(length);

        if start > end {
            return Vec::new(); // invalid range
        }

        self.wrap_lines[start..exclusive_end]
            .iter()
            .map(|line_segment| self.render_line(line_segment))
            .collect()
    }

    fn render_line(&self, line_segment: &'a LineSegment<'a>) -> Line<'a> {
        let mut line = line_segment.line.clone();
        let text_width = line.width();

        let line_type = line_segment.line_type;
        let background = self.determine_background(line_segment, line_type);

        match line_type {
            Some(LineType::Text) => {
                self.apply_text_styling(&mut line, background)
            }
            Some(LineType::Code(block_line)) => {
                line =
                    self.apply_code_block_styling(line, block_line, background);
            }
            None => {}
        }

        self.add_padding(&mut line, text_width, background);

        line
    }

    fn determine_background(
        &self,
        line_segment: &LineSegment<'a>,
        line_type: Option<LineType>,
    ) -> Option<Color> {
        match line_type {
            Some(LineType::Text) => {
                if line_segment.background.is_some() {
                    Some(if line_segment.background == Some(Color::Reset) {
                        Color::Black
                    } else {
                        line_segment.background.unwrap()
                    })
                } else {
                    None
                }
            }
            Some(LineType::Code(block_line)) => match block_line.get_type() {
                CodeBlockLineType::Line => Some(Color::Rgb(80, 80, 80)),
                _ => line_segment.background,
            },
            None => line_segment.background,
        }
    }

    fn apply_text_styling(&self, line: &mut Line, background: Option<Color>) {
        if let Some(bg) = background {
            for span in &mut line.spans {
                if span.style.bg.is_none()
                    || span.style.bg == Some(Color::Reset)
                {
                    span.style.bg = Some(bg);
                }
            }
        }
    }

    fn apply_code_block_styling(
        &self,
        line: Line<'a>,
        block_line: CodeBlockLine,
        background: Option<Color>,
    ) -> Line<'a> {
        match block_line.get_type() {
            CodeBlockLineType::Line => {
                let bg = Some(Color::Rgb(80, 80, 80));
                Line::from(
                    line.spans
                        .into_iter()
                        .map(|mut span| {
                            if span.style.bg.is_none()
                                || span.style.bg == Some(Color::Reset)
                            {
                                span.style.bg = bg;
                            }
                            span
                        })
                        .collect::<Vec<_>>(),
                )
            }
            CodeBlockLineType::Start => {
                let masked = Masked::new("```", '>');
                Line::from(vec![self.create_masked_span(masked, background)])
            }
            CodeBlockLineType::End => {
                let masked = Masked::new("```", '<');
                Line::from(vec![self.create_masked_span(masked, background)])
            }
        }
    }

    fn create_masked_span(
        &self,
        masked: Masked<'_>,
        background: Option<Color>,
    ) -> Span<'a> {
        if let Some(bg) = background {
            Span::styled(masked.to_string(), Style::default().bg(bg))
        } else {
            Span::raw(masked.to_string())
        }
    }

    fn add_padding(
        &self,
        line: &mut Line,
        text_width: usize,
        background: Option<Color>,
    ) {
        // Left padding
        line.spans.insert(
            0,
            Span::styled(
                " ",
                Style::default().bg(background.unwrap_or(Color::Black)),
            ),
        );

        // Right padding and fill
        if text_width.saturating_add(2) < self.window_width {
            let spaces_needed =
                self.window_width.saturating_sub(text_width + 2);
            let spaces = " ".repeat(spaces_needed);

            if let Some(bg_color) = background {
                line.spans
                    .push(Span::styled(spaces, Style::default().bg(bg_color)));
            } else {
                line.spans.push(Span::raw(spaces));
            }
        }

        // Right padding
        line.spans.push(Span::styled(
            " ",
            Style::default().bg(background.unwrap_or(Color::Black)),
        ));
    }
}
