//! Theme definitions for the TUI.

use ratatui::style::Color;

/// A color theme for the TUI.
#[derive(Debug, Clone)]
pub struct Theme {
    /// Theme name
    pub name: &'static str,

    // Base colors
    /// Background color
    pub bg: Color,
    /// Foreground (text) color
    pub fg: Color,

    // Semantic colors
    /// Color for received data
    pub rx_color: Color,
    /// Color for transmitted data
    pub tx_color: Color,
    /// Color for error messages
    pub error_color: Color,
    /// Color for success messages
    pub success_color: Color,
    /// Color for warnings
    pub warning_color: Color,

    // UI element colors
    /// Border color
    pub border: Color,
    /// Selection/highlight color
    pub selection: Color,
    /// Cursor color
    pub cursor: Color,
    /// Inactive element color
    pub inactive: Color,
    /// Accent color for highlights
    pub accent: Color,
}

impl Theme {
    /// Dark theme (default)
    pub const fn dark() -> Self {
        Self {
            name: "dark",
            bg: Color::Rgb(30, 30, 46),
            fg: Color::Rgb(205, 214, 244),
            rx_color: Color::Rgb(166, 227, 161),
            tx_color: Color::Rgb(137, 180, 250),
            error_color: Color::Rgb(243, 139, 168),
            success_color: Color::Rgb(166, 227, 161),
            warning_color: Color::Rgb(249, 226, 175),
            border: Color::Rgb(88, 91, 112),
            selection: Color::Rgb(69, 71, 90),
            cursor: Color::Rgb(245, 224, 220),
            inactive: Color::Rgb(108, 112, 134),
            accent: Color::Rgb(203, 166, 247),
        }
    }

    /// Light theme
    pub const fn light() -> Self {
        Self {
            name: "light",
            bg: Color::Rgb(239, 241, 245),
            fg: Color::Rgb(76, 79, 105),
            rx_color: Color::Rgb(64, 160, 43),
            tx_color: Color::Rgb(30, 102, 245),
            error_color: Color::Rgb(210, 15, 57),
            success_color: Color::Rgb(64, 160, 43),
            warning_color: Color::Rgb(223, 142, 29),
            border: Color::Rgb(172, 176, 190),
            selection: Color::Rgb(204, 208, 218),
            cursor: Color::Rgb(220, 138, 120),
            inactive: Color::Rgb(140, 143, 161),
            accent: Color::Rgb(136, 57, 239),
        }
    }

    /// Solarized Dark theme
    pub const fn solarized_dark() -> Self {
        Self {
            name: "solarized",
            bg: Color::Rgb(0, 43, 54),
            fg: Color::Rgb(131, 148, 150),
            rx_color: Color::Rgb(133, 153, 0),
            tx_color: Color::Rgb(38, 139, 210),
            error_color: Color::Rgb(220, 50, 47),
            success_color: Color::Rgb(133, 153, 0),
            warning_color: Color::Rgb(181, 137, 0),
            border: Color::Rgb(88, 110, 117),
            selection: Color::Rgb(7, 54, 66),
            cursor: Color::Rgb(238, 232, 213),
            inactive: Color::Rgb(101, 123, 131),
            accent: Color::Rgb(108, 113, 196),
        }
    }

    /// Dracula theme
    pub const fn dracula() -> Self {
        Self {
            name: "dracula",
            bg: Color::Rgb(40, 42, 54),
            fg: Color::Rgb(248, 248, 242),
            rx_color: Color::Rgb(80, 250, 123),
            tx_color: Color::Rgb(139, 233, 253),
            error_color: Color::Rgb(255, 85, 85),
            success_color: Color::Rgb(80, 250, 123),
            warning_color: Color::Rgb(241, 250, 140),
            border: Color::Rgb(68, 71, 90),
            selection: Color::Rgb(68, 71, 90),
            cursor: Color::Rgb(255, 121, 198),
            inactive: Color::Rgb(98, 114, 164),
            accent: Color::Rgb(189, 147, 249),
        }
    }

    /// Nord theme
    pub const fn nord() -> Self {
        Self {
            name: "nord",
            bg: Color::Rgb(46, 52, 64),
            fg: Color::Rgb(216, 222, 233),
            rx_color: Color::Rgb(163, 190, 140),
            tx_color: Color::Rgb(129, 161, 193),
            error_color: Color::Rgb(191, 97, 106),
            success_color: Color::Rgb(163, 190, 140),
            warning_color: Color::Rgb(235, 203, 139),
            border: Color::Rgb(76, 86, 106),
            selection: Color::Rgb(67, 76, 94),
            cursor: Color::Rgb(236, 239, 244),
            inactive: Color::Rgb(107, 112, 137),
            accent: Color::Rgb(180, 142, 173),
        }
    }

    /// Get theme by name
    pub fn by_name(name: &str) -> Option<&'static Theme> {
        THEMES.iter().find(|t| t.name == name)
    }
}

impl Default for Theme {
    fn default() -> Self {
        Self::dark()
    }
}

/// Available themes
pub static THEMES: &[Theme] = &[
    Theme::dark(),
    Theme::light(),
    Theme::solarized_dark(),
    Theme::dracula(),
    Theme::nord(),
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_theme_by_name() {
        assert!(Theme::by_name("dark").is_some());
        assert!(Theme::by_name("light").is_some());
        assert!(Theme::by_name("solarized").is_some());
        assert!(Theme::by_name("dracula").is_some());
        assert!(Theme::by_name("nord").is_some());
        assert!(Theme::by_name("nonexistent").is_none());
    }

    #[test]
    fn test_theme_default() {
        let theme = Theme::default();
        assert_eq!(theme.name, "dark");
    }
}
