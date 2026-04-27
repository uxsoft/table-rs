use iced::alignment::Vertical;
use iced::widget::{column, row as iced_row, text};
use iced::{Element, Length};
use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::select::select_sized;
use iced_longbridge::components::sidebar::{Group, Sidebar};
use iced_longbridge::theme::Size;

use crate::data::SortDirection;
use crate::ui::icons::{icon, icon_button, icon_colored, IconKind};
use crate::{Message, TableApp};

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;

    let header: Element<'_, Message> = text("Table RS")
        .size(15.0)
        .color(theme.foreground)
        .into();

    // Sort section
    let sort_extra = sort_section(app);
    let sort_group: Group<'_, Message> = Group::new().label("Sort").extra(sort_extra);

    // Group-by section
    let group_extra = group_section(app);
    let group_group: Group<'_, Message> = Group::new().label("Group by").extra(group_extra);

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
        .push(sort_group)
        .push(group_group)
        .footer(footer)
        .width(280.0)
        .view(theme)
}

fn sort_section(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let options = column_name_options(app);
    let selected = app
        .sheet
        .sort
        .as_ref()
        .and_then(|s| app.sheet.columns.get(s.column).map(|c| c.name.clone()));

    let names = app
        .sheet
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();

    let picker = select_sized(
        theme,
        Size::Sm,
        options,
        selected,
        move |name| {
            let idx = names.iter().position(|n| *n == name);
            Message::SortColumn(idx)
        },
    )
    .width(Length::Fill);

    let mut controls = iced_row![].spacing(4);
    if let Some(ref s) = app.sheet.sort {
        let (kind, label_text) = match s.direction {
            SortDirection::Ascending => (IconKind::ArrowUp, "Asc"),
            SortDirection::Descending => (IconKind::ArrowDown, "Desc"),
        };
        let label_row = iced_row![
            icon(theme, kind, 12.0),
            text(label_text).size(13.0).color(theme.foreground),
        ]
        .spacing(6)
        .align_y(Vertical::Center);
        controls = controls.push(icon_button(
            theme,
            label_row,
            Variant::Outline,
            Size::Sm,
            Some(Message::ToggleSortDirection),
            false,
        ));
        controls = controls.push(button_ex(
            theme,
            "Clear",
            Variant::Ghost,
            Size::Sm,
            Some(Message::SortColumn(None)),
            false,
            false,
        ));
    }

    column![picker, controls]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn group_section(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let options = column_name_options(app);
    let selected = app
        .sheet
        .group_by
        .and_then(|i| app.sheet.columns.get(i).map(|c| c.name.clone()));

    let names = app
        .sheet
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect::<Vec<_>>();

    let picker = select_sized(
        theme,
        Size::Sm,
        options,
        selected,
        move |name| {
            let idx = names.iter().position(|n| *n == name);
            Message::GroupByColumn(idx)
        },
    )
    .width(Length::Fill);

    let mut controls = iced_row![].spacing(4);
    if app.sheet.group_by.is_some() {
        controls = controls.push(button_ex(
            theme,
            "Clear",
            Variant::Ghost,
            Size::Sm,
            Some(Message::GroupByColumn(None)),
            false,
            false,
        ));
    }

    column![picker, controls]
        .spacing(6)
        .width(Length::Fill)
        .into()
}

fn column_name_options(app: &TableApp) -> Vec<String> {
    app.sheet
        .columns
        .iter()
        .map(|c| c.name.clone())
        .collect()
}
