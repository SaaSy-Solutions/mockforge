//! Color palette and style helpers with dark and light theme support.

use std::sync::OnceLock;

use ratatui::style::{Color, Modifier, Style};

/// Active palette, set once at startup via [`Theme::init`].
static PALETTE: OnceLock<Palette> = OnceLock::new();

/// A complete colour palette.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub bg: Color,
    pub fg: Color,
    pub dim: Color,
    pub surface: Color,
    pub overlay: Color,
    pub blue: Color,
    pub green: Color,
    pub red: Color,
    pub yellow: Color,
    pub peach: Color,
    pub mauve: Color,
    pub teal: Color,
    pub pink: Color,
}

impl Palette {
    /// Catppuccin Mocha (dark).
    pub const DARK: Self = Self {
        bg: Color::Rgb(22, 22, 30),
        fg: Color::Rgb(205, 214, 244),
        dim: Color::Rgb(108, 112, 134),
        surface: Color::Rgb(30, 30, 46),
        overlay: Color::Rgb(49, 50, 68),
        blue: Color::Rgb(137, 180, 250),
        green: Color::Rgb(166, 227, 161),
        red: Color::Rgb(243, 139, 168),
        yellow: Color::Rgb(249, 226, 175),
        peach: Color::Rgb(250, 179, 135),
        mauve: Color::Rgb(203, 166, 247),
        teal: Color::Rgb(148, 226, 213),
        pink: Color::Rgb(245, 194, 231),
    };

    /// Catppuccin Latte (light).
    pub const LIGHT: Self = Self {
        bg: Color::Rgb(239, 241, 245),
        fg: Color::Rgb(76, 79, 105),
        dim: Color::Rgb(140, 143, 161),
        surface: Color::Rgb(230, 233, 239),
        overlay: Color::Rgb(204, 208, 218),
        blue: Color::Rgb(30, 102, 245),
        green: Color::Rgb(64, 160, 43),
        red: Color::Rgb(210, 15, 57),
        yellow: Color::Rgb(223, 142, 29),
        peach: Color::Rgb(254, 100, 11),
        mauve: Color::Rgb(136, 57, 239),
        teal: Color::Rgb(23, 146, 153),
        pink: Color::Rgb(234, 118, 203),
    };
}

/// Theme accessor. All colour/style lookups go through here.
pub struct Theme;

impl Theme {
    /// Initialise the global palette. Call once at startup.
    /// If not called, defaults to dark.
    pub fn init(light: bool) {
        let palette = if light { Palette::LIGHT } else { Palette::DARK };
        let _ = PALETTE.set(palette);
    }

    #[inline]
    fn p() -> &'static Palette {
        PALETTE.get().unwrap_or(&Palette::DARK)
    }

    // ── Colour accessors ────────────────────────────────────────────
    #[inline]
    pub fn fg() -> Color {
        Self::p().fg
    }
    #[inline]
    pub fn bg() -> Color {
        Self::p().bg
    }
    #[inline]
    pub fn dim_color() -> Color {
        Self::p().dim
    }
    #[inline]
    pub fn surface_color() -> Color {
        Self::p().surface
    }
    #[inline]
    pub fn overlay_color() -> Color {
        Self::p().overlay
    }
    #[inline]
    pub fn blue() -> Color {
        Self::p().blue
    }
    #[inline]
    pub fn green() -> Color {
        Self::p().green
    }
    #[inline]
    pub fn red() -> Color {
        Self::p().red
    }
    #[inline]
    pub fn yellow() -> Color {
        Self::p().yellow
    }
    #[inline]
    pub fn peach() -> Color {
        Self::p().peach
    }
    #[inline]
    pub fn mauve() -> Color {
        Self::p().mauve
    }
    #[inline]
    pub fn teal() -> Color {
        Self::p().teal
    }
    #[inline]
    pub fn pink() -> Color {
        Self::p().pink
    }

    // ── Backward-compatible `const` aliases (dark defaults) ─────────
    // These exist only for code that uses `Theme::FG` etc. in const contexts.
    // Prefer the functions above for theme-aware colours.
    pub const FG: Color = Color::Rgb(205, 214, 244);
    pub const BG: Color = Color::Rgb(22, 22, 30);
    pub const DIM: Color = Color::Rgb(108, 112, 134);
    pub const SURFACE: Color = Color::Rgb(30, 30, 46);
    pub const OVERLAY: Color = Color::Rgb(49, 50, 68);
    pub const BLUE: Color = Color::Rgb(137, 180, 250);
    pub const GREEN: Color = Color::Rgb(166, 227, 161);
    pub const RED: Color = Color::Rgb(243, 139, 168);
    pub const YELLOW: Color = Color::Rgb(249, 226, 175);
    pub const PEACH: Color = Color::Rgb(250, 179, 135);
    pub const MAUVE: Color = Color::Rgb(203, 166, 247);
    pub const TEAL: Color = Color::Rgb(148, 226, 213);
    pub const PINK: Color = Color::Rgb(245, 194, 231);

    pub const STATUS_UP: Color = Self::GREEN;
    pub const STATUS_DOWN: Color = Self::RED;
    pub const STATUS_WARN: Color = Self::YELLOW;

    // ── Composed styles (theme-aware) ───────────────────────────────

    pub fn base() -> Style {
        Style::default().fg(Self::fg()).bg(Self::bg())
    }

    pub fn surface() -> Style {
        Style::default().fg(Self::fg()).bg(Self::surface_color())
    }

    pub fn title() -> Style {
        Style::default().fg(Self::blue()).add_modifier(Modifier::BOLD)
    }

    pub fn highlight() -> Style {
        Style::default().fg(Self::bg()).bg(Self::blue())
    }

    pub fn tab_active() -> Style {
        Style::default().fg(Self::bg()).bg(Self::blue()).add_modifier(Modifier::BOLD)
    }

    pub fn tab_inactive() -> Style {
        Style::default().fg(Self::dim_color()).bg(Self::surface_color())
    }

    pub fn status_bar() -> Style {
        Style::default().fg(Self::fg()).bg(Self::overlay_color())
    }

    pub fn key_hint() -> Style {
        Style::default().fg(Self::yellow()).add_modifier(Modifier::BOLD)
    }

    pub fn error() -> Style {
        Style::default().fg(Self::red())
    }

    pub fn success() -> Style {
        Style::default().fg(Self::green())
    }

    pub fn dim() -> Style {
        Style::default().fg(Self::dim_color())
    }

    pub fn http_method(method: &str) -> Style {
        let color = match method.to_uppercase().as_str() {
            "GET" => Self::green(),
            "POST" => Self::blue(),
            "PUT" => Self::yellow(),
            "PATCH" => Self::peach(),
            "DELETE" => Self::red(),
            "HEAD" | "OPTIONS" => Self::dim_color(),
            _ => Self::fg(),
        };
        Style::default().fg(color).add_modifier(Modifier::BOLD)
    }

    pub fn status_code(code: u16) -> Style {
        let color = match code {
            200..=299 => Self::green(),
            300..=399 => Self::blue(),
            400..=499 => Self::yellow(),
            500..=599 => Self::red(),
            _ => Self::dim_color(),
        };
        Style::default().fg(color)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dark_palette_default() {
        // Without init, should return dark palette colours.
        let p = Theme::p();
        assert_eq!(p.bg, Palette::DARK.bg);
        assert_eq!(p.fg, Palette::DARK.fg);
    }

    #[test]
    fn composed_styles_use_palette() {
        let base = Theme::base();
        assert_eq!(base.fg, Some(Theme::fg()));
        assert_eq!(base.bg, Some(Theme::bg()));
    }

    #[test]
    fn http_method_styles() {
        let get_style = Theme::http_method("GET");
        assert_eq!(get_style.fg, Some(Theme::green()));

        let delete_style = Theme::http_method("DELETE");
        assert_eq!(delete_style.fg, Some(Theme::red()));
    }

    #[test]
    fn status_code_styles() {
        assert_eq!(Theme::status_code(200).fg, Some(Theme::green()));
        assert_eq!(Theme::status_code(404).fg, Some(Theme::yellow()));
        assert_eq!(Theme::status_code(500).fg, Some(Theme::red()));
    }

    #[test]
    fn const_aliases_match_dark_palette() {
        assert_eq!(Theme::FG, Palette::DARK.fg);
        assert_eq!(Theme::BG, Palette::DARK.bg);
        assert_eq!(Theme::BLUE, Palette::DARK.blue);
        assert_eq!(Theme::RED, Palette::DARK.red);
    }

    #[test]
    fn light_palette_differs_from_dark() {
        assert_ne!(Palette::DARK.bg, Palette::LIGHT.bg);
        assert_ne!(Palette::DARK.fg, Palette::LIGHT.fg);
        assert_ne!(Palette::DARK.blue, Palette::LIGHT.blue);
    }
}
