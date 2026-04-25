use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row as iced_row, text, text_input, Space};
use iced::{Background, Border, Element, Length, Padding, Shadow};

use iced_longbridge::components::button::Variant;
use iced_longbridge::components::collapsible::collapsible;
use iced_longbridge::components::context_menu::ContextMenu;
use iced_longbridge::components::menu::{menu, Item as MenuItem};
use iced_longbridge::components::popover::popover_dismissable;
use iced_longbridge::components::table::{
    table_with, Column, ResizeHandlers, SortDir, TableOptions,
};
use iced_longbridge::theme::{AppTheme, Size};

use crate::data::{CellValue, ColumnType, SortDirection};
use crate::ui::icons::{icon, icon_button, icon_colored, IconKind};
use crate::{sort_key_for, Message, TableApp, MAX_SORT_COLS};

type RowRef<'a> = (usize, &'a [CellValue]);

pub fn view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;

    let body: Element<'_, Message> = if app.groups.is_some() {
        grouped_view(app)
    } else {
        flat_view(app)
    };

    let add_row_btn = icon_button(
        theme,
        iced_row![
            icon(theme, IconKind::Plus, 12.0),
            text("Add row").size(13.0).color(theme.foreground),
        ]
        .spacing(6)
        .align_y(Vertical::Center),
        Variant::Ghost,
        Size::Sm,
        Some(Message::AddRow),
        false,
    );

    let add_row_container = container(add_row_btn)
        .padding(Padding::from([6.0, 12.0]))
        .width(Length::Fill);

    container(
        column![body, add_row_container]
            .spacing(6)
            .width(Length::Fill),
    )
    .padding(Padding::from([8.0, 12.0]))
    .width(Length::Fill)
    .height(Length::Fill)
    .into()
}

fn flat_view(app: &TableApp) -> Element<'_, Message> {
    let rows: Vec<RowRef<'_>> = app
        .sheet
        .rows
        .iter()
        .enumerate()
        .map(|(i, r)| (i, r.as_slice()))
        .collect();
    let columns = build_columns(app);
    let options = build_table_options(app, true);
    let tbl = table_with(&app.theme, &rows, columns, options);
    // `table_with` returns an Element we can return directly; put it in a
    // container so the outer column doesn't stretch unboundedly.
    container(tbl).width(Length::Fill).into()
}

fn grouped_view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let groups = app.groups.as_ref().expect("grouped_view called without groups");
    let mut col = column![].spacing(6).width(Length::Fill);
    for (gi, g) in groups.iter().enumerate() {
        let group_rows: Vec<RowRef<'_>> = g
            .row_indices
            .iter()
            .filter_map(|&ri| app.sheet.rows.get(ri).map(|r| (ri, r.as_slice())))
            .collect();

        // Only the first group's table drives resize/sort; avoids duplicate
        // mouse catchers and lets the user still sort/resize from the top-most
        // header. Sort/group via sidebar remains available on all groups.
        let is_primary = gi == 0;
        let columns = build_columns(app);
        let options = build_table_options(app, is_primary);
        let label = format!("{}  ({})", g.key, g.row_indices.len());

        let inner: Element<'_, Message> = if g.collapsed {
            Space::new().height(Length::Fixed(0.0)).into()
        } else {
            table_with(theme, &group_rows, columns, options)
        };

        col = col.push(collapsible(
            theme,
            label,
            !g.collapsed,
            Message::ToggleGroup(gi),
            inner,
        ));
    }
    col.into()
}

fn build_table_options<'a>(
    app: &'a TableApp,
    wire_interactions: bool,
) -> TableOptions<'a, Message> {
    let sort = app.sheet.sort.as_ref().and_then(|s| {
        sort_key_for(s.column).map(|k| {
            let dir = match s.direction {
                SortDirection::Ascending => SortDir::Asc,
                SortDirection::Descending => SortDir::Desc,
            };
            (k, dir)
        })
    });

    TableOptions {
        sort,
        on_sort: if wire_interactions {
            Some(Box::new(Message::TableSort))
        } else {
            None
        },
        striped: true,
        row_height: 34.0,
        resize: if wire_interactions {
            Some(ResizeHandlers {
                on_grab: Box::new(Message::TableResizeStart),
                on_drag: Box::new(Message::TableResizeMove),
                on_release: Message::TableResizeEnd,
                dragging: app.table_resize_col,
            })
        } else {
            None
        },
    }
}

fn build_columns<'a>(app: &'a TableApp) -> Vec<Column<'a, RowRef<'a>, Message>> {
    let theme = app.theme;
    let mut cols: Vec<Column<'a, RowRef<'a>, Message>> = Vec::new();
    let clipboard_cell_some = app.clipboard_cell.is_some();
    let clipboard_row_some = app.clipboard_row.is_some();
    let only_one_column = app.sheet.col_count() <= 1;

    for (ci, def) in app.sheet.columns.iter().enumerate() {
        let col_width = Length::Fixed(def.width);
        let col_type = def.col_type.clone();
        let has_formula = def.formula.is_some();
        let header = def.name.clone();
        let is_numeric = matches!(
            def.col_type,
            ColumnType::Number | ColumnType::Currency(_) | ColumnType::Formula
        );
        let currency_symbol = match &def.col_type {
            ColumnType::Currency(sym) => sym.clone(),
            _ => "$".to_string(),
        };
        let t = theme;
        let editing = app.editing;
        let selected = app.selected_cell;
        let edit_value = app.edit_value.as_str();

        let render = move |rr: &RowRef<'a>| -> Element<'a, Message> {
            let (row_idx, cells) = *rr;
            let cell = cells.get(ci).cloned().unwrap_or(CellValue::Empty);
            render_cell(
                &t,
                row_idx,
                ci,
                &cell,
                &col_type,
                has_formula,
                &currency_symbol,
                editing,
                selected,
                edit_value,
                clipboard_cell_some,
                clipboard_row_some,
                only_one_column,
            )
        };

        let mut c = Column::new(header, render)
            .width(col_width)
            .align(if is_numeric {
                Horizontal::Right
            } else {
                Horizontal::Left
            });

        // Only register sortable keys for columns within the reserved range.
        if ci < MAX_SORT_COLS {
            if let Some(key) = sort_key_for(ci) {
                c = c.sortable(key);
            }
        }

        cols.push(c);
    }

    // Trailing kebab (row menu) column.
    let row_menu_open = app.row_menu_open;
    let clipboard_has_value = app.clipboard_row.is_some();
    let t = theme;
    let kebab_col = Column::new("", move |rr: &RowRef<'a>| {
        let (row_idx, _) = *rr;
        render_row_menu(&t, row_idx, row_menu_open, clipboard_has_value)
    })
    .width(Length::Fixed(44.0))
    .align(Horizontal::Center);
    cols.push(kebab_col);

    cols
}

#[allow(clippy::too_many_arguments)]
fn render_cell<'a>(
    theme: &AppTheme,
    row_idx: usize,
    col_idx: usize,
    cell: &CellValue,
    col_type: &ColumnType,
    has_formula: bool,
    currency_symbol: &str,
    editing: Option<(usize, usize)>,
    selected: Option<(usize, usize)>,
    edit_value: &'a str,
    clipboard_cell_some: bool,
    clipboard_row_some: bool,
    only_one_column: bool,
) -> Element<'a, Message> {
    let t = *theme;
    let is_selected = selected == Some((row_idx, col_idx)) && editing != Some((row_idx, col_idx));
    let is_formula = has_formula || matches!(col_type, ColumnType::Formula);

    // Inline editor for the focused cell — no context menu so the text input
    // owns its native interactions.
    if editing == Some((row_idx, col_idx)) {
        return text_input("", edit_value)
            .on_input(Message::CellEdited)
            .on_submit(Message::CellEditSubmit)
            .size(13.0)
            .padding(Padding::from([2.0, 6.0]))
            .style(move |_, status| cell_input_style(&t, status))
            .into();
    }

    let child: Element<'a, Message> = if is_formula {
        // Formula cells are read-only.
        let display = cell.display_value(currency_symbol);
        container(
            text(display)
                .size(13.0)
                .color(t.muted_foreground),
        )
        .padding(Padding::from([0.0, 4.0]))
        .width(Length::Fill)
        .into()
    } else {
        let display = cell.display_value(currency_symbol);
        let body: Element<'a, Message> = if display.is_empty() {
            container(icon_colored(IconKind::Dot, 12.0, t.muted_foreground))
                .height(Length::Fixed(18.0))
                .align_y(Vertical::Center)
                .into()
        } else {
            text(display).size(13.0).color(t.foreground).into()
        };

        button(body)
            .padding(Padding::from([2.0, 6.0]))
            .width(Length::Fill)
            .on_press(Message::CellClicked(row_idx, col_idx))
            .style(move |_, status| cell_button_style(&t, status, is_selected))
            .into()
    };

    let items = cell_context_items(
        row_idx,
        col_idx,
        is_formula,
        clipboard_cell_some,
        clipboard_row_some,
        only_one_column,
    );
    ContextMenu::new(child, items).view(theme)
}

fn cell_context_items(
    row: usize,
    col: usize,
    is_formula: bool,
    clipboard_cell_some: bool,
    clipboard_row_some: bool,
    only_one_column: bool,
) -> Vec<MenuItem<Message>> {
    let mut items: Vec<MenuItem<Message>> = Vec::new();

    let mut cut = MenuItem::new("Cut", Message::CutCell(row, col)).shortcut("⌘X");
    if is_formula {
        cut = cut.disabled();
    }
    items.push(cut);

    items.push(MenuItem::new("Copy", Message::CopyCell(row, col)).shortcut("⌘C"));

    let mut paste = MenuItem::new("Paste", Message::PasteCell(row, col)).shortcut("⌘V");
    if !clipboard_cell_some || is_formula {
        paste = paste.disabled();
    }
    items.push(paste);

    items.push(MenuItem::Separator);

    let mut clear = MenuItem::new("Clear contents", Message::ClearCell(row, col))
        .shortcut("Del");
    if is_formula {
        clear = clear.disabled();
    }
    items.push(clear);

    items.push(MenuItem::Separator);

    items.push(MenuItem::new(
        "Insert row above",
        Message::InsertRowAbove(row),
    ));
    items.push(MenuItem::new(
        "Insert row below",
        Message::InsertRowBelow(row),
    ));

    let mut paste_row = MenuItem::new("Paste row", Message::PasteRow(row)).shortcut("⇧⌘V");
    if !clipboard_row_some {
        paste_row = paste_row.disabled();
    }
    items.push(paste_row);

    items.push(MenuItem::new("Delete row", Message::DeleteRow(row)).danger());

    items.push(MenuItem::Separator);

    items.push(MenuItem::new(
        "Insert column left",
        Message::InsertColumnLeft(col),
    ));
    items.push(MenuItem::new(
        "Insert column right",
        Message::InsertColumnRight(col),
    ));

    let mut delete_col = MenuItem::new("Delete column", Message::DeleteColumn(col)).danger();
    if only_one_column {
        delete_col = delete_col.disabled();
    }
    items.push(delete_col);

    items
}

fn render_row_menu<'a>(
    theme: &AppTheme,
    row_idx: usize,
    row_menu_open: Option<usize>,
    clipboard_has_value: bool,
) -> Element<'a, Message> {
    let t = *theme;
    let is_open = row_menu_open == Some(row_idx);

    let trigger: Element<'_, Message> = button(
        container(icon_colored(
            IconKind::EllipsisVertical,
            16.0,
            t.muted_foreground,
        ))
        .width(Length::Fill)
        .align_x(Horizontal::Center),
    )
    .padding(Padding::from([2.0, 8.0]))
    .width(Length::Fixed(32.0))
    .on_press(Message::RowMenuToggle(Some(row_idx)))
    .style(move |_, status| cell_button_style(&t, status, false))
    .into();

    let panel = is_open.then(|| {
        let mut items: Vec<MenuItem<Message>> = vec![
            MenuItem::new("Cut", Message::CutRow(row_idx)).shortcut("⌘X"),
            MenuItem::new("Copy", Message::CopyRow(row_idx)).shortcut("⌘C"),
        ];
        let mut paste = MenuItem::new("Paste", Message::PasteRow(row_idx)).shortcut("⌘V");
        if !clipboard_has_value {
            paste = paste.disabled();
        }
        items.push(paste);
        items.push(MenuItem::Separator);
        items.push(MenuItem::new("Delete row", Message::DeleteRow(row_idx)).danger());
        menu(theme, items)
    });

    popover_dismissable(theme, trigger, panel, Message::RowMenuToggle(None))
}

fn cell_input_style(t: &AppTheme, status: iced::widget::text_input::Status) -> iced::widget::text_input::Style {
    use iced::widget::text_input::Status::*;
    let (border_color, border_width) = match status {
        Focused { .. } => (t.ring, 2.0),
        _ => (t.primary, 1.0),
    };
    iced::widget::text_input::Style {
        background: Background::Color(t.background),
        border: Border {
            color: border_color,
            width: border_width,
            radius: 3.0.into(),
        },
        icon: t.muted_foreground,
        placeholder: t.muted_foreground,
        value: t.foreground,
        selection: iced_longbridge::theme::with_alpha(t.primary, 0.3),
    }
}

fn cell_button_style(
    t: &AppTheme,
    status: iced::widget::button::Status,
    is_selected: bool,
) -> iced::widget::button::Style {
    use iced::widget::button::Status::*;
    let bg = match status {
        Hovered => Some(Background::Color(iced_longbridge::theme::with_alpha(
            t.accent, 0.6,
        ))),
        Pressed => Some(Background::Color(t.accent)),
        _ if is_selected => Some(Background::Color(iced_longbridge::theme::with_alpha(
            t.accent, 0.4,
        ))),
        _ => None,
    };
    let (border_color, border_width) = if is_selected {
        (t.primary, 1.5)
    } else {
        (iced::Color::TRANSPARENT, 0.0)
    };
    iced::widget::button::Style {
        background: bg,
        text_color: t.foreground,
        border: Border {
            color: border_color,
            width: border_width,
            radius: 3.0.into(),
        },
        shadow: Shadow::default(),
        snap: true,
    }
}
