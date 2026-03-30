use iced::widget::{button, column, container, text};
use iced::{Border, Element, Length, Padding, Shadow, Vector};

use crate::Message;

const DROPDOWN_WIDTH: f32 = 150.0;

pub fn view_menubar<'a>(open_menu: Option<&str>) -> Element<'a, Message> {
    let is_file_open = open_menu == Some("file");
    let file_label = if is_file_open { "File ▲" } else { "File ▼" };

    let file_btn = button(text(file_label).size(13))
        .on_press(if is_file_open {
            Message::CloseMenu
        } else {
            Message::OpenMenu("file".to_string())
        })
        .padding(Padding::from([4.0, 12.0]))
        .style(menu_button_style);

    let bar_row = iced::widget::row![file_btn]
        .padding(Padding::from([2.0, 4.0]))
        .align_y(iced::Alignment::Center);

    container(bar_row)
        .width(Length::Fill)
        .style(menubar_bg_style)
        .into()
}

pub fn view_file_dropdown<'a>() -> Element<'a, Message> {
    let items = column![
        menu_item("Open        Ctrl+O", Message::FileOpen),
        menu_item("Save        Ctrl+S", Message::FileSave),
    ]
    .spacing(0)
    .width(Length::Fixed(DROPDOWN_WIDTH));

    container(items)
        .style(dropdown_container_style)
        .into()
}

fn menu_item<'a>(label: &'a str, msg: Message) -> Element<'a, Message> {
    button(text(label).size(13))
        .on_press(msg)
        .width(Length::Fixed(DROPDOWN_WIDTH))
        .padding(Padding::from([7.0, 12.0]))
        .style(menu_item_style)
        .into()
}

// --- Styles ---

fn menubar_bg_style(theme: &iced::Theme) -> container::Style {
    let p = theme.extended_palette();
    container::Style {
        background: Some(p.background.weak.color.into()),
        border: Border {
            color: p.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn dropdown_container_style(theme: &iced::Theme) -> container::Style {
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

fn menu_button_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(match status {
            button::Status::Hovered | button::Status::Pressed => {
                p.background.strong.color.into()
            }
            _ => p.background.weak.color.into(),
        }),
        text_color: p.background.base.text,
        border: Border::default(),
        ..Default::default()
    }
}

fn menu_item_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let p = theme.extended_palette();
    button::Style {
        background: Some(match status {
            button::Status::Hovered | button::Status::Pressed => {
                p.primary.weak.color.into()
            }
            _ => iced::Color::TRANSPARENT.into(),
        }),
        text_color: match status {
            button::Status::Hovered | button::Status::Pressed => p.primary.weak.text,
            _ => p.background.base.text,
        },
        border: Border::default(),
        ..Default::default()
    }
}

pub fn transparent_btn_style(_theme: &iced::Theme, _status: button::Status) -> button::Style {
    button::Style {
        background: None,
        shadow: Shadow::default(),
        ..Default::default()
    }
}
