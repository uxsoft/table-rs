use iced::alignment::{Horizontal, Vertical};
use iced::widget::{button, column, container, row as iced_row, text, text_input, Space};
use iced::{Background, Border, Element, Length, Padding, Shadow};

use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::collapsible::collapsible;
use iced_longbridge::components::context_menu::ContextMenu;
use iced_longbridge::components::hover_card::hover_card;
use iced_longbridge::components::input::input_sized;
use iced_longbridge::components::menu::{menu, Item as MenuItem};
use iced_longbridge::components::popover::popover_dismissable;
use iced_longbridge::components::select::select_sized;
use iced_longbridge::components::switch::switch;
use iced_longbridge::components::table::{table, Column, SortDir};
use iced_longbridge::theme::{AppTheme, Size};

use crate::data::{CellValue, ColumnType, NumberFormat, SortDirection};
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

    let add_row_container = container(iced_row![add_row_btn].spacing(8).align_y(Vertical::Center))
        .padding(Padding::from([6.0, 12.0]))
        .width(Length::Fill);

    container(
        column![body, add_row_container]
            .spacing(6)
            .width(Length::Fill),
    )
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
    let tbl: Element<'_, Message> = build_table(app, &rows, columns, true);
    // Container so the outer column doesn't stretch unboundedly.
    container(tbl).width(Length::Fill).into()
}

fn grouped_view(app: &TableApp) -> Element<'_, Message> {
    let theme = &app.theme;
    let groups = app
        .groups
        .as_ref()
        .expect("grouped_view called without groups");
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
        let label = format!("{}  ({})", g.key, g.row_indices.len());

        let inner: Element<'_, Message> = if g.collapsed {
            Space::new().height(Length::Fixed(0.0)).into()
        } else {
            build_table(app, &group_rows, columns, is_primary)
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

fn build_table<'a>(
    app: &'a TableApp,
    rows: &[RowRef<'a>],
    columns: Vec<Column<'a, RowRef<'a>, Message>>,
    wire_interactions: bool,
) -> Element<'a, Message> {
    let sort = app.sheet.sort.as_ref().and_then(|s| {
        sort_key_for(s.column).map(|k| {
            let dir = match s.direction {
                SortDirection::Ascending => SortDir::Asc,
                SortDirection::Descending => SortDir::Desc,
            };
            (k, dir)
        })
    });

    let mut tbl = table(&app.theme, rows, columns)
        .striped(true)
        .row_height(34.0);

    if wire_interactions {
        tbl = tbl.sort(sort, Message::TableSort);
        tbl = tbl.resize(&app.table_resize, Message::TableResize);
    }

    tbl.into()
}

fn build_columns<'a>(app: &'a TableApp) -> Vec<Column<'a, RowRef<'a>, Message>> {
    let theme = app.theme;
    let mut cols: Vec<Column<'a, RowRef<'a>, Message>> = Vec::new();
    let clipboard_cell_some = app.clipboard_cell.is_some();
    let clipboard_row_some = app.clipboard_row.is_some();
    let only_one_column = app.sheet.col_count() <= 1;

    for (ci, def) in app.sheet.columns.iter().enumerate() {
        let col_width = app.table_resize.width(ci);
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
        let render = move |rr: &RowRef<'a>| -> Element<'a, Message> {
            let (row_idx, cells) = *rr;
            let cell = cells.get(ci).cloned().unwrap_or(CellValue::Empty);
            render_cell(
                &theme,
                row_idx,
                ci,
                &cell,
                &col_type,
                has_formula,
                &currency_symbol,
                &def.format.clone(),
                app.editing,
                app.selected_cell,
                app.edit_value.as_str(),
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

        c = c
            .header_button(IconKind::Settings, Message::ColumnSettingsToggle(Some(ci)))
            .header_button_tooltip("Column settings");
        if app.column_settings_open == Some(ci) {
            c = c.header_panel(build_column_settings_panel(app, ci));
        }

        cols.push(c);
    }

    // Trailing "+" column — header button creates a new Text column. Body
    // cells are empty (they only exist because every column needs a renderer).
    let add_col = Column::new("", move |_rr: &RowRef<'a>| -> Element<'a, Message> {
        Space::new().width(Length::Fill).into()
    })
    // .width(Length::Fixed(36.0))
    .align(Horizontal::Center)
    .header_button(IconKind::Plus, Message::AddColumn(ColumnType::Text))
    .header_button_tooltip("Add column");
    cols.push(add_col);

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

const FORMULA_SYNTAX_HELP: &str = "Formula syntax\n\n\
    {ColumnName}   value of that column in this row\n\
    + - * /        arithmetic\n\
    ( )            grouping\n\
\n\
Examples\n\
    {Price} * {Quantity}\n\
    ({Value} + 10) / 2";

fn type_options() -> Vec<String> {
    vec![
        "Text".to_string(),
        "Number".to_string(),
        "Currency".to_string(),
        "Formula".to_string(),
    ]
}

fn type_label(t: &ColumnType) -> String {
    match t {
        ColumnType::Text => "Text",
        ColumnType::Number => "Number",
        ColumnType::Currency(_) => "Currency",
        ColumnType::Formula => "Formula",
    }
    .to_string()
}

fn precision_options() -> Vec<String> {
    (0..=6u8).map(|n| n.to_string()).collect()
}

fn build_column_settings_panel<'a>(app: &'a TableApp, ci: usize) -> Element<'a, Message> {
    let theme = &app.theme;
    let t = *theme;
    let def = match app.sheet.columns.get(ci) {
        Some(d) => d,
        None => {
            return Space::new()
                .width(Length::Shrink)
                .height(Length::Shrink)
                .into()
        }
    };

    let panel_width = Length::Fixed(280.0);

    let section_label = |label: &str| -> Element<'a, Message> {
        text(label.to_string())
            .size(11.0)
            .color(t.muted_foreground)
            .into()
    };

    // Name input.
    let name_input = input_sized(theme, Size::Sm, "Column name", &def.name)
        .on_input(move |s| Message::ColumnNameChanged(ci, s))
        .width(Length::Fill);

    // Type select.
    let current_type_label = type_label(&def.col_type);
    let current_currency_sym = match &def.col_type {
        ColumnType::Currency(sym) => sym.clone(),
        _ => "$".to_string(),
    };
    let type_select = select_sized(
        theme,
        Size::Sm,
        type_options(),
        Some(current_type_label),
        move |label| {
            let new_type = match label.as_str() {
                "Number" => ColumnType::Number,
                "Currency" => ColumnType::Currency(current_currency_sym.clone()),
                "Formula" => ColumnType::Formula,
                _ => ColumnType::Text,
            };
            Message::ColumnTypeChanged(ci, new_type)
        },
    )
    .width(Length::Fill);

    let mut col = column![
        section_label("Name"),
        name_input,
        Space::new().height(Length::Fixed(4.0)),
        section_label("Type"),
        type_select,
    ]
    .spacing(4)
    .width(panel_width);

    col = col.push(Space::new().height(Length::Fixed(4.0)));
    col = col.push(section_label("Sort"));
    col = col.push(build_sort_controls(app, ci));

    col = col.push(Space::new().height(Length::Fixed(4.0)));
    col = col.push(build_group_switch(app, ci));

    // Number / Currency formatting.
    let is_numeric = matches!(def.col_type, ColumnType::Number | ColumnType::Currency(_));
    if is_numeric {
        col = col.push(Space::new().height(Length::Fixed(4.0)));

        let precision_value = def.format.precision.min(6);
        let precision_select = select_sized(
            theme,
            Size::Sm,
            precision_options(),
            Some(precision_value.to_string()),
            move |label| {
                let n = label.parse::<u8>().unwrap_or(2);
                Message::ColumnPrecisionChanged(ci, n)
            },
        )
        .width(Length::Fill);

        col = col.push(section_label("Decimals"));
        col = col.push(precision_select);

        let thousands_switch = switch(
            theme,
            def.format.thousands,
            Some("Thousands separator".into()),
        )
        .on_toggle(move |_| Message::ColumnThousandsToggled(ci))
        .text_size(13.0);

        col = col.push(Space::new().height(Length::Fixed(4.0)));
        col = col.push(thousands_switch);
    }

    // Currency symbol input.
    if let ColumnType::Currency(sym) = &def.col_type {
        col = col.push(Space::new().height(Length::Fixed(4.0)));
        col = col.push(section_label("Currency symbol"));
        let sym_input = input_sized(theme, Size::Sm, "$", sym)
            .on_input(move |s| Message::ColumnCurrencySymbolChanged(ci, s));
        col = col.push(sym_input);
    }

    // Formula editor.
    if matches!(def.col_type, ColumnType::Formula) {
        col = col.push(Space::new().height(Length::Fixed(4.0)));

        let header_row = iced_row![
            section_label("Formula"),
            Space::new().width(Length::Fill),
            hover_card(
                theme,
                icon_colored(IconKind::FunctionSquare, 14.0, t.muted_foreground),
                text(FORMULA_SYNTAX_HELP)
                    .size(12.0)
                    .color(t.popover_foreground)
                    .into(),
            ),
        ]
        .align_y(Vertical::Center);
        col = col.push(header_row);

        let formula_value = if app.formula.editing_col == Some(ci) {
            app.formula.value.as_str()
        } else {
            def.formula.as_deref().unwrap_or("")
        };
        let input: Element<'a, Message> =
            input_sized(theme, Size::Sm, "= expression…", formula_value)
                .on_input(Message::FormulaChanged)
                .on_submit(Message::FormulaEditCommit)
                .width(Length::Fill)
                .into();

        let suggestions = app.formula.suggestions(&app.sheet);
        let suggestion_panel: Option<Element<'a, Message>> =
            if suggestions.is_empty() || app.formula.editing_col != Some(ci) {
                None
            } else {
                let selected = app.formula.suggestions_selected.min(suggestions.len() - 1);
                let rows: Vec<Element<'a, Message>> = suggestions
                    .iter()
                    .enumerate()
                    .map(|(i, (_col_idx, name))| {
                        let variant = if i == selected {
                            Variant::Secondary
                        } else {
                            Variant::Ghost
                        };
                        button_ex(
                            theme,
                            name.clone(),
                            variant,
                            Size::Sm,
                            Some(Message::FormulaSuggestionClick(i)),
                            false,
                            false,
                        )
                    })
                    .collect();
                Some(column(rows).spacing(2).width(Length::Fixed(220.0)).into())
            };

        let input_with_popover =
            popover_dismissable(theme, input, suggestion_panel, Message::FormulaEscape);
        col = col.push(input_with_popover);

        if let Some(err) = &app.formula.error {
            let dot: Element<'a, Message> = container(text(""))
                .width(Length::Fixed(8.0))
                .height(Length::Fixed(8.0))
                .style(move |_| container::Style {
                    background: Some(Background::Color(t.danger)),
                    text_color: Some(t.danger_foreground),
                    border: Border {
                        color: t.danger,
                        width: 0.0,
                        radius: 4.0.into(),
                    },
                    shadow: Shadow::default(),
                    snap: true,
                })
                .into();
            let err_row = iced_row![
                dot,
                text(format!("Didn't evaluate: {err}"))
                    .size(11.0)
                    .color(t.danger),
            ]
            .spacing(6)
            .align_y(Vertical::Center);
            col = col.push(err_row);
        }

        let commit = icon_button(
            theme,
            iced_row![
                icon_colored(IconKind::Check, 12.0, theme.primary_foreground),
                text("Apply").size(12.0).color(theme.primary_foreground),
            ]
            .spacing(4)
            .align_y(Vertical::Center),
            Variant::Primary,
            Size::Sm,
            Some(Message::FormulaEditCommit),
            false,
        );
        let cancel = icon_button(
            theme,
            iced_row![
                icon(theme, IconKind::Close, 12.0),
                text("Cancel").size(12.0).color(theme.foreground),
            ]
            .spacing(4)
            .align_y(Vertical::Center),
            Variant::Ghost,
            Size::Sm,
            Some(Message::FormulaEditCancel),
            false,
        );

        let actions = iced_row![Space::new().width(Length::Fill), cancel, commit]
            .spacing(6)
            .align_y(Vertical::Center);
        col = col.push(actions);
    }

    // Delete column — danger button with a confirm popover.
    let only_one_column = app.sheet.col_count() <= 1;
    col = col.push(Space::new().height(Length::Fixed(8.0)));
    col = col.push(build_delete_column_control(app, ci, only_one_column));

    container(col)
        .padding(Padding::from(12.0))
        .width(panel_width)
        .into()
}

fn build_sort_controls<'a>(app: &'a TableApp, ci: usize) -> Element<'a, Message> {
    let theme = &app.theme;
    let current = app.sheet.sort.as_ref().filter(|s| s.column == ci);
    let active_dir = current.map(|s| s.direction);

    let asc_variant = if active_dir == Some(SortDirection::Ascending) {
        Variant::Secondary
    } else {
        Variant::Outline
    };
    let desc_variant = if active_dir == Some(SortDirection::Descending) {
        Variant::Secondary
    } else {
        Variant::Outline
    };

    let asc_btn = icon_button(
        theme,
        iced_row![
            icon(theme, IconKind::ArrowUp, 12.0),
            text("Asc").size(12.0).color(theme.foreground),
        ]
        .spacing(4)
        .align_y(Vertical::Center),
        asc_variant,
        Size::Sm,
        Some(Message::SortColumnDir(ci, SortDirection::Ascending)),
        false,
    );
    let desc_btn = icon_button(
        theme,
        iced_row![
            icon(theme, IconKind::ArrowDown, 12.0),
            text("Desc").size(12.0).color(theme.foreground),
        ]
        .spacing(4)
        .align_y(Vertical::Center),
        desc_variant,
        Size::Sm,
        Some(Message::SortColumnDir(ci, SortDirection::Descending)),
        false,
    );

    let mut row = iced_row![asc_btn, desc_btn].spacing(4);
    if active_dir.is_some() {
        row = row.push(button_ex(
            theme,
            "Clear",
            Variant::Ghost,
            Size::Sm,
            Some(Message::SortColumn(None)),
            false,
            false,
        ));
    }
    row.align_y(Vertical::Center).into()
}

fn build_group_switch<'a>(app: &'a TableApp, ci: usize) -> Element<'a, Message> {
    let theme = &app.theme;
    let active = app.sheet.group_by == Some(ci);
    let target = if active { None } else { Some(ci) };
    switch(theme, active, Some("Group by this column".into()))
        .on_toggle(move |_| Message::GroupByColumn(target))
        .text_size(13.0)
        .into()
}

fn build_delete_column_control<'a>(
    app: &'a TableApp,
    ci: usize,
    only_one_column: bool,
) -> Element<'a, Message> {
    let theme = &app.theme;
    let t = *theme;

    if app.column_delete_confirm && !only_one_column {
        let col_name = app
            .sheet
            .columns
            .get(ci)
            .map(|d| d.name.as_str())
            .unwrap_or("");
        let warning = text(format!(
            "Delete column \"{}\"? This cannot be undone.",
            col_name
        ))
        .size(12.0)
        .color(t.foreground);

        let cancel_btn = button_ex(
            theme,
            "Cancel",
            Variant::Ghost,
            Size::Sm,
            Some(Message::ColumnDeleteConfirmToggle(false)),
            false,
            false,
        );
        let confirm_btn = button_ex(
            theme,
            "Delete",
            Variant::Danger,
            Size::Sm,
            Some(Message::DeleteColumn(ci)),
            false,
            false,
        );

        let actions = iced_row![Space::new().width(Length::Fill), cancel_btn, confirm_btn]
            .spacing(6)
            .align_y(Vertical::Center);

        let confirm_panel = container(
            column![warning, Space::new().height(Length::Fixed(8.0)), actions]
                .spacing(0)
                .width(Length::Fill),
        )
        .padding(Padding::from(10.0))
        .style(move |_| container::Style {
            background: Some(Background::Color(iced_longbridge::theme::with_alpha(
                t.danger, 0.08,
            ))),
            text_color: Some(t.foreground),
            border: Border {
                color: t.danger,
                width: 1.0,
                radius: 6.0.into(),
            },
            shadow: Shadow::default(),
            snap: true,
        });

        return confirm_panel.into();
    }

    let trigger_label = iced_row![
        icon_colored(IconKind::Trash, 12.0, t.danger_foreground),
        text("Delete column").size(12.0).color(t.danger_foreground),
    ]
    .spacing(6)
    .align_y(Vertical::Center);

    let on_press = if only_one_column {
        None
    } else {
        Some(Message::ColumnDeleteConfirmToggle(true))
    };

    icon_button(
        theme,
        trigger_label,
        Variant::Danger,
        Size::Sm,
        on_press,
        only_one_column,
    )
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
    format: &NumberFormat,
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
            .padding(Padding::from([2.0, 6.0])) // TODO Remove padding
            .style(move |_, status| cell_input_style(&t, status))
            .into();
    }

    let child: Element<'a, Message> = if is_formula {
        // Formula cells are read-only.
        let display = cell.display_value(currency_symbol, format);
        container(text(display).size(13.0).color(t.muted_foreground))
            .padding(Padding::from([0.0, 4.0]))
            .width(Length::Fill)
            .into()
    } else {
        let display = cell.display_value(currency_symbol, format);
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

    let mut cut = MenuItem::new("Cut", Message::CutCell(row, col))
        .icon(IconKind::Trash)
        .shortcut("⌘X");
    if is_formula {
        cut = cut.disabled();
    }
    items.push(cut);

    items.push(
        MenuItem::new("Copy", Message::CopyCell(row, col))
            .icon(IconKind::Copy)
            .shortcut("⌘C"),
    );

    let mut paste = MenuItem::new("Paste", Message::PasteCell(row, col))
        .icon(IconKind::Paste)
        .shortcut("⌘V");
    if !clipboard_cell_some || is_formula {
        paste = paste.disabled();
    }
    items.push(paste);

    items.push(MenuItem::Separator);

    let mut clear = MenuItem::new("Clear contents", Message::ClearCell(row, col)).shortcut("Del");
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

    let mut paste_row = MenuItem::new("Paste row", Message::PasteRow(row))
        .icon(IconKind::Paste)
        .shortcut("⇧⌘V");
    if !clipboard_row_some {
        paste_row = paste_row.disabled();
    }
    items.push(paste_row);

    items.push(
        MenuItem::new("Delete row", Message::DeleteRow(row))
            .icon(IconKind::Trash)
            .danger(),
    );

    items.push(MenuItem::Separator);

    items.push(MenuItem::new(
        "Insert column left",
        Message::InsertColumnLeft(col),
    ));
    items.push(MenuItem::new(
        "Insert column right",
        Message::InsertColumnRight(col),
    ));

    let mut delete_col = MenuItem::new("Delete column", Message::DeleteColumn(col))
        .icon(IconKind::Trash)
        .danger();
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
            MenuItem::new("Cut", Message::CutRow(row_idx))
                .icon(IconKind::Trash)
                .shortcut("⌘X"),
            MenuItem::new("Copy", Message::CopyRow(row_idx))
                .icon(IconKind::Copy)
                .shortcut("⌘C"),
        ];
        let mut paste = MenuItem::new("Paste", Message::PasteRow(row_idx))
            .icon(IconKind::Paste)
            .shortcut("⌘V");
        if !clipboard_has_value {
            paste = paste.disabled();
        }
        items.push(paste);
        items.push(MenuItem::Separator);
        items.push(
            MenuItem::new("Delete row", Message::DeleteRow(row_idx))
                .icon(IconKind::Trash)
                .danger(),
        );
        menu(theme, items)
    });

    popover_dismissable(theme, trigger, panel, Message::RowMenuToggle(None))
}

fn cell_input_style(
    t: &AppTheme,
    status: iced::widget::text_input::Status,
) -> iced::widget::text_input::Style {
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
