use ratatui::style::{Color, Style};

pub enum ColorSchemeType {
    Default,
    Dark,
    Light,
    HighContrast,
    Pastel,
}

pub struct ColorSet {
    pub background: Color,
    pub primary_fg: Color,
    pub primary_bg: Color,
    pub secondary_fg: Color,
    pub secondary_bg: Color,
}

impl ColorSet {
    pub fn primary_style(&self) -> Style {
        Style::new().fg(self.primary_fg).bg(self.primary_bg)
    }

    pub fn secondary_style(&self) -> Style {
        Style::new().fg(self.secondary_fg).bg(self.secondary_bg)
    }
}

pub struct ColorScheme {
    colors: ColorSet,
}

impl ColorScheme {
    pub fn new(scheme_type: ColorSchemeType) -> Self {
        let colors = Self::get_colors(&scheme_type);
        ColorScheme {
            colors: colors,
        }
    }

    pub fn switch_scheme(&mut self, new_scheme: ColorSchemeType) {
        self.colors = Self::get_colors(&new_scheme);
    }

    pub fn get_primary_style(&self) -> Style {
        self.colors.primary_style()
    }

    pub fn get_secondary_style(&self) -> Style {
        self.colors.secondary_style()
    }

    pub fn get_background(&self) -> Color {
        self.colors.background
    }

    fn get_colors(scheme: &ColorSchemeType) -> ColorSet {
        match scheme {
            ColorSchemeType::Default => ColorSet {
                background: Color::Black,
                primary_fg: Color::White,
                primary_bg: Color::Rgb(0, 48, 48),
                secondary_fg: Color::White,
                secondary_bg: Color::Rgb(0, 24, 24),
            },
            ColorSchemeType::Dark => ColorSet {
                background: Color::Rgb(15, 15, 15),
                primary_fg: Color::Rgb(200, 200, 200),
                primary_bg: Color::Rgb(30, 30, 30),
                secondary_fg: Color::Rgb(200, 200, 200),
                secondary_bg: Color::Rgb(45, 45, 45),
            },
            ColorSchemeType::Light => ColorSet {
                background: Color::White,
                primary_fg: Color::Black,
                primary_bg: Color::Rgb(230, 230, 230),
                secondary_fg: Color::Black,
                secondary_bg: Color::Rgb(200, 200, 200),
            },
            ColorSchemeType::HighContrast => ColorSet {
                background: Color::Black,
                primary_fg: Color::White,
                primary_bg: Color::Blue,
                secondary_fg: Color::White,
                secondary_bg: Color::Green,
            },
            ColorSchemeType::Pastel => ColorSet {
                background: Color::Rgb(255, 255, 240),
                primary_fg: Color::Black,
                primary_bg: Color::Rgb(255, 204, 204),
                secondary_fg: Color::Black,
                secondary_bg: Color::Rgb(204, 229, 255),
            },
        }
    }
}