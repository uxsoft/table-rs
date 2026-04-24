mod data;
mod ui;

use data::formula::evaluate_all_formulas;
use data::operations::{group_rows, sort_rows};
use data::{CellValue, ColumnType, Group, Sheet, SortConfig, SortDirection};

use iced::alignment::Vertical;
use iced::widget::{column, container, row, text};
use iced::{
    Background, Border, Element, Font, Length, Padding, Point, Shadow, Subscription,
    Task, Theme,
};
use iced_longbridge::components::button::{button_ex, Variant};
use iced_longbridge::components::icon::IconName;
use iced_longbridge::components::menu::Item;
use iced_longbridge::components::menu_bar::{menu_bar, MenuBarMenu};
use iced_longbridge::components::notification::{
    notification_layer, Notification, NotificationKind, NotificationList,
};
use iced_longbridge::theme::{AppTheme, Appearance, Size};

const INTER_FONT: &[u8] = include_bytes!("../assets/fonts/Inter-Regular.ttf");
const SYMBOLS_NERD_FONT: &[u8] =
    include_bytes!("../assets/fonts/SymbolsNerdFont-Regular.ttf");

/// Maximum column count we reserve sortable `&'static str` keys for.
/// Columns beyond this render without click-to-sort affordance (sidebar
/// controls still work). 64 is comfortably above any realistic spreadsheet.
pub const MAX_SORT_COLS: usize = 64;

/// Leaked "col_0" .. "col_63" keys so longbridge's `&'static str`-keyed sort
/// API can refer to runtime-defined columns.
pub static SORT_KEYS: std::sync::LazyLock<Vec<&'static str>> =
    std::sync::LazyLock::new(|| {
        (0..MAX_SORT_COLS)
            .map(|i| Box::leak(format!("col_{i}").into_boxed_str()) as &'static str)
            .collect()
    });

pub fn sort_key_for(col: usize) -> Option<&'static str> {
    SORT_KEYS.get(col).copied()
}

pub fn col_for_sort_key(key: &str) -> Option<usize> {
    key.strip_prefix("col_").and_then(|s| s.parse().ok())
}

fn main() -> iced::Result {
    iced::application(TableApp::new, TableApp::update, TableApp::view)
        .title(TableApp::title)
        .theme(TableApp::theme)
        .subscription(TableApp::subscription)
        .font(INTER_FONT)
        .font(SYMBOLS_NERD_FONT)
        .default_font(Font::with_name("Inter"))
        .window_size((1280.0, 760.0))
        .run()
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
    CellEdited(String),
    CellEditCommit,
    CellEditCancel,

    // Rows & columns
    AddRow,
    NewColNameChanged(String),
    NewColTypeChanged(ColumnType),
    AddColumn(ColumnType),
    ColumnTypeChanged(usize, ColumnType),

    // Column formula
    FormulaOpenEditor(usize),
    FormulaChanged(String),
    FormulaEditCommit,
    FormulaEditCancel,

    // Sort
    TableSort(&'static str),
    SortColumn(Option<usize>),
    ToggleSortDirection,

    // Group
    GroupByColumn(Option<usize>),
    ToggleGroup(usize),

    // Row context actions
    RowMenuToggle(Option<usize>),
    CutRow(usize),
    CopyRow(usize),
    PasteRow(usize),
    DeleteRow(usize),

    // Table resize (longbridge)
    TableResizeStart(usize),
    TableResizeMove(Point),
    TableResizeEnd,

    // Menu bar
    MenuBarToggle(usize),

    // Theme
    ThemeToggle,

    // Notifications
    NotifyTick,
    NotifyDismiss(u64),
}

pub struct TableApp {
    pub sheet: Sheet,
    pub editing: Option<(usize, usize)>,
    pub edit_value: String,
    pub groups: Option<Vec<Group>>,
    pub new_col_name: String,
    pub new_col_type: ColumnType,
    pub clipboard_row: Option<Vec<CellValue>>,
    pub editing_formula_col: Option<usize>,
    pub editing_formula_value: String,

    // Table resize state (longbridge ResizeHandlers)
    pub table_resize_col: Option<usize>,
    pub table_resize_last_x: Option<f32>,

    // Transient UI state
    pub row_menu_open: Option<usize>,
    pub menubar_open: Option<usize>,

    // Theme + notifications
    pub theme: AppTheme,
    pub notifications: NotificationList,
    next_notification_id: u64,
}

impl TableApp {
    fn new() -> Self {
        let mut app = TableApp {
            sheet: Sheet::new_empty(),
            editing: None,
            edit_value: String::new(),
            groups: None,
            new_col_name: String::new(),
            new_col_type: ColumnType::Text,
            clipboard_row: None,
            editing_formula_col: None,
            editing_formula_value: String::new(),
            table_resize_col: None,
            table_resize_last_x: None,
            row_menu_open: None,
            menubar_open: None,
            theme: AppTheme::dark(),
            notifications: NotificationList::new(),
            next_notification_id: 1,
        };
        evaluate_all_formulas(&mut app.sheet);
        app.recompute_groups();
        app
    }

    fn title(&self) -> String {
        match self.sheet.file_path {
            Some(ref p) => format!(
                "Table RS — {}",
                p.file_name().and_then(|s| s.to_str()).unwrap_or("untitled")
            ),
            None => "Table RS".to_string(),
        }
    }

    fn theme(&self) -> Theme {
        self.theme.iced_theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        iced::time::every(std::time::Duration::from_millis(250))
            .map(|_| Message::NotifyTick)
    }

    pub fn recompute_groups(&mut self) {
        self.groups = group_rows(&self.sheet);
    }

    fn notify(&mut self, kind: NotificationKind, title: impl Into<String>) {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        self.notifications
            .push(Notification::new(id, kind, title).ttl_ms(3_500));
    }

    fn notify_msg(
        &mut self,
        kind: NotificationKind,
        title: impl Into<String>,
        msg: impl Into<String>,
    ) {
        let id = self.next_notification_id;
        self.next_notification_id += 1;
        self.notifications.push(
            Notification::new(id, kind, title)
                .message(msg)
                .ttl_ms(5_000),
        );
    }

    fn commit_edit(&mut self) {
        if let Some((row, col)) = self.editing.take() {
            let value = std::mem::take(&mut self.edit_value);
            self.sheet.set_cell(row, col, value);
            evaluate_all_formulas(&mut self.sheet);
            if self.sheet.sort.is_some() {
                sort_rows(&mut self.sheet);
            }
            self.recompute_groups();
        }
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::NotifyTick => {
                self.notifications
                    .tick(std::time::Duration::from_millis(250));
            }
            Message::NotifyDismiss(id) => {
                self.notifications.dismiss(id);
            }
            Message::ThemeToggle => {
                self.theme = match self.theme.appearance {
                    Appearance::Light => AppTheme::dark(),
                    Appearance::Dark => AppTheme::light(),
                };
            }
            Message::MenuBarToggle(i) => {
                self.menubar_open = match self.menubar_open {
                    Some(cur) if cur == i => None,
                    _ => Some(i),
                };
                self.row_menu_open = None;
            }
            Message::RowMenuToggle(row) => {
                self.row_menu_open = match (self.row_menu_open, row) {
                    (Some(cur), Some(r)) if cur == r => None,
                    _ => row,
                };
                self.menubar_open = None;
            }
            Message::TableResizeStart(i) => {
                self.table_resize_col = Some(i);
                self.table_resize_last_x = None;
            }
            Message::TableResizeMove(pt) => {
                if let Some(i) = self.table_resize_col {
                    if let Some(last) = self.table_resize_last_x {
                        let delta = pt.x - last;
                        if let Some(col) = self.sheet.columns.get_mut(i) {
                            col.width = (col.width + delta).clamp(60.0, 800.0);
                        }
                    }
                    self.table_resize_last_x = Some(pt.x);
                }
            }
            Message::TableResizeEnd => {
                self.table_resize_col = None;
                self.table_resize_last_x = None;
            }
            Message::CopyRow(row) => {
                if row < self.sheet.rows.len() {
                    self.clipboard_row = Some(self.sheet.rows[row].clone());
                }
                self.row_menu_open = None;
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
                self.row_menu_open = None;
            }
            Message::PasteRow(row) => {
                if let Some(clipboard) = self.clipboard_row.clone() {
                    self.sheet.insert_row_after(row, clipboard);
                    self.recompute_groups();
                }
                self.row_menu_open = None;
            }
            Message::DeleteRow(row) => {
                self.sheet.delete_row(row);
                if let Some((erow, _)) = self.editing {
                    if erow >= self.sheet.rows.len() {
                        self.editing = None;
                    }
                }
                self.recompute_groups();
                self.row_menu_open = None;
            }
            Message::FileOpen => {
                self.menubar_open = None;
                return Task::perform(
                    async {
                        let handle = rfd::AsyncFileDialog::new()
                            .add_filter("CSV", &["csv"])
                            .pick_file()
                            .await;
                        match handle {
                            Some(h) => data::csv_io::load(h.path())
                                .map_err(|e| e.to_string()),
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
                    let name = self
                        .sheet
                        .file_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .unwrap_or("file")
                        .to_string();
                    self.notify_msg(NotificationKind::Success, "Loaded", name);
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.notify_msg(NotificationKind::Error, "Load failed", e);
                }
            },
            Message::FileSave => {
                self.menubar_open = None;
                let sheet = self.sheet.clone();
                if let Some(ref path) = sheet.file_path {
                    let path = path.clone();
                    return Task::perform(
                        async move {
                            data::csv_io::save(&sheet, &path).map_err(|e| e.to_string())
                        },
                        Message::FileSaved,
                    );
                } else {
                    return Task::perform(
                        async move {
                            let handle = rfd::AsyncFileDialog::new()
                                .add_filter("CSV", &["csv"])
                                .save_file()
                                .await;
                            match handle {
                                Some(h) => data::csv_io::save(&sheet, h.path())
                                    .map_err(|e| e.to_string()),
                                None => Err("Cancelled".to_string()),
                            }
                        },
                        Message::FileSaved,
                    );
                }
            }
            Message::FileSaved(result) => match result {
                Ok(()) => {
                    self.notify(NotificationKind::Success, "Saved");
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.notify_msg(NotificationKind::Error, "Save failed", e);
                }
            },
            Message::CellClicked(row, col) => {
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some()) {
                    return Task::none();
                }
                self.commit_edit();
                if row < self.sheet.rows.len() && col < self.sheet.columns.len() {
                    self.editing = Some((row, col));
                    self.edit_value = self.sheet.rows[row][col].edit_value();
                }
            }
            Message::CellEdited(value) => {
                self.edit_value = value;
            }
            Message::CellEditCommit => {
                self.commit_edit();
            }
            Message::CellEditCancel => {
                self.editing = None;
                self.edit_value.clear();
            }
            Message::TableSort(key) => {
                if let Some(col) = col_for_sort_key(key) {
                    // Formula columns: open formula editor instead of sorting.
                    if matches!(
                        self.sheet.columns.get(col),
                        Some(c) if c.col_type == ColumnType::Formula
                    ) {
                        self.open_formula_editor(col);
                        return Task::none();
                    }
                    match &self.sheet.sort {
                        Some(s) if s.column == col => match s.direction {
                            SortDirection::Ascending => {
                                self.sheet.sort = Some(SortConfig {
                                    column: col,
                                    direction: SortDirection::Descending,
                                });
                            }
                            SortDirection::Descending => {
                                self.sheet.sort = None;
                            }
                        },
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
            }
            Message::ColumnTypeChanged(col, new_type) => {
                if col < self.sheet.columns.len() {
                    self.sheet.columns[col].col_type = new_type.clone();
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
            Message::NewColNameChanged(name) => {
                self.new_col_name = name;
            }
            Message::NewColTypeChanged(t) => {
                self.new_col_type = t;
            }
            Message::AddColumn(col_type) => {
                let name = if self.new_col_name.trim().is_empty() {
                    format!("Column {}", self.sheet.col_count() + 1)
                } else {
                    self.new_col_name.trim().to_string()
                };
                let is_formula = col_type == ColumnType::Formula;
                self.sheet.add_column(name, col_type);
                self.new_col_name.clear();
                if is_formula {
                    let new_col = self.sheet.columns.len() - 1;
                    self.open_formula_editor(new_col);
                }
                self.recompute_groups();
            }
            Message::FormulaOpenEditor(col) => {
                self.open_formula_editor(col);
            }
            Message::FormulaChanged(value) => {
                self.editing_formula_value = value;
            }
            Message::FormulaEditCommit => {
                if let Some(col) = self.editing_formula_col {
                    let expr = self.editing_formula_value.trim().to_string();
                    if col < self.sheet.columns.len() {
                        self.sheet.columns[col].formula =
                            if expr.is_empty() { None } else { Some(expr) };
                        evaluate_all_formulas(&mut self.sheet);
                        self.recompute_groups();
                    }
                }
                self.editing_formula_col = None;
                self.editing_formula_value.clear();
            }
            Message::FormulaEditCancel => {
                self.editing_formula_col = None;
                self.editing_formula_value.clear();
            }
            Message::SortColumn(col) => {
                self.sheet.sort = col.map(|c| SortConfig {
                    column: c,
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
            Message::GroupByColumn(col) => {
                self.sheet.group_by = col;
                self.recompute_groups();
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

    fn open_formula_editor(&mut self, col: usize) {
        if col < self.sheet.columns.len() {
            let current = self.sheet.columns[col].formula.clone().unwrap_or_default();
            self.editing_formula_col = Some(col);
            self.editing_formula_value = current;
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = &self.theme;

        let theme_toggle_label = match self.theme.appearance {
            Appearance::Light => "☾",
            Appearance::Dark => "☀",
        };
        let theme_toggle: Element<'_, Message> = button_ex(
            theme,
            theme_toggle_label,
            Variant::Ghost,
            Size::Sm,
            Some(Message::ThemeToggle),
            false,
            false,
        );

        let file_menu = MenuBarMenu::new(
            "File",
            Message::MenuBarToggle(0),
            vec![
                Item::new("Open", Message::FileOpen).icon(IconName::Folder),
                Item::new("Save", Message::FileSave).icon(IconName::Save),
            ],
        );
        let menubar = menu_bar(theme, vec![file_menu], self.menubar_open);

        let t = *theme;
        let chrome = container(
            row![
                text("▦").size(14.0).color(t.foreground),
                menubar,
                theme_toggle,
            ]
            .spacing(8)
            .align_y(Vertical::Center),
        )
        .padding(Padding::from([4.0, 14.0]))
        .width(Length::Fill)
        .style(move |_| container::Style {
            background: Some(Background::Color(t.background)),
            text_color: Some(t.foreground),
            border: Border {
                color: t.border,
                width: 0.0,
                radius: 0.0.into(),
            },
            shadow: Shadow::default(),
            snap: true,
        });

        let body = row![
            container(ui::sidebar::view(self)).width(Length::Fixed(280.0)),
            container(ui::main_panel::view(self))
                .width(Length::Fill)
                .height(Length::Fill),
        ]
        .spacing(0);

        let base: Element<'_, Message> = column![chrome, body]
            .width(Length::Fill)
            .height(Length::Fill)
            .into();

        notification_layer(theme, base, &self.notifications, Message::NotifyDismiss)
    }
}
