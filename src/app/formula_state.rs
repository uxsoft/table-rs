use std::time::{Duration, Instant};

use crate::data::{self, formula::evaluate_all_formulas, Sheet};

#[derive(Default)]
pub struct FormulaEditorState {
    pub editing_col: Option<usize>,
    pub value: String,
    pub suggestions_selected: usize,
    pub autocomplete_suppressed: bool,
    pub last_edit: Option<Instant>,
    pub error: Option<String>,
    pub error_checked_for: Option<String>,
}

pub struct CommitResult {
    /// True if the popover was open and a suggestion was accepted; the editor
    /// stays open. The caller should not advance application state further.
    pub accepted_suggestion: bool,
    /// True if the column's formula text was modified by the commit. Caller
    /// should mark the document dirty.
    pub formula_changed: bool,
}

impl FormulaEditorState {
    pub fn open(&mut self, col: usize, sheet: &Sheet) {
        if col < sheet.columns.len() {
            let current = sheet.columns[col].formula.clone().unwrap_or_default();
            self.editing_col = Some(col);
            self.value = current;
            self.suggestions_selected = 0;
            self.autocomplete_suppressed = false;
            self.last_edit = Some(Instant::now());
            self.error = None;
            self.error_checked_for = None;
        }
    }

    pub fn close(&mut self) {
        self.editing_col = None;
        self.value.clear();
        self.suggestions_selected = 0;
        self.autocomplete_suppressed = false;
        self.last_edit = None;
        self.error = None;
        self.error_checked_for = None;
    }

    /// Current autocomplete suggestions. Returns `(column_index, name)` pairs.
    /// Empty when the popover should not show.
    pub fn suggestions(&self, sheet: &Sheet) -> Vec<(usize, String)> {
        if self.editing_col.is_none() || self.autocomplete_suppressed {
            return Vec::new();
        }
        let Some((_, partial)) = data::formula::last_open_brace_partial(&self.value) else {
            return Vec::new();
        };
        let names: Vec<&str> = sheet.columns.iter().map(|c| c.name.as_str()).collect();
        data::formula::match_suggestions(&names, partial, self.editing_col)
            .into_iter()
            .map(|(i, n)| (i, n.to_string()))
            .collect()
    }

    fn suggestion_names(&self, sheet: &Sheet) -> Vec<String> {
        self.suggestions(sheet).into_iter().map(|(_, n)| n).collect()
    }

    pub fn apply_suggestion(&mut self, sheet: &Sheet, idx_in_suggestions: usize) {
        let suggestions = self.suggestions(sheet);
        if suggestions.is_empty() {
            return;
        }
        let idx = idx_in_suggestions.min(suggestions.len() - 1);
        let name = suggestions[idx].1.clone();
        if let Some((brace_idx, _)) = data::formula::last_open_brace_partial(&self.value) {
            self.value = data::formula::apply_suggestion(&self.value, brace_idx, &name);
            self.suggestions_selected = 0;
            self.autocomplete_suppressed = false;
            self.last_edit = Some(Instant::now());
        }
    }

    pub fn handle_changed(&mut self, value: String, sheet: &Sheet) {
        let prev_names = self.suggestion_names(sheet);
        self.value = value;
        self.autocomplete_suppressed = false;
        let new_names = self.suggestion_names(sheet);
        if new_names != prev_names {
            self.suggestions_selected = 0;
        }
        self.last_edit = Some(Instant::now());
    }

    pub fn handle_suggestion_move(&mut self, delta: i32, sheet: &Sheet) {
        let suggestions = self.suggestions(sheet);
        if suggestions.is_empty() {
            return;
        }
        let len = suggestions.len() as i32;
        let cur = self.suggestions_selected as i32;
        self.suggestions_selected = (cur + delta).clamp(0, len - 1) as usize;
    }

    pub fn handle_suggestion_accept(&mut self, sheet: &Sheet) {
        if !self.suggestions(sheet).is_empty() {
            self.apply_suggestion(sheet, self.suggestions_selected);
        }
    }

    pub fn handle_suggestion_click(&mut self, sheet: &Sheet, idx: usize) {
        if !self.suggestions(sheet).is_empty() {
            self.apply_suggestion(sheet, idx);
        }
    }

    pub fn handle_escape(&mut self, sheet: &Sheet) {
        if !self.suggestions(sheet).is_empty() {
            self.autocomplete_suppressed = true;
            self.suggestions_selected = 0;
        }
    }

    pub fn handle_commit(&mut self, sheet: &mut Sheet) -> CommitResult {
        if !self.suggestions(sheet).is_empty() {
            self.apply_suggestion(sheet, self.suggestions_selected);
            return CommitResult {
                accepted_suggestion: true,
                formula_changed: false,
            };
        }
        let mut changed = false;
        if let Some(col) = self.editing_col {
            let expr = self.value.trim().to_string();
            if col < sheet.columns.len() {
                let new_formula = if expr.is_empty() { None } else { Some(expr) };
                if sheet.columns[col].formula != new_formula {
                    changed = true;
                }
                sheet.columns[col].formula = new_formula;
                evaluate_all_formulas(sheet);
            }
        }
        self.close();
        CommitResult {
            accepted_suggestion: false,
            formula_changed: changed,
        }
    }

    /// Debounced re-evaluation of the in-progress formula. Driven by the
    /// 250 ms `NotifyTick` subscription; runs at most once per settled edit
    /// (tracked via `error_checked_for`). Evaluates against row 0 to surface
    /// structural errors like unknown column names.
    pub fn maybe_check_error(&mut self, sheet: &Sheet) {
        if self.editing_col.is_none() {
            return;
        }
        let Some(last_edit) = self.last_edit else {
            return;
        };
        if last_edit.elapsed() < Duration::from_millis(300) {
            return;
        }
        if self.error_checked_for.as_deref() == Some(self.value.as_str()) {
            return;
        }
        let value = self.value.clone();
        let trimmed = value.trim();
        if trimmed.is_empty() || sheet.rows.is_empty() {
            self.error = None;
        } else {
            match data::formula::evaluate_column_formula(sheet, trimmed, 0) {
                Ok(_) => self.error = None,
                Err(msg) => self.error = Some(msg),
            }
        }
        self.error_checked_for = Some(value);
    }
}
