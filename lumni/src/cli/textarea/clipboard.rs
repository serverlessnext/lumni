use arboard::Clipboard;
use std::io::{self, Error, Write};

pub struct ClipboardProvider {
    clipboard: Clipboard,
}

impl ClipboardProvider {
    pub fn new() -> Self {
        let clipboard = Clipboard::new().unwrap();
        Self { clipboard }
    }

    pub fn write_line(&mut self, s: &str, append: bool) -> io::Result<()> {
        let new_text = if append {
            let mut current_text = self.clipboard.get_text().unwrap_or_default();
            if !current_text.is_empty() {
                current_text.push('\n'); // Ensure newline separation between entries
            }
            current_text.push_str(s);
            current_text
        } else {
            s.to_string()
        };

        self.clipboard
            .set_text(new_text)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e.to_string()))
    }

    pub fn read_text(&mut self) -> Result<String, Error> {
        self.clipboard
            .get_text()
            .map_err(|e| Error::new(io::ErrorKind::Other, e.to_string()))
    }
}

impl Write for ClipboardProvider {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut current_text = self.clipboard.get_text().unwrap_or_default();
        let new_text = String::from_utf8_lossy(buf);
        current_text.push_str(&new_text);
        self.clipboard
            .set_text(current_text)
            .map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}
