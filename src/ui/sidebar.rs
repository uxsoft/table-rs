use iced::widget::{column, row as iced_row, text};
use iced::{Element, Length};
use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::input::input_sized;
use iced_longbridge::components::select::select_sized;
use iced_longbridge::components::sidebar::{Group, Sidebar};
use iced_longbridge::theme::Size;

use crate::data::{ColumnType, SortDirection};
use crate::{Message, TableApp};

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;

    let header: Element<'_, Message> = text("Table RS")
        .size(15.0)
        .color(theme.foreground)
        .into();

    // Columns section: list each column with a type select.
    let columns_extra = columns_section(app);
    let columns_group: Group<'_, Message> = Group::new()
        .label("Columns")
        .extra(columns_extra);

    // Add-column section: name input, type select buttons.
    let add_col_extra = add_column_section(app);
    let add_col_group: Group<'_, Message> = Group::new()
        .label("Add column")
        .extra(add_col_extra);

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
        let info = format!(
            "{} · {} rows × {} cols",
            path,
            app.sheet.row_count(),
            app.sheet.col_count()
        );
        text(info).size(11.0).color(theme.muted_foreground).into()
    };

    Sidebar::new()
        .header(header)
        .push(columns_group)
        .push(add_col_group)
        .push(sort_group)
        .push(group_group)
        .footer(footer)
        .width(280.0)
        .view(theme)
}

fn columns_section(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let mut col = column![].spacing(6).width(Length::Fill);
    if app.sheet.columns.is_empty() {
        return text("No columns")
            .size(12.0)
            .color(theme.muted_foreground)
            .into();
    }
    for (idx, c) in app.sheet.columns.iter().enumerate() {
        let name = text(c.name.clone())
            .size(12.0)
            .color(theme.foreground)
            .width(Length::Fill);
        let type_select = select_sized(
            theme,
            Size::Sm,
            column_type_options(),
            Some(label_for_type(&c.col_type)),
            move |label| Message::ColumnTypeChanged(idx, type_from_label(&label)),
        )
        .width(Length::Fixed(130.0));
        col = col.push(
            iced_row![name, type_select]
                .spacing(6)
                .width(Length::Fill),
        );
    }
    col.into()
}

fn add_column_section(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let name_input = input_sized(theme, Size::Sm, "Column name", &app.new_col_name)
        .on_input(Message::NewColNameChanged);

    let type_picker = select_sized(
        theme,
        Size::Sm,
        column_type_options(),
        Some(label_for_type(&app.new_col_type)),
        |label| Message::NewColTypeChanged(type_from_label(&label)),
    )
    .width(Length::Fill);

    let add_btn = button_ex(
        theme,
        "Add column",
        Variant::Secondary,
        Size::Sm,
        Some(Message::AddColumn(app.new_col_type.clone())),
        false,
        false,
    );

    column![name_input, type_picker, add_btn]
        .spacing(6)
        .width(Length::Fill)
        .into()
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
        let label = match s.direction {
            SortDirection::Ascending => "▲ Asc",
            SortDirection::Descending => "▼ Desc",
        };
        controls = controls.push(button_ex(
            theme,
            label,
            Variant::Outline,
            Size::Sm,
            Some(Message::ToggleSortDirection),
            false,
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

fn column_type_options() -> Vec<String> {
    vec![
        "T  Text".to_string(),
        "#  Number".to_string(),
        "$  Currency".to_string(),
        "ƒ  Formula".to_string(),
    ]
}

fn label_for_type(t: &ColumnType) -> String {
    match t {
        ColumnType::Text => "T  Text".to_string(),
        ColumnType::Number => "#  Number".to_string(),
        ColumnType::Currency(_) => "$  Currency".to_string(),
        ColumnType::Formula => "ƒ  Formula".to_string(),
    }
}

fn type_from_label(s: &str) -> ColumnType {
    match s {
        "#  Number" => ColumnType::Number,
        "$  Currency" => ColumnType::Currency("$".to_string()),
        "ƒ  Formula" => ColumnType::Formula,
        _ => ColumnType::Text,
    }
}
