//! Lucide SVG icon system + a button helper that mirrors `button_ex` styling
//! but accepts an arbitrary Element child (so an SVG, or an SVG + label row,
//! can sit inside).
//!
//! Icons are vendored as SVG files under `assets/icons/`, embedded at compile
//! time and rendered through `iced::widget::svg`. Lucide SVGs use
//! `stroke="currentColor"` so iced's per-svg `Style { color }` override gives
//! us monochrome tinting that follows the active theme.

use std::collections::HashMap;
use std::sync::OnceLock;

use iced::alignment::Vertical;
use iced::widget::{button, svg};
use iced::{Color, ContentFit, Element, Length, Padding};

use iced_longbridge::components::button::Variant;
use iced_longbridge::components::icon::Icon as LbIcon;
use iced_longbridge::styles;
use iced_longbridge::theme::{with_alpha, AppTheme, Size};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum IconKind {
    Check,
    Close,
    FolderOpen,
    Save,
    Trash,
    Copy,
    Paste,
    ArrowUp,
    ArrowDown,
    PanelLeft,
    EllipsisVertical,
    Moon,
    Sun,
    Plus,
    FunctionSquare,
    Dot,
    Settings,
}

impl IconKind {
    fn bytes(self) -> &'static [u8] {
        match self {
            IconKind::Check => include_bytes!("../../assets/icons/check.svg"),
            IconKind::Close => include_bytes!("../../assets/icons/x.svg"),
            IconKind::FolderOpen => include_bytes!("../../assets/icons/folder-open.svg"),
            IconKind::Save => include_bytes!("../../assets/icons/save.svg"),
            IconKind::Trash => include_bytes!("../../assets/icons/trash-2.svg"),
            IconKind::Copy => include_bytes!("../../assets/icons/copy.svg"),
            IconKind::Paste => include_bytes!("../../assets/icons/clipboard-paste.svg"),
            IconKind::ArrowUp => include_bytes!("../../assets/icons/arrow-up.svg"),
            IconKind::ArrowDown => include_bytes!("../../assets/icons/arrow-down.svg"),
            IconKind::PanelLeft => include_bytes!("../../assets/icons/panel-left.svg"),
            IconKind::EllipsisVertical => {
                include_bytes!("../../assets/icons/ellipsis-vertical.svg")
            }
            IconKind::Moon => include_bytes!("../../assets/icons/moon.svg"),
            IconKind::Sun => include_bytes!("../../assets/icons/sun.svg"),
            IconKind::Plus => include_bytes!("../../assets/icons/plus.svg"),
            IconKind::FunctionSquare => include_bytes!("../../assets/icons/square-function.svg"),
            IconKind::Dot => include_bytes!("../../assets/icons/dot.svg"),
            IconKind::Settings => include_bytes!("../../assets/icons/settings.svg"),
        }
    }

    fn handle(self) -> svg::Handle {
        static CACHE: OnceLock<HashMap<IconKind, svg::Handle>> = OnceLock::new();
        let cache = CACHE.get_or_init(|| {
            const ALL: &[IconKind] = &[
                IconKind::Check,
                IconKind::Close,
                IconKind::FolderOpen,
                IconKind::Save,
                IconKind::Trash,
                IconKind::Copy,
                IconKind::Paste,
                IconKind::ArrowUp,
                IconKind::ArrowDown,
                IconKind::PanelLeft,
                IconKind::EllipsisVertical,
                IconKind::Moon,
                IconKind::Sun,
                IconKind::Plus,
                IconKind::FunctionSquare,
                IconKind::Dot,
                IconKind::Settings,
            ];
            ALL.iter()
                .map(|&k| (k, svg::Handle::from_memory(k.bytes())))
                .collect()
        });
        cache
            .get(&self)
            .cloned()
            .expect("every IconKind variant is in the cache")
    }
}

/// Lets `IconKind` flow into iced-longbridge builders that accept
/// `impl Into<Icon>` (menu items, etc.) and reuses the cached handle so we
/// don't reparse the SVG per render.
impl From<IconKind> for LbIcon {
    fn from(kind: IconKind) -> Self {
        LbIcon::from_svg_handle(kind.handle())
    }
}

/// Render an icon tinted with `theme.foreground`.
pub fn icon<'a, Message: 'a>(
    theme: &AppTheme,
    kind: IconKind,
    size: f32,
) -> Element<'a, Message> {
    icon_colored(kind, size, theme.foreground)
}

/// Render an icon with an explicit color (used for muted variants and for
/// icons sitting inside colored buttons whose foreground we already know).
pub fn icon_colored<'a, Message: 'a>(
    kind: IconKind,
    size: f32,
    color: Color,
) -> Element<'a, Message> {
    svg::Svg::new(kind.handle())
        .width(Length::Fixed(size))
        .height(Length::Fixed(size))
        .content_fit(ContentFit::Contain)
        .style(move |_theme: &iced::Theme, _status| svg::Style { color: Some(color) })
        .into()
}

/// Mirrors `iced_longbridge::components::button::button_ex`'s look, but takes
/// an arbitrary `Element` content (typically an icon, or an icon + label
/// row) rather than a `String`. Variant/Size styling is copied from
/// longbridge so the look matches the rest of the chrome exactly.
pub fn icon_button<'a, Message: Clone + 'a>(
    theme: &AppTheme,
    content: impl Into<Element<'a, Message>>,
    variant: Variant,
    size: Size,
    on_press: Option<Message>,
    disabled: bool,
) -> Element<'a, Message> {
    let theme = *theme;
    let height = size.height();
    let padding_x = size.padding_x();
    let radius = size.radius();

    let inner: Element<'a, Message> = iced::widget::container(content.into())
        .align_y(Vertical::Center)
        .height(Length::Fill)
        .into();

    let mut btn = button(inner)
        .padding(Padding::from([0.0, padding_x]))
        .height(Length::Fixed(height))
        .style(move |_, status| variant_style(&theme, variant, status, radius));

    if !disabled {
        if let Some(msg) = on_press {
            btn = btn.on_press(msg);
        }
    }
    btn.into()
}

fn variant_style(
    t: &AppTheme,
    variant: Variant,
    status: button::Status,
    radius: f32,
) -> button::Style {
    use button::Status::*;
    let (bg, fg, border_color, border_width) = match variant {
        Variant::Primary => {
            let bg = match status {
                Hovered => t.primary_hover,
                Pressed => t.primary_active,
                Disabled => with_alpha(t.primary, 0.5),
                Active => t.primary,
            };
            (Some(bg), t.primary_foreground, t.primary, 0.0)
        }
        Variant::Secondary => {
            let bg = match status {
                Hovered => t.secondary_hover,
                Pressed => t.secondary_active,
                Disabled => with_alpha(t.secondary, 0.5),
                Active => t.secondary,
            };
            (Some(bg), t.secondary_foreground, t.secondary, 0.0)
        }
        Variant::Outline => {
            let bg = match status {
                Hovered => t.accent,
                Pressed => t.muted,
                _ => t.background,
            };
            (Some(bg), t.foreground, t.border, 1.0)
        }
        Variant::Ghost => {
            let bg = match status {
                Hovered => t.accent,
                Pressed => t.muted,
                _ => Color::TRANSPARENT,
            };
            (Some(bg), t.foreground, Color::TRANSPARENT, 0.0)
        }
        Variant::Danger => (
            Some(styles::tinted_status(t.danger, status)),
            t.danger_foreground,
            t.danger,
            0.0,
        ),
        Variant::Success => (
            Some(styles::tinted_status(t.success, status)),
            t.success_foreground,
            t.success,
            0.0,
        ),
        Variant::Warning => (
            Some(styles::tinted_status(t.warning, status)),
            t.warning_foreground,
            t.warning,
            0.0,
        ),
        Variant::Info => (
            Some(styles::tinted_status(t.info, status)),
            t.info_foreground,
            t.info,
            0.0,
        ),
        Variant::Link => {
            let fg = match status {
                Hovered | Pressed => t.link_hover,
                _ => t.link,
            };
            (None, fg, Color::TRANSPARENT, 0.0)
        }
    };

    styles::button_style(bg, fg, border_color, border_width, radius)
}
