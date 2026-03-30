use iced::widget::{button, container, mouse_area, row, text, text_input, Column, Row, Scrollable};
use iced::{Alignment, Element, Length, Padding};

use crate::data::{ColumnType, Group, Sheet};
use crate::Message;

const ROW_HEIGHT: f32 = 32.0;
const HEADER_HEIGHT: f32 = 40.0;
const ROW_NUMBER_WIDTH: f32 = 32.0;

fn cell_padding() -> Padding {
    Padding::from([4.0, 8.0])
}

pub fn view_table<'a>(
    sheet: &'a Sheet,
    editing: Option<(usize, usize)>,
    edit_value: &'a str,
    groups: &'a Option<Vec<Group>>,
) -> Element<'a, Message> {
    let header = view_header(sheet);

    let body: Element<'a, Message> = if let Some(groups) = groups {
        view_grouped_body(sheet, groups, editing, edit_value)
    } else {
        view_flat_body(sheet, editing, edit_value)
    };

    let scrollable_body = Scrollable::new(body).height(Length::Fill);

    Column::new()
        .push(header)
        .push(scrollable_body)
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
}

fn view_header<'a>(sheet: &'a Sheet) -> Element<'a, Message> {
    let mut header_row = Row::new()
        .height(Length::Fixed(HEADER_HEIGHT))
        .align_y(Alignment::Center);

    // Row number column
    header_row = header_row.push(
        container(text("#").size(12))
            .width(Length::Fixed(ROW_NUMBER_WIDTH))
            .padding(cell_padding())
            .style(header_cell_style),
    );

    for (i, col) in sheet.columns.iter().enumerate() {
        let sort_indicator = match &sheet.sort {
            Some(s) if s.column == i => match s.direction {
                crate::data::SortDirection::Ascending => " ▲",
                crate::data::SortDirection::Descending => " ▼",
            },
            _ => "",
        };

        let type_badge = match &col.col_type {
            ColumnType::Text => "Aa",
            ColumnType::Number => "#",
            ColumnType::Currency(_) => "$",
            ColumnType::Formula => "fx",
        };

        // For formula columns, show the expression in the header (like Airtable).
        let label = if col.col_type == ColumnType::Formula {
            if let Some(ref expr) = col.formula {
                format!("fx {} = {}{}", col.name, expr, sort_indicator)
            } else {
                format!("fx {} (click to set formula){}", col.name, sort_indicator)
            }
        } else {
            format!("{} {}{}", type_badge, col.name, sort_indicator)
        };

        let header_cell = button(text(label).size(13))
            .on_press(Message::HeaderClicked(i))
            .padding(cell_padding())
            .width(Length::Fixed(col.width))
            .style(header_button_style);

        header_row = header_row.push(header_cell);
    }

    container(header_row)
        .style(header_bar_style)
        .width(Length::Fill)
        .into()
}

fn view_flat_body<'a>(
    sheet: &'a Sheet,
    editing: Option<(usize, usize)>,
    edit_value: &'a str,
) -> Element<'a, Message> {
    let mut col = Column::new().spacing(0);
    for r in 0..sheet.rows.len() {
        col = col.push(view_data_row(sheet, r, editing, edit_value, r % 2 == 1));
    }
    col = col.push(view_add_row(sheet));
    col.width(Length::Fill).into()
}

fn view_grouped_body<'a>(
    sheet: &'a Sheet,
    groups: &'a [Group],
    editing: Option<(usize, usize)>,
    edit_value: &'a str,
) -> Element<'a, Message> {
    let mut col = Column::new().spacing(0);

    for (gi, group) in groups.iter().enumerate() {
        // Group header
        let count_label = format!("{} ({} rows)", group.key, group.row_indices.len());
        let collapse_label = if group.collapsed { "▶" } else { "▼" };

        let group_header = button(
            row![
                text(collapse_label).size(12),
                text(count_label).size(13),
            ]
            .spacing(6)
            .align_y(Alignment::Center),
        )
        .on_press(Message::ToggleGroup(gi))
        .padding(Padding::from([6.0, 12.0]))
        .width(Length::Fill)
        .style(group_header_style);

        col = col.push(group_header);

        if !group.collapsed {
            for (i, &ri) in group.row_indices.iter().enumerate() {
                col = col.push(view_data_row(sheet, ri, editing, edit_value, i % 2 == 1));
            }
        }
    }
    col = col.push(view_add_row(sheet));

    col.width(Length::Fill).into()
}

fn view_data_row<'a>(
    sheet: &'a Sheet,
    row_index: usize,
    editing: Option<(usize, usize)>,
    edit_value: &'a str,
    alternate: bool,
) -> Element<'a, Message> {
    let row_data = &sheet.rows[row_index];
    let mut data_row = Row::new()
        .height(Length::Fixed(ROW_HEIGHT))
        .align_y(Alignment::Center);

    // Row number — right-click opens context menu
    let row_num_cell = container(text(format!("{}", row_index)).size(11))
        .width(Length::Fixed(ROW_NUMBER_WIDTH))
        .height(ROW_HEIGHT)
        .padding(cell_padding())
        .style(row_number_style);

    data_row = data_row.push(
        mouse_area(row_num_cell)
            .on_right_press(Message::RowRightClicked(row_index)),
    );

    for (c, cell) in row_data.iter().enumerate() {
        let col_def = &sheet.columns[c];
        let width = Length::Fixed(col_def.width);

        let cell_element: Element<'a, Message> =
            if editing == Some((row_index, c)) {
                let input_id = format!("cell_{}_{}", row_index, c);
                text_input("", edit_value)
                    .id(text_input::Id::new(input_id))
                    .on_input(move |v| Message::CellEdited(row_index, c, v))
                    .on_submit(Message::CellEditCommit)
                    .size(13)
                    .padding(cell_padding())
                    .width(width)
                    .into()
            } else {
                let currency_sym = match &col_def.col_type {
                    ColumnType::Currency(s) => s.as_str(),
                    _ => "$",
                };
                let display = cell.display_value(currency_sym);
                let txt = text(display).size(13);

                // Formula columns are read-only: no on_press handler.
                let is_formula_col = col_def.col_type == ColumnType::Formula
                    && col_def.formula.is_some();

                let cell_container = if is_formula_col {
                    button(txt)
                        .padding(cell_padding())
                        .width(width)
                        .height(ROW_HEIGHT)
                        .style(formula_cell_style)
                } else {
                    button(txt)
                        .on_press(Message::CellClicked(row_index, c))
                        .padding(cell_padding())
                        .width(width)
                        .height(ROW_HEIGHT)
                        .style(if alternate {
                            data_cell_alt_style
                        } else {
                            data_cell_style
                        })
                };

                cell_container.into()
            };

        data_row = data_row.push(cell_element);
    }

    container(data_row)
        .width(Length::Fill)
        .into()
}

fn view_add_row<'a>(sheet: &'a Sheet) -> Element<'a, Message> {
    let total_width: f32 =
        ROW_NUMBER_WIDTH + sheet.columns.iter().map(|c| c.width).sum::<f32>();

    let add_btn = button(
        container(text("+ Add row").size(12))
            .width(Length::Fixed(total_width))
            .padding(cell_padding()),
    )
    .on_press(Message::AddRow)
    .height(Length::Fixed(ROW_HEIGHT))
    .width(Length::Fixed(total_width))
    .style(add_row_style);

    container(add_btn).width(Length::Fill).into()
}

// -- Styles --

fn header_bar_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.primary.weak.color.into()),
        ..Default::default()
    }
}

fn header_cell_style(_theme: &iced::Theme) -> container::Style {
    container::Style {
        ..Default::default()
    }
}

fn header_button_style(theme: &iced::Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(palette.primary.weak.color.into()),
        text_color: palette.primary.weak.text,
        border: iced::Border {
            color: palette.primary.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn data_cell_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => palette.background.weak.color,
        _ => palette.background.base.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: palette.background.base.text,
        border: iced::Border {
            color: palette.background.weak.color,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn formula_cell_style(theme: &iced::Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    // Slightly tinted background to indicate read-only computed value.
    button::Style {
        background: Some(
            iced::Color {
                r: palette.primary.weak.color.r,
                g: palette.primary.weak.color.g,
                b: palette.primary.weak.color.b,
                a: 0.12,
            }
            .into(),
        ),
        text_color: palette.background.base.text,
        border: iced::Border {
            color: palette.background.weak.color,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn data_cell_alt_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => palette.background.weak.color,
        _ => palette.background.weak.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: palette.background.weak.text,
        border: iced::Border {
            color: palette.background.weak.color,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn group_header_style(theme: &iced::Theme, _status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    button::Style {
        background: Some(palette.primary.weak.color.into()),
        text_color: palette.primary.weak.text,
        border: iced::Border {
            color: palette.primary.strong.color,
            width: 0.0,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}

fn row_number_style(theme: &iced::Theme) -> container::Style {
    let palette = theme.extended_palette();
    container::Style {
        background: Some(palette.background.weak.color.into()),
        ..Default::default()
    }
}

fn add_row_style(theme: &iced::Theme, status: button::Status) -> button::Style {
    let palette = theme.extended_palette();
    let bg = match status {
        button::Status::Hovered => palette.background.weak.color,
        _ => palette.background.base.color,
    };
    button::Style {
        background: Some(bg.into()),
        text_color: palette.background.base.text.scale_alpha(0.5),
        border: iced::Border {
            color: palette.background.weak.color,
            width: 0.5,
            radius: 0.0.into(),
        },
        ..Default::default()
    }
}
