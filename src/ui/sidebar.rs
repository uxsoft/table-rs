use iced::alignment::Vertical;
use iced::widget::{row as iced_row, text};
use iced::Element;
use iced_longbridge::components::sidebar::Sidebar;

use crate::ui::icons::{icon_colored, IconKind};
use crate::{Message, TableApp};

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;

    let header: Element<'_, Message> = text("Table RS")
        .size(15.0)
        .color(theme.foreground)
        .into();

    let footer: Element<'_, Message> = {
        let path = app
            .sheet
            .file_path
            .as_ref()
            .and_then(|p| p.file_name())
            .and_then(|s| s.to_str())
            .unwrap_or("(unsaved)");
        iced_row![
            text(format!("{path} · {} rows", app.sheet.row_count()))
                .size(11.0)
                .color(theme.muted_foreground),
            icon_colored(IconKind::Close, 10.0, theme.muted_foreground),
            text(format!("{} cols", app.sheet.col_count()))
                .size(11.0)
                .color(theme.muted_foreground),
        ]
        .spacing(4)
        .align_y(Vertical::Center)
        .into()
    };

    Sidebar::new()
        .header(header)
        .footer(footer)
        .width(280.0)
        .view(theme)
}
