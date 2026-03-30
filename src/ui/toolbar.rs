use iced::widget::{button, container, pick_list, text, text_input, Row};
use iced::{Alignment, Element, Length, Padding};

use crate::data::{ColumnType, Sheet, SortDirection};
use crate::Message;

pub fn view_toolbar<'a>(
    sheet: &'a Sheet,
    show_add_col: bool,
    new_col_name: &'a str,
) -> Element<'a, Message> {
    let mut toolbar = Row::new()
        .spacing(8)
        .padding(Padding::from([8.0, 12.0]))
        .align_y(Alignment::Center);

    // Add Row
    toolbar = toolbar.push(
        button(text("+ Row").size(13))
            .on_press(Message::AddRow)
            .padding(Padding::from([4.0, 12.0])),
    );

    // Add Column
    if show_add_col {
        toolbar = toolbar.push(
            text_input("Column name", new_col_name)
                .on_input(Message::NewColNameChanged)
                .on_submit(Message::AddColumn(ColumnType::Text))
                .size(13)
                .width(Length::Fixed(120.0)),
        );

        let col_types = vec!["Text", "Number", "Currency", "Formula"];
        for ct in col_types {
            let col_type = match ct {
                "Text" => ColumnType::Text,
                "Number" => ColumnType::Number,
                "Currency" => ColumnType::Currency("$".into()),
                "Formula" => ColumnType::Formula,
                _ => ColumnType::Text,
            };
            toolbar = toolbar.push(
                button(text(ct).size(11))
                    .on_press(Message::AddColumn(col_type))
                    .padding(Padding::from([2.0, 6.0])),
            );
        }

        toolbar = toolbar.push(
            button(text("Cancel").size(11))
                .on_press(Message::ToggleAddColumn)
                .padding(Padding::from([2.0, 6.0])),
        );
    } else {
        toolbar = toolbar.push(
            button(text("+ Column").size(13))
                .on_press(Message::ToggleAddColumn)
                .padding(Padding::from([4.0, 12.0])),
        );
    }

    // Separator
    toolbar = toolbar.push(text("│").size(16));

    // Sort controls
    let col_names: Vec<String> = sheet.columns.iter().map(|c| c.name.clone()).collect();
    let selected_sort = sheet
        .sort
        .as_ref()
        .and_then(|s| col_names.get(s.column).cloned());

    toolbar = toolbar.push(text("Sort:").size(13));
    toolbar = toolbar.push(
        pick_list(col_names.clone(), selected_sort, |name| {
            let idx = sheet
                .columns
                .iter()
                .position(|c| c.name == name)
                .unwrap_or(0);
            Message::SortColumn(idx)
        })
        .placeholder("None")
        .text_size(13)
        .width(Length::Fixed(120.0)),
    );

    if sheet.sort.is_some() {
        let dir_label = match sheet.sort.as_ref().map(|s| s.direction) {
            Some(SortDirection::Ascending) => "Asc",
            Some(SortDirection::Descending) => "Desc",
            None => "Asc",
        };
        toolbar = toolbar.push(
            button(text(dir_label).size(11))
                .on_press(Message::ToggleSortDirection)
                .padding(Padding::from([2.0, 6.0])),
        );
        toolbar = toolbar.push(
            button(text("✕").size(11))
                .on_press(Message::ClearSort)
                .padding(Padding::from([2.0, 6.0])),
        );
    }

    // Separator
    toolbar = toolbar.push(text("│").size(16));

    // Group controls
    let selected_group = sheet
        .group_by
        .and_then(|g| col_names.get(g).cloned());

    toolbar = toolbar.push(text("Group:").size(13));
    toolbar = toolbar.push(
        pick_list(col_names.clone(), selected_group, |name| {
            let idx = sheet
                .columns
                .iter()
                .position(|c| c.name == name)
                .unwrap_or(0);
            Message::GroupByColumn(idx)
        })
        .placeholder("None")
        .text_size(13)
        .width(Length::Fixed(120.0)),
    );

    if sheet.group_by.is_some() {
        toolbar = toolbar.push(
            button(text("✕").size(11))
                .on_press(Message::ClearGroup)
                .padding(Padding::from([2.0, 6.0])),
        );
    }

    container(toolbar)
        .width(Length::Fill)
        .style(toolbar_style)
        .into()
}

fn toolbar_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        border: iced::Border {
            color: palette.background.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
