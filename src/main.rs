mod app;
mod data;
mod ui;

use app::formula_state::FormulaEditorState;
use app::notifications::NotificationManager;
use data::formula::evaluate_all_formulas;
use data::operations::{group_rows, sort_rows};
use data::{CellValue, ColumnType, Group, Sheet, SortConfig, SortDirection};

use iced::alignment::Vertical;
use iced::widget::{column, container, row};
use iced::{
    Background, Border, Element, Font, Length, Padding, Shadow, Subscription,
    Task, Theme,
};
use iced_longbridge::components::button::Variant;
use iced_longbridge::components::menu::Item;
use iced_longbridge::components::menu_bar::{menu_bar, MenuBarMenu};
use iced_longbridge::components::notification::{notification_layer, NotificationKind};
use iced_longbridge::components::table::{ResizeEvent, ResizeState};
use iced_longbridge::theme::{AppTheme, Appearance, Size};

use crate::ui::icons::{icon, icon_button, IconKind};

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
    FileOpenConfirmed(Option<std::path::PathBuf>),
    FileOpenRecent(std::path::PathBuf),
    FileLoaded(Result<Sheet, String>),
    FileSave,
    FileSaved(Result<(), String>),

    // No-op marker (used by async tasks whose result should not trigger anything).
    NoOp,

    // Cell editing
    CellClicked(usize, usize),
    CellEdited(String),
    CellEditCommit,
    CellEditSubmit,
    CellEditCancel,
    CellEditBegin,
    CellMove(i32, i32),
    CellClear,

    // Rows & columns
    AddRow,
    AddColumn(ColumnType),
    ColumnTypeChanged(usize, ColumnType),
    ColumnNameChanged(usize, String),
    ColumnPrecisionChanged(usize, u8),
    ColumnThousandsToggled(usize),
    ColumnCurrencySymbolChanged(usize, String),

    // Column formula
    FormulaOpenEditor(usize),
    FormulaChanged(String),
    FormulaEditCommit,
    FormulaEditCancel,

    // Formula autocomplete popover
    FormulaSuggestionMove(i32),
    FormulaSuggestionAccept,
    FormulaSuggestionClick(usize),
    FormulaEscape,

    // Sort
    TableSort(&'static str),
    SortColumn(Option<usize>),
    ToggleSortDirection,

    // Group
    GroupByColumn(Option<usize>),
    ToggleGroup(usize),

    // Row context actions
    RowMenuToggle(Option<usize>),
    ColumnSettingsToggle(Option<usize>),
    CutRow(usize),
    CopyRow(usize),
    PasteRow(usize),
    DeleteRow(usize),

    // Cell context actions
    CutCell(usize, usize),
    CopyCell(usize, usize),
    PasteCell(usize, usize),
    ClearCell(usize, usize),

    // Row inserts at position
    InsertRowAbove(usize),
    InsertRowBelow(usize),

    // Column ops
    InsertColumnLeft(usize),
    InsertColumnRight(usize),
    DeleteColumn(usize),

    // Table resize (longbridge)
    TableResize(ResizeEvent),

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
    pub selected_cell: Option<(usize, usize)>,
    pub edit_value: String,
    pub groups: Option<Vec<Group>>,
    pub clipboard_row: Option<Vec<CellValue>>,
    pub clipboard_cell: Option<CellValue>,
    pub formula: FormulaEditorState,

    // Table resize state (longbridge ResizeState)
    pub table_resize: ResizeState,

    // Transient UI state
    pub row_menu_open: Option<usize>,
    pub column_settings_open: Option<usize>,
    pub menubar_open: Option<usize>,

    // Theme + notifications
    pub theme: AppTheme,
    pub notifications: NotificationManager,

    // Unsaved-changes tracker
    pub dirty: bool,

    // Recent files (most-recent first)
    pub recent_files: Vec<std::path::PathBuf>,
}

impl TableApp {
    fn new() -> Self {
        let sheet = Sheet::new_empty();
        let widths = sheet.columns.iter().map(|c| c.width).collect();
        let mut app = TableApp {
            sheet,
            editing: None,
            selected_cell: None,
            edit_value: String::new(),
            groups: None,
            clipboard_row: None,
            clipboard_cell: None,
            formula: FormulaEditorState::default(),
            table_resize: ResizeState::new(widths).min_width(60.0).max_width(800.0),
            row_menu_open: None,
            column_settings_open: None,
            menubar_open: None,
            theme: AppTheme::dark(),
            notifications: NotificationManager::new(),
            dirty: false,
            recent_files: data::recent::load(),
        };
        evaluate_all_formulas(&mut app.sheet);
        app.recompute_groups();
        app
    }

    fn rebuild_resize_state(&mut self) {
        let widths = self.sheet.columns.iter().map(|c| c.width).collect();
        self.table_resize = ResizeState::new(widths).min_width(60.0).max_width(800.0);
    }

    fn sync_widths_from_resize(&mut self) {
        let ws = self.table_resize.widths();
        for (i, w) in ws.iter().enumerate() {
            if let Some(col) = self.sheet.columns.get_mut(i) {
                col.width = *w;
            }
        }
    }

    fn title(&self) -> String {
        let marker = if self.dirty { "• " } else { "" };
        match self.sheet.file_path {
            Some(ref p) => format!(
                "{}Table RS — {}",
                marker,
                p.file_name().and_then(|s| s.to_str()).unwrap_or("untitled")
            ),
            None => format!("{}Table RS", marker),
        }
    }

    fn theme(&self) -> Theme {
        self.theme.iced_theme()
    }

    fn subscription(&self) -> Subscription<Message> {
        let tick = iced::time::every(std::time::Duration::from_millis(250))
            .map(|_| Message::NotifyTick);
        let keys = iced::event::listen_with(app::keyboard::handle_key_event);
        Subscription::batch(vec![tick, keys])
    }

    pub fn recompute_groups(&mut self) {
        self.groups = group_rows(&self.sheet);
    }

    fn commit_edit(&mut self) {
        if let Some((row, col)) = self.editing.take() {
            let value = std::mem::take(&mut self.edit_value);
            let before = self
                .sheet
                .rows
                .get(row)
                .and_then(|r| r.get(col))
                .map(|c| c.edit_value())
                .unwrap_or_default();
            if before != value {
                self.dirty = true;
            }
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
                self.formula.maybe_check_error(&self.sheet);
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
            Message::ColumnSettingsToggle(col) => {
                self.column_settings_open = match (self.column_settings_open, col) {
                    (Some(cur), Some(c)) if cur == c => None,
                    _ => col,
                };
                match self.column_settings_open {
                    Some(c) if matches!(
                        self.sheet.columns.get(c).map(|d| &d.col_type),
                        Some(ColumnType::Formula)
                    ) =>
                    {
                        if self.formula.editing_col != Some(c) {
                            self.formula.open(c, &self.sheet);
                        }
                    }
                    _ => {
                        if self.formula.editing_col.is_some() {
                            self.formula.close();
                        }
                    }
                }
            }
            Message::TableResize(event) => {
                let was_release = matches!(event, ResizeEvent::Release);
                self.table_resize.apply(event);
                self.sync_widths_from_resize();
                if was_release {
                    self.dirty = true;
                }
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
                    self.dirty = true;
                }
                self.row_menu_open = None;
            }
            Message::PasteRow(row) => {
                if let Some(clipboard) = self.clipboard_row.clone() {
                    self.sheet.insert_row_after(row, clipboard);
                    self.recompute_groups();
                    self.dirty = true;
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
                self.dirty = true;
                self.row_menu_open = None;
            }
            Message::CopyCell(row, col) => {
                self.commit_edit();
                if let Some(value) = self.sheet.rows.get(row).and_then(|r| r.get(col)).cloned() {
                    self.clipboard_cell = Some(value);
                }
            }
            Message::CutCell(row, col) => {
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some() || c.col_type == ColumnType::Formula) {
                    return Task::none();
                }
                self.commit_edit();
                if let Some(value) = self.sheet.rows.get(row).and_then(|r| r.get(col)).cloned() {
                    self.clipboard_cell = Some(value);
                    self.sheet.set_cell(row, col, String::new());
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                    self.dirty = true;
                }
            }
            Message::PasteCell(row, col) => {
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some() || c.col_type == ColumnType::Formula) {
                    return Task::none();
                }
                self.commit_edit();
                if let Some(ref value) = self.clipboard_cell {
                    let raw = value.edit_value();
                    self.sheet.set_cell(row, col, raw);
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                    self.dirty = true;
                }
            }
            Message::ClearCell(row, col) => {
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some() || c.col_type == ColumnType::Formula) {
                    return Task::none();
                }
                self.commit_edit();
                if row < self.sheet.rows.len() && col < self.sheet.columns.len() {
                    self.sheet.set_cell(row, col, String::new());
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                    self.dirty = true;
                }
            }
            Message::InsertRowAbove(row) => {
                self.commit_edit();
                self.sheet.insert_blank_row_at(row);
                if let Some((er, ec)) = self.editing {
                    if er >= row {
                        self.editing = Some((er + 1, ec));
                    }
                }
                if let Some((sr, sc)) = self.selected_cell {
                    if sr >= row {
                        self.selected_cell = Some((sr + 1, sc));
                    }
                }
                self.recompute_groups();
                self.dirty = true;
            }
            Message::InsertRowBelow(row) => {
                self.commit_edit();
                self.sheet.insert_blank_row_at(row + 1);
                if let Some((er, ec)) = self.editing {
                    if er > row {
                        self.editing = Some((er + 1, ec));
                    }
                }
                if let Some((sr, sc)) = self.selected_cell {
                    if sr > row {
                        self.selected_cell = Some((sr + 1, sc));
                    }
                }
                self.recompute_groups();
                self.dirty = true;
            }
            Message::InsertColumnLeft(col) => {
                self.commit_edit();
                let name = format!("Column {}", self.sheet.col_count() + 1);
                self.sheet.insert_column_at(col, name, ColumnType::Text);
                if let Some(ref mut s) = self.sheet.sort {
                    if s.column >= col {
                        s.column += 1;
                    }
                }
                if let Some(g) = self.sheet.group_by {
                    if g >= col {
                        self.sheet.group_by = Some(g + 1);
                    }
                }
                if let Some((sr, sc)) = self.selected_cell {
                    if sc >= col {
                        self.selected_cell = Some((sr, sc + 1));
                    }
                }
                self.editing = None;
                self.recompute_groups();
                self.rebuild_resize_state();
                self.dirty = true;
            }
            Message::InsertColumnRight(col) => {
                self.commit_edit();
                let name = format!("Column {}", self.sheet.col_count() + 1);
                let insert_at = col + 1;
                self.sheet.insert_column_at(insert_at, name, ColumnType::Text);
                if let Some(ref mut s) = self.sheet.sort {
                    if s.column >= insert_at {
                        s.column += 1;
                    }
                }
                if let Some(g) = self.sheet.group_by {
                    if g >= insert_at {
                        self.sheet.group_by = Some(g + 1);
                    }
                }
                if let Some((sr, sc)) = self.selected_cell {
                    if sc >= insert_at {
                        self.selected_cell = Some((sr, sc + 1));
                    }
                }
                self.editing = None;
                self.recompute_groups();
                self.rebuild_resize_state();
                self.dirty = true;
            }
            Message::DeleteColumn(col) => {
                if self.sheet.col_count() <= 1 {
                    return Task::none();
                }
                self.commit_edit();
                self.sheet.delete_column(col);
                self.sheet.sort = match self.sheet.sort.take() {
                    Some(s) if s.column == col => None,
                    Some(mut s) => {
                        if s.column > col {
                            s.column -= 1;
                        }
                        Some(s)
                    }
                    None => None,
                };
                self.sheet.group_by = match self.sheet.group_by {
                    Some(g) if g == col => None,
                    Some(g) if g > col => Some(g - 1),
                    other => other,
                };
                if let Some((sr, sc)) = self.selected_cell {
                    if sc == col {
                        self.selected_cell = None;
                    } else if sc > col {
                        self.selected_cell = Some((sr, sc - 1));
                    }
                }
                self.editing = None;
                sort_rows(&mut self.sheet);
                self.recompute_groups();
                self.rebuild_resize_state();
                self.dirty = true;
            }
            Message::FileOpen => {
                self.menubar_open = None;
                return self.begin_file_open(None);
            }
            Message::FileOpenRecent(path) => {
                self.menubar_open = None;
                return self.begin_file_open(Some(path));
            }
            Message::FileOpenConfirmed(None) => {
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
            Message::FileOpenConfirmed(Some(path)) => {
                return Task::perform(
                    async move { data::csv_io::load(&path).map_err(|e| e.to_string()) },
                    Message::FileLoaded,
                );
            }
            Message::NoOp => {}
            Message::FileLoaded(result) => match result {
                Ok(mut sheet) => {
                    evaluate_all_formulas(&mut sheet);
                    self.sheet = sheet;
                    self.editing = None;
                    self.selected_cell = None;
                    self.dirty = false;
                    self.recompute_groups();
                    sort_rows(&mut self.sheet);
                    self.rebuild_resize_state();
                    if let Some(path) = self.sheet.file_path.clone() {
                        self.remember_recent(&path);
                    }
                    let name = self
                        .sheet
                        .file_path
                        .as_ref()
                        .and_then(|p| p.file_name())
                        .and_then(|s| s.to_str())
                        .unwrap_or("file")
                        .to_string();
                    self.notifications
                        .notify_msg(NotificationKind::Success, "Loaded", name);
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.notifications
                        .notify_msg(NotificationKind::Error, "Load failed", e);
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
                    self.dirty = false;
                    if let Some(path) = self.sheet.file_path.clone() {
                        self.remember_recent(&path);
                    }
                    self.notifications.notify(NotificationKind::Success, "Saved");
                }
                Err(e) if e == "Cancelled" => {}
                Err(e) => {
                    self.notifications
                        .notify_msg(NotificationKind::Error, "Save failed", e);
                }
            },
            Message::CellClicked(row, col) => {
                if matches!(self.sheet.columns.get(col), Some(c) if c.formula.is_some()) {
                    return Task::none();
                }
                self.commit_edit();
                if row < self.sheet.rows.len() && col < self.sheet.columns.len() {
                    self.selected_cell = Some((row, col));
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
            Message::CellEditSubmit => {
                self.commit_edit();
                if let Some((r, c)) = self.selected_cell {
                    let last = self.sheet.rows.len().saturating_sub(1);
                    let nr = (r + 1).min(last);
                    self.selected_cell = Some((nr, c));
                }
            }
            Message::CellEditCancel => {
                self.editing = None;
                self.edit_value.clear();
            }
            Message::CellEditBegin => {
                if self.editing.is_some() {
                    return Task::none();
                }
                if let Some((r, c)) = self.selected_cell {
                    if matches!(self.sheet.columns.get(c), Some(col) if col.formula.is_some()) {
                        return Task::none();
                    }
                    if r < self.sheet.rows.len() && c < self.sheet.columns.len() {
                        self.editing = Some((r, c));
                        self.edit_value = self.sheet.rows[r][c].edit_value();
                    }
                }
            }
            Message::CellMove(dr, dc) => {
                if self.editing.is_some() {
                    return Task::none();
                }
                let rows = self.sheet.row_count() as i32;
                let cols = self.sheet.col_count() as i32;
                if rows == 0 || cols == 0 {
                    return Task::none();
                }
                let (r, c) = self.selected_cell.unwrap_or((0, 0));
                let nr = (r as i32 + dr).clamp(0, rows - 1) as usize;
                let nc = (c as i32 + dc).clamp(0, cols - 1) as usize;
                self.selected_cell = Some((nr, nc));
            }
            Message::CellClear => {
                if self.editing.is_some() {
                    return Task::none();
                }
                if let Some((r, c)) = self.selected_cell {
                    if matches!(self.sheet.columns.get(c), Some(col) if col.formula.is_some()) {
                        return Task::none();
                    }
                    self.sheet.set_cell(r, c, String::new());
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                    self.dirty = true;
                }
            }
            Message::TableSort(key) => {
                if let Some(col) = col_for_sort_key(key) {
                    // Formula columns: open formula editor instead of sorting.
                    if matches!(
                        self.sheet.columns.get(col),
                        Some(c) if c.col_type == ColumnType::Formula
                    ) {
                        self.formula.open(col, &self.sheet);
                        return Task::none();
                    }
                    self.dirty = true;
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
                    let became_formula = matches!(new_type, ColumnType::Formula);
                    self.sheet.columns[col].col_type = new_type.clone();
                    for row in &mut self.sheet.rows {
                        if let Some(cell) = row.get(col) {
                            let raw = cell.edit_value();
                            row[col] = data::parse_cell_value(&raw, &new_type);
                        }
                    }
                    evaluate_all_formulas(&mut self.sheet);
                    self.recompute_groups();
                    self.dirty = true;
                    if became_formula && self.formula.editing_col != Some(col) {
                        self.formula.open(col, &self.sheet);
                    } else if !became_formula && self.formula.editing_col == Some(col) {
                        self.formula.close();
                    }
                }
            }
            Message::ColumnNameChanged(col, name) => {
                if let Some(c) = self.sheet.columns.get_mut(col) {
                    c.name = name;
                    self.dirty = true;
                }
            }
            Message::ColumnPrecisionChanged(col, p) => {
                if let Some(c) = self.sheet.columns.get_mut(col) {
                    c.format.precision = p.min(6);
                    self.dirty = true;
                }
            }
            Message::ColumnThousandsToggled(col) => {
                if let Some(c) = self.sheet.columns.get_mut(col) {
                    c.format.thousands = !c.format.thousands;
                    self.dirty = true;
                }
            }
            Message::ColumnCurrencySymbolChanged(col, sym) => {
                if let Some(c) = self.sheet.columns.get_mut(col) {
                    if matches!(c.col_type, ColumnType::Currency(_)) {
                        c.col_type = ColumnType::Currency(sym);
                        self.dirty = true;
                    }
                }
            }
            Message::AddRow => {
                self.sheet.add_row();
                self.recompute_groups();
                self.dirty = true;
            }
            Message::AddColumn(col_type) => {
                let name = format!("Column {}", self.sheet.col_count() + 1);
                let is_formula = col_type == ColumnType::Formula;
                self.sheet.add_column(name, col_type);
                if is_formula {
                    let new_col = self.sheet.columns.len() - 1;
                    self.formula.open(new_col, &self.sheet);
                }
                self.recompute_groups();
                self.rebuild_resize_state();
                self.dirty = true;
            }
            Message::FormulaOpenEditor(col) => {
                self.formula.open(col, &self.sheet);
            }
            Message::FormulaChanged(value) => {
                self.formula.handle_changed(value, &self.sheet);
            }
            Message::FormulaEditCommit => {
                let result = self.formula.handle_commit(&mut self.sheet);
                if result.accepted_suggestion {
                    return Task::none();
                }
                if result.formula_changed {
                    self.dirty = true;
                }
                self.recompute_groups();
                self.column_settings_open = None;
            }
            Message::FormulaEditCancel => {
                self.formula.close();
                self.column_settings_open = None;
            }
            Message::FormulaSuggestionMove(delta) => {
                self.formula.handle_suggestion_move(delta, &self.sheet);
            }
            Message::FormulaSuggestionAccept => {
                self.formula.handle_suggestion_accept(&self.sheet);
            }
            Message::FormulaSuggestionClick(idx) => {
                self.formula.handle_suggestion_click(&self.sheet, idx);
            }
            Message::FormulaEscape => {
                self.formula.handle_escape(&self.sheet);
            }
            Message::SortColumn(col) => {
                self.sheet.sort = col.map(|c| SortConfig {
                    column: c,
                    direction: SortDirection::Ascending,
                });
                sort_rows(&mut self.sheet);
                self.recompute_groups();
                self.dirty = true;
            }
            Message::ToggleSortDirection => {
                if let Some(ref mut s) = self.sheet.sort {
                    s.direction = match s.direction {
                        SortDirection::Ascending => SortDirection::Descending,
                        SortDirection::Descending => SortDirection::Ascending,
                    };
                    self.dirty = true;
                }
                sort_rows(&mut self.sheet);
                self.recompute_groups();
            }
            Message::GroupByColumn(col) => {
                self.sheet.group_by = col;
                self.recompute_groups();
                self.dirty = true;
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

    fn begin_file_open(&self, path: Option<std::path::PathBuf>) -> Task<Message> {
        if self.dirty {
            let path_for_confirm = path.clone();
            return Task::perform(
                async {
                    rfd::AsyncMessageDialog::new()
                        .set_title("Unsaved changes")
                        .set_description("Discard unsaved changes and open another file?")
                        .set_buttons(rfd::MessageButtons::OkCancel)
                        .show()
                        .await
                },
                move |result| match result {
                    rfd::MessageDialogResult::Ok | rfd::MessageDialogResult::Yes => {
                        Message::FileOpenConfirmed(path_for_confirm.clone())
                    }
                    _ => Message::NoOp,
                },
            );
        }
        Task::done(Message::FileOpenConfirmed(path))
    }

    fn remember_recent(&mut self, path: &std::path::Path) {
        data::recent::push(&mut self.recent_files, path);
        let snapshot = self.recent_files.clone();
        std::thread::spawn(move || {
            let _ = data::recent::save(&snapshot);
        });
    }

    fn view(&self) -> Element<'_, Message> {
        let theme = &self.theme;

        let theme_toggle_kind = match self.theme.appearance {
            Appearance::Light => IconKind::Moon,
            Appearance::Dark => IconKind::Sun,
        };
        let theme_toggle: Element<'_, Message> = icon_button(
            theme,
            icon(theme, theme_toggle_kind, 16.0),
            Variant::Ghost,
            Size::Sm,
            Some(Message::ThemeToggle),
            false,
        );

        let mut file_items = vec![
            Item::new("Open", Message::FileOpen)
                .icon(IconKind::FolderOpen)
                .shortcut("Ctrl+O"),
            Item::new("Save", Message::FileSave)
                .icon(IconKind::Save)
                .shortcut("Ctrl+S"),
        ];
        if !self.recent_files.is_empty() {
            file_items.push(Item::Separator);
            file_items.push(Item::Header("Recent".into()));
            for path in &self.recent_files {
                let label = path
                    .file_name()
                    .and_then(|s| s.to_str())
                    .unwrap_or("(unknown)")
                    .to_string();
                file_items.push(Item::new(label, Message::FileOpenRecent(path.clone())));
            }
        }
        let file_menu = MenuBarMenu::new("File", Message::MenuBarToggle(0), file_items);
        let menubar = menu_bar(theme, vec![file_menu], self.menubar_open);

        let t = *theme;
        let chrome = container(
            row![
                icon(theme, IconKind::PanelLeft, 14.0),
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

        notification_layer(theme, base, &self.notifications.list, Message::NotifyDismiss)
    }
}
