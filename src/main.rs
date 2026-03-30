mod components;
mod data;
mod ui;

use data::formula::evaluate_all_formulas;
use data::operations::{group_rows, sort_rows};
use data::{ColumnType, Group, Sheet, SortConfig, SortDirection};
use iced::widget::{button, column, container, horizontal_space, stack, text};
use iced::Point;
use iced::{alignment, Element, Font, Length, Padding, Task, Theme};

const MENUBAR_HEIGHT: f32 = 30.0;
const INTER_FONT: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const SYMBOLS_NERD_FONT: &[u8] =
    include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");

fn main() -> iced::Result {
    iced::application("Table RS", TableApp::update, TableApp::view)
        .subscription(TableApp::subscription)
        .font(INTER_FONT)
        .font(SYMBOLS_NERD_FONT)
        .default_font(Font::with_name("Inter"))
        .theme(|_| Theme::TokyoNight)
        .window_size((1200.0, 700.0))
        .run_with(|| {
            let mut app = TableApp::new();
            evaluate_all_formulas(&mut app.sheet);
            app.recompute_groups();
            (app, Task::none())
        })
}

#[derive(Debug, Clone)]
pub enum Message {
    // File ops
    FileOpen,
    FileLoaded(Result<Sheet, String>),
    FileSave,
    FileSaved(Result<(), String>),

    // Cell editing
    CellClicked(usize, usize),
    CellEdited(usize, usize, String),
    CellEditCommit,
    CellEditCancel,

    // Column header
    HeaderClicked(usize),
    ColumnTypeChanged(usize, ColumnType),

    // Rows & columns
    AddRow,
    ToggleAddColumn,
    NewColNameChanged(String),
    AddColumn(ColumnType),

    // Column formula
    FormulaChanged(usize, String),
    FormulaEditCommit(usize),

    // Sort
    SortColumn(usize),
    ToggleSortDirection,
    ClearSort,

    // Group
    GroupByColumn(usize),
    ClearGroup,
    ToggleGroup(usize),

    // Menu
    OpenMenu(String),
    CloseMenu,

    // Context menu
    RowRightClicked(usize),
    CutRow(usize),
    CopyRow(usize),
    PasteRow(usize),
    DeleteRow(usize),
    CloseContextMenu,

    // Column resizing
    ColResizeStart(usize),
    ColResizeEnd,

    // Cursor tracking (for context menu positioning)
    CursorMoved(Point),
}

struct TableApp {
    sheet: Sheet,
    editing: Option<(usize, usize)>,
    edit_value: String,
    groups: Option<Vec<Group>>,
    show_add_col: bool,
    new_col_name: String,
    status: String,
    open_menu: Option<String>,
    context_menu: Option<(usize, Point)>,
    clipboard_row: Option<Vec<data::CellValue>>,
    cursor_pos: Point,
    /// Column index whose formula is currently being edited in the toolbar
    editing_formula_col: Option<usize>,
    editing_formula_value: String,
    resizing_col: Option<usize>,
    resize_start_x: f32,
    resize_start_width: f32,
}

impl TableApp {
    fn new() -> Self {
        TableApp {
            sheet: Sheet::new_empty(),
            editing: None,
            edit_value: String::new(),
            groups: None,
            show_add_col: false,
            new_col_name: String::new(),
            status: "Ready".to_string(),
            open_menu: None,
            context_menu: None,
            clipboard_row: None,
            cursor_pos: Point::ORIGIN,
            editing_formula_col: None,
            editing_formula_value: String::new(),
            resizing_col: None,
            resize_start_x: 0.0,
            resize_start_width: 0.0,
        }
    }

    fn recompute_groups(&mut self) {
        self.groups = group_rows(&self.sheet);
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::CursorMoved(pos) => {
                // Drive column resize if active.
                if let Some(col) = self.resizing_col {
                    let delta = pos.x - self.resize_start_x;
                    let new_width = (self.resize_start_width + delta).max(40.0);
                    if col < self.sheet.columns.len() {
                        self.sheet.columns[col].width = new_width;
                    }
                }
                self.cursor_pos = pos;
            }
            Message::ColResizeStart(col) => {
                if col < self.sheet.columns.len() {
                    self.resizing_col = Some(col);
                    self.resize_start_x = self.cursor_pos.x;
                    self.resize_start_width = self.sheet.columns[col].width;
                }
            }
            Message::ColResizeEnd => {
                self.resizing_col = None;
            }
            Message::RowRightClicked(row) => {
                self.context_menu = Some((row, self.cursor_pos));
                self.open_menu = None;
            }
            Message::CloseContextMenu => {
                self.context_menu = None;
            }
            Message::CopyRow(row) => {
                if row < self.sheet.rows.len() {
                    self.clipboard_row = Some(self.sheet.rows[row].clone());
                }
                self.context_menu = None;
            }
            Message::CutRow(row) => {
                if row < self.sheet.rows.len() {
                    self.clipboard_row = Some(self.sheet.rows[row].clone());
                    self.sheet.delete_row(row);
                    if let Some((erow, _)) = self.editing {
                        if erow >= self.sheet.rows.len() {
                            self.editing = None;
                        }
                    }
                    self.recompute_groups();
                }
                self.context_menu = None;
            }
            Message::PasteRow(row) => {
                if let Some(clipboard) = self.clipboard_row.clone() {
                    self.sheet.insert_row_after(row, clipboard);
                    self.recompute_groups();
                }
                self.context_menu = None;
            }
            Message::DeleteRow(row) => {
                self.sheet.delete_row(row);
                if let Some((erow, _)) = self.editing {
                    if erow >= self.sheet.rows.len() {
                        self.editing = None;
                    }
                }
                self.recompute_groups();
                self.context_menu = None;
            }
            Message::OpenMenu(name) => {
                self.open_menu = Some(name);
                self.context_menu = None;
            }
            Message::CloseMenu => {
                self.open_menu = None;
            }
            Message::FileOpen => {
                self.open_menu = None;
                return Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("CSV", &["csv"])
                            .pick_file()
                            .await;

                        match handle {
                            Some(h) => {
                                let path = h.path().to_path_buf();
                                data::csv_io::load(&path)
                                    .map_err(|e| e.to_string())
                            }
                            None => Err("Cancelled".to_string()),
                        }
                    },
                    Message::FileLoaded,
                );
            }
            Message::FileLoaded(result) => match result {
                Ok(mut sheet) => {
                    evaluate_all_formulas(&mut sheet);
                    self.sheet = sheet;
                    self.editing = None;
                    self.recompute_groups();
                    sort_rows(&mut self.sheet);
                    self.status = "File loaded".to_string();
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.status = format!("Error: {}", e);
                }
            },
            Message::FileSave => {
                self.open_menu = None;
                let sheet = self.sheet.clone();
                if let Some(ref path) = sheet.file_path {
                    let path = path.clone();
                    return Task::perform(
                        async move {
                            data::csv_io::save(&sheet, &path)
                                .map_err(|e| e.to_string())
                        },
                        Message::FileSaved,
                    );
                } else {
                    // Save As
                    return Task::perform(
                        async move {
                            let handle = rfd::AsyncFileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .save_file()
                                .await;

                            match handle {
                                Some(h) => {
                                    let path = h.path().to_path_buf();
                                    data::csv_io::save(&sheet, &path)
                                        .map_err(|e| e.to_string())
                                }
                                None => Err("Cancelled".to_string()),
                            }
                        },
                        Message::FileSaved,
                    );
                }
            }
            Message::FileSaved(result) => match result {
                Ok(()) => {
                    self.status = "File saved".to_string();
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.status = format!("Save error: {}", e);
                }
            },
            Message::CellClicked(row, col) => {
                // Formula columns are read-only — clicking does nothing.
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some()) {
                    return Task::none();
                }
                // Commit previous edit if any
                self.commit_edit();
                self.editing = Some((row, col));
                self.edit_value = self.sheet.rows[row][col].edit_value();
            }
            Message::CellEdited(_row, _col, value) => {
                self.edit_value = value;
            }
            Message::CellEditCommit => {
                self.commit_edit();
            }
            Message::CellEditCancel => {
                self.editing = None;
                self.edit_value.clear();
            }
            Message::HeaderClicked(col) => {
                // If this is a formula column, open formula editor instead of sorting.
                if matches!(self.sheet.columns.get(col), Some(c) if c.col_type == ColumnType::Formula) {
                    let current = self.sheet.columns[col]
                        .formula
                        .clone()
                        .unwrap_or_default();
                    self.editing_formula_col = Some(col);
                    self.editing_formula_value = current;
                    return Task::none();
                }
                // Toggle sort on this column
                match &self.sheet.sort {
                    Some(s) if s.column == col => {
                        match s.direction {
                            SortDirection::Ascending => {
                                self.sheet.sort = Some(SortConfig {
                                    column: col,
                                    direction: SortDirection::Descending,
                                });
                            }
                            SortDirection::Descending => {
                                self.sheet.sort = None;
                            }
                        }
                    }
                    _ => {
                        self.sheet.sort = Some(SortConfig {
                            column: col,
                            direction: SortDirection::Ascending,
                        });
                    }
                }
                sort_rows(&mut self.sheet);
                self.recompute_groups();
            }
            Message::ColumnTypeChanged(col, new_type) => {
                if col < self.sheet.columns.len() {
                    self.sheet.columns[col].col_type = new_type.clone();
                    // Re-parse all cells in this column
                    for row in &mut self.sheet.rows {
                        if let Some(cell) = row.get(col) {
                            let raw = cell.edit_value();
                            row[col] = data::parse_cell_value(&raw, &new_type);
                        }
                    }
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                }
            }
            Message::AddRow => {
                self.sheet.add_row();
                self.recompute_groups();
            }
            Message::ToggleAddColumn => {
                self.show_add_col = !self.show_add_col;
                self.new_col_name.clear();
            }
            Message::NewColNameChanged(name) => {
                self.new_col_name = name;
            }
            Message::AddColumn(col_type) => {
                let name = if self.new_col_name.trim().is_empty() {
                    format!("Column {}", self.sheet.col_count() + 1)
                } else {
                    self.new_col_name.trim().to_string()
                };
                let is_formula = col_type == ColumnType::Formula;
                self.sheet.add_column(name, col_type);
                self.show_add_col = false;
                self.new_col_name.clear();
                // Immediately open the formula editor for new formula columns.
                if is_formula {
                    let new_col = self.sheet.columns.len() - 1;
                    self.editing_formula_col = Some(new_col);
                    self.editing_formula_value = String::new();
                }
                self.recompute_groups();
            }
            Message::FormulaChanged(_col, value) => {
                self.editing_formula_value = value;
            }
            Message::FormulaEditCommit(col) => {
                let expr = self.editing_formula_value.trim().to_string();
                if col < self.sheet.columns.len() {
                    self.sheet.columns[col].formula = if expr.is_empty() {
                        None
                    } else {
                        Some(expr)
                    };
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                }
                self.editing_formula_col = None;
                self.editing_formula_value.clear();
            }
            Message::SortColumn(col) => {
                self.sheet.sort = Some(SortConfig {
                    column: col,
                    direction: SortDirection::Ascending,
                });
                sort_rows(&mut self.sheet);
                self.recompute_groups();
            }
            Message::ToggleSortDirection => {
                if let Some(ref mut s) = self.sheet.sort {
                    s.direction = match s.direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                }
                sort_rows(&mut self.sheet);
                self.recompute_groups();
            }
            Message::ClearSort => {
                self.sheet.sort = None;
                self.recompute_groups();
            }
            Message::GroupByColumn(col) => {
                self.sheet.group_by = Some(col);
                self.recompute_groups();
            }
            Message::ClearGroup => {
                self.sheet.group_by = None;
                self.groups = None;
            }
            Message::ToggleGroup(gi) => {
                if let Some(ref mut groups) = self.groups {
                    if let Some(g) = groups.get_mut(gi) {
                        g.collapsed = !g.collapsed;
                    }
                }
            }
        }
        Task::none()
    }

    fn commit_edit(&mut self) {
        if let Some((row, col)) = self.editing.take() {
            let value = self.edit_value.clone();
            self.sheet.set_cell(row, col, value);
            evaluate_all_formulas(&mut self.sheet);
            if self.sheet.sort.is_some() {
                sort_rows(&mut self.sheet);
            }
            self.recompute_groups();
            self.edit_value.clear();
        }
    }

    fn subscription(&self) -> iced::Subscription<Message> {
        iced::event::listen_with(|event, _status, _id| {
            match event {
                iced::Event::Mouse(iced::mouse::Event::CursorMoved { position }) => {
                    Some(Message::CursorMoved(position))
                }
                iced::Event::Mouse(
                    iced::mouse::Event::ButtonReleased(iced::mouse::Button::Left),
                ) => Some(Message::ColResizeEnd),
                _ => None,
            }
        })
    }

    fn view(&self) -> Element<'_, Message> {
        let menubar = components::menubar::view_menubar(self.open_menu.as_deref());

        let toolbar = ui::toolbar::view_toolbar(
            &self.sheet,
            self.show_add_col,
            &self.new_col_name,
            self.editing_formula_col,
            &self.editing_formula_value,
        );

        let table = ui::table_view::view_table(
            &self.sheet,
            self.editing,
            &self.edit_value,
            &self.groups,
            self.resizing_col,
        );

        let status_bar = container(text(&self.status).size(12))
            .padding(Padding::from([4.0, 12.0]))
            .width(Length::Fill);

        let main: Element<'_, Message> = column![menubar, toolbar, table, status_bar]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        if self.open_menu.as_deref() == Some("file") {
            let dropdown = components::menubar::view_file_dropdown();

            // Full-screen transparent backdrop: clicking it closes the menu
            let backdrop: Element<'_, Message> = button(horizontal_space())
                .on_press(Message::CloseMenu)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(components::menubar::transparent_btn_style)
                .into();

            // Position dropdown below the menu bar
            let positioned: Element<'_, Message> = container(dropdown)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Top)
                .padding(Padding {
                    top: MENUBAR_HEIGHT,
                    right: 0.0,
                    bottom: 0.0,
                    left: 0.0,
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

            stack![main, stack![backdrop, positioned]].into()
        } else if let Some((row_index, pos)) = self.context_menu {
            let ctx_menu = components::context_menu::view_context_menu(
                row_index,
                self.clipboard_row.is_some(),
            );

            let backdrop: Element<'_, Message> = button(horizontal_space())
                .on_press(Message::CloseContextMenu)
                .width(Length::Fill)
                .height(Length::Fill)
                .style(components::menubar::transparent_btn_style)
                .into();

            let positioned: Element<'_, Message> = container(ctx_menu)
                .align_x(alignment::Horizontal::Left)
                .align_y(alignment::Vertical::Top)
                .padding(Padding {
                    top: pos.y,
                    left: pos.x,
                    right: 0.0,
                    bottom: 0.0,
                })
                .width(Length::Fill)
                .height(Length::Fill)
                .into();

            stack![main, stack![backdrop, positioned]].into()
        } else {
            main
        }
    }
}
