//! Styling Definitions for Iced Widgets
use iced_wgpu::core::{Shadow, Vector};
use iced_widget::core::{Background, Border, Color};
use iced_widget::{button, container, text_input, theme, Theme};

/// Generate Password TextInput Theme
pub fn password() -> theme::TextInput {
    theme::TextInput::Custom(Box::new(PasswordStyle {}))
}

/// Generate Show/Hide Button Theme
pub fn show() -> theme::Button {
    theme::Button::Custom(Box::new(ButtonStyle {}))
}

/// Generate MenuBox Container Theme
pub fn menubox() -> theme::Container {
    theme::Container::Custom(Box::new(MenuBoxStyle {}))
}

/// Password TextInput Styling
struct PasswordStyle {}

impl text_input::StyleSheet for PasswordStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> text_input::Appearance {
        let blank = Color::TRANSPARENT;
        let text_color = Color::WHITE;
        text_input::Appearance {
            background: Background::Color(blank),
            border: Border::default(),
            icon_color: text_color,
        }
    }
    fn focused(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    fn hovered(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    fn disabled(&self, style: &Self::Style) -> text_input::Appearance {
        self.active(style)
    }
    fn value_color(&self, _style: &Self::Style) -> Color {
        Color::WHITE
    }
    fn disabled_color(&self, _style: &Self::Style) -> Color {
        Color::BLACK
    }
    fn selection_color(&self, _style: &Self::Style) -> Color {
        Color::WHITE
    }
    fn placeholder_color(&self, _style: &Self::Style) -> Color {
        Color::WHITE
    }
}

/// Password Show/Hide Button Styling
struct ButtonStyle {}

impl button::StyleSheet for ButtonStyle {
    type Style = Theme;

    fn active(&self, _style: &Self::Style) -> button::Appearance {
        button::Appearance {
            shadow_offset: Vector::default(),
            background: None,
            border: Border::default(),
            shadow: Shadow::default(),
            text_color: Color::WHITE,
        }
    }
    fn pressed(&self, style: &Self::Style) -> button::Appearance {
        self.active(style)
    }
}

/// MenuBox Container Styling
struct MenuBoxStyle {}

impl container::StyleSheet for MenuBoxStyle {
    type Style = Theme;

    fn appearance(&self, _style: &Self::Style) -> container::Appearance {
        let bg = iced_widget::core::Color::from_rgba8(1, 4, 11, 0.7);
        let text = iced_widget::core::Color::WHITE;
        container::Appearance {
            background: Some(iced_widget::core::Background::Color(bg)),
            text_color: Some(text),
            ..container::Appearance::default()
        }
    }
}
