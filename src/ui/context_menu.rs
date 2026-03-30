use iced::widget::{button, column, container, horizontal_rule, text};
use iced::{Border, Element, Length, Padding, Shadow, Vector};

use crate::Message;

pub const MENU_WIDTH: f32 = 160.0;

pub fn view_context_menu<'a>(row_index: usize, has_clipboard: bool) -> Element<'a, Message> {
    let items = column![
        menu_item("Cut", Message::CutRow(row_index)),
        menu_item("Copy", Message::CopyRow(row_index)),
        paste_item(row_index, has_clipboard),
        separator_item(),
        delete_item(row_index),
    ]
    .spacing(0)
    .width(Length::Fixed(MENU_WIDTH));

    container(items)
        .style(context_menu_container_style)
        .into()
}

fn menu_item<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13))
        .on_press(msg)
        .width(Length::Fixed(MENU_WIDTH))
        .padding(Padding::from([7.0, 12.0]))
        .style(menu_item_style)
        .into()
}

fn paste_item<'a>(row_index: usize, has_clipboard: bool) -> Element<'a, Message> {
    let btn = button(text("Paste").size(13))
        .width(Length::Fixed(MENU_WIDTH))
        .padding(Padding::from([7.0, 12.0]));
    if has_clipboard {
        btn.on_press(Message::PasteRow(row_index))
            .style(menu_item_style)
            .into()
    } else {
        btn.style(menu_item_disabled_style).into()
    }
}

fn delete_item<'a>(row_index: usize) -> Element<'a, Message> {
    button(text("Delete Row").size(13))
        .on_press(Message::DeleteRow(row_index))
        .width(Length::Fixed(MENU_WIDTH))
        .padding(Padding::from([7.0, 12.0]))
        .style(menu_item_delete_style)
        .into()
}

fn separator_item<'a>() -> Element<'a, Message> {
    container(horizontal_rule(1))
        .width(Length::Fixed(MENU_WIDTH))
        .padding(Padding::from([2.0, 8.0]))
        .into()
}

fn context_menu_container_style(theme: &iced::Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.base.color.into()),
        border: Border {
            color: p.background.strong.color,
            width: 1.0,
            radius: 4.0.into(),
        },
        shadow: Shadow {
            color: iced::Color::from_rgba(0.0, 0.0, 0.0, 0.35),
            offset: Vector::new(2.0, 4.0),
            blur_radius: 10.0,
        },
        ..Default::default()
    }
}

fn menu_item_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(match status {
            button::Status::Hovered | button::Status::Pressed => p.primary.weak.color.into(),
            _ => p.background.base.color.into(),
        }),
        text_color: p.background.base.text,
        border: Border::default(),
        ..Default::default()
    }
}

fn menu_item_disabled_style(theme: &iced::Theme, _status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(p.background.base.color.into()),
        text_color: p.background.base.text.scale_alpha(0.4),
        border: Border::default(),
        ..Default::default()
    }
}

fn menu_item_delete_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(match status {
            button::Status::Hovered | button::Status::Pressed => p.danger.weak.color.into(),
            _ => p.background.base.color.into(),
        }),
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => p.danger.weak.text,
            _ => p.danger.base.color,
        },
        border: Border::default(),
        ..Default::default()
    }
}
