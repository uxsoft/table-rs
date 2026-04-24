use evalexpr::*;
use regex::Regex;

use super::{CellValue, Sheet, letter_to_col_index};

/// Parse a cell reference like "A1" into (col_index, row_index).
/// Column is letter(s), row is 1-based number.
fn parse_cell_ref(reference: &str) -> Option<(usize, usize)> {
    let re = Regex::new(r"^([A-Z]+)(\d+)$").unwrap();
    let caps = re.captures(reference)?;
    let col = letter_to_col_index(&caps[1])?;
    let row: usize = caps[2].parse().ok()?;
    if row == 0 {
        return None;
    }
    Some((col, row - 1)) // convert to 0-based row
}

/// Expand range functions like SUM(A1:A5) into evalexpr-compatible expressions.
fn expand_range_functions(expr: &str, sheet: &Sheet) -> String {
    let range_re =
        Regex::new(r"(SUM|AVG|MIN|MAX|COUNT)\(([A-Z]+\d+):([A-Z]+\d+)\)").unwrap();

    let mut result = expr.to_string();

    // Iterate and replace range functions
    while let Some(caps) = range_re.captures(&result) {
        let full_match = caps.get(0).unwrap();
        let func = &caps[1];
        let start_ref = &caps[2];
        let end_ref = &caps[3];

        let replacement = if let (Some((sc, sr)), Some((ec, er))) =
            (parse_cell_ref(start_ref), parse_cell_ref(end_ref))
        {
            let mut values = Vec::new();
            for r in sr..=er {
                for c in sc..=ec {
                    if let Some(row) = sheet.rows.get(r) {
                        if let Some(cell) = row.get(c) {
                            if let Some(v) = cell.as_f64() {
                                values.push(v);
                            }
                        }
                    }
                }
            }

            match func {
                "SUM" => {
                    let sum: f64 = values.iter().sum();
                    format!("{}", sum)
                }
                "AVG" => {
                    if values.is_empty() {
                        "0".to_string()
                    } else {
                        let sum: f64 = values.iter().sum();
                        format!("{}", sum / values.len() as f64)
                    }
                }
                "MIN" => {
                    values
                        .iter()
                        .cloned()
                        .reduce(f64::min)
                        .map(|v| format!("{}", v))
                        .unwrap_or_else(|| "0".to_string())
                }
                "MAX" => {
                    values
                        .iter()
                        .cloned()
                        .reduce(f64::max)
                        .map(|v| format!("{}", v))
                        .unwrap_or_else(|| "0".to_string())
                }
                "COUNT" => format!("{}", values.len()),
                _ => "0".to_string(),
            }
        } else {
            "0".to_string()
        };

        result = result[..full_match.start()].to_string()
            + &replacement
            + &result[full_match.end()..];
    }

    result
}

/// Replace individual cell references (A1, B2) with their numeric values.
fn resolve_cell_references(expr: &str, sheet: &Sheet) -> String {
    let cell_re = Regex::new(r"([A-Z]+)(\d+)").unwrap();
    let mut result = String::new();
    let mut last_end = 0;

    for caps in cell_re.captures_iter(expr) {
        let m = caps.get(0).unwrap();
        result.push_str(&expr[last_end..m.start()]);

        let full_ref = m.as_str();
        if let Some((col, row)) = parse_cell_ref(full_ref) {
            if let Some(r) = sheet.rows.get(row) {
                if let Some(cell) = r.get(col) {
                    if let Some(v) = cell.as_f64() {
                        result.push_str(&format!("{}", v));
                    } else {
                        result.push('0');
                    }
                } else {
                    result.push('0');
                }
            } else {
                result.push('0');
            }
        } else {
            result.push_str(full_ref);
        }

        last_end = m.end();
    }
    result.push_str(&expr[last_end..]);
    result
}

/// Evaluate a single formula cell in the sheet using absolute cell references (A1, B2…).
pub fn evaluate_formula(sheet: &Sheet, expr: &str) -> Result<f64, String> {
    // First expand range functions
    let expanded = expand_range_functions(expr, sheet);
    // Then resolve individual cell references
    let resolved = resolve_cell_references(&expanded, sheet);
    // Evaluate with evalexpr
    match eval(&resolved) {
        Ok(Value::Float(f)) => Ok(f),
        Ok(Value::Int(i)) => Ok(i as f64),
        Ok(other) => Err(format!("Expected number, got: {:?}", other)),
        Err(e) => Err(format!("{}", e)),
    }
}

/// Locate the last unclosed `{` in `s` and return its byte index plus the
/// partial text after it. Returns `None` when every `{` has a matching `}`
/// after it (or when there's no `{` at all).
///
/// Used by the formula editor to decide whether to show the column-name
/// autocomplete popover and what to match against.
pub fn last_open_brace_partial(s: &str) -> Option<(usize, &str)> {
    let open_idx = s.rfind('{')?;
    let partial = &s[open_idx + 1..];
    if partial.contains('}') {
        return None;
    }
    Some((open_idx, partial))
}

/// Replace the substring of `formula` from `brace_idx` to the end with
/// `{name}`. Used to accept an autocomplete suggestion: `{Val` + `Value`
/// → `{Value}`.
pub fn apply_suggestion(formula: &str, brace_idx: usize, name: &str) -> String {
    let mut out = String::with_capacity(brace_idx + name.len() + 2);
    out.push_str(&formula[..brace_idx]);
    out.push('{');
    out.push_str(name);
    out.push('}');
    out
}

/// Filter `names` to those starting with `partial` (case-insensitive),
/// preserving order. Returns `(original_index, name)` pairs. When
/// `exclude_idx` is set, that index is skipped (self-reference makes no
/// sense in a column formula).
pub fn match_suggestions<'a>(
    names: &[&'a str],
    partial: &str,
    exclude_idx: Option<usize>,
) -> Vec<(usize, &'a str)> {
    let partial_lower = partial.to_lowercase();
    names
        .iter()
        .enumerate()
        .filter(|(i, _)| Some(*i) != exclude_idx)
        .filter(|(_, n)| n.to_lowercase().starts_with(&partial_lower))
        .map(|(i, n)| (i, *n))
        .collect()
}

/// Evaluate a column-level formula (Airtable-style) for a specific row.
///
/// The formula uses `{ColumnName}` placeholders that are resolved to the
/// numeric value of that column in the given row. Example: `{Value} * {Price}`.
pub fn evaluate_column_formula(sheet: &Sheet, expr: &str, row_idx: usize) -> Result<f64, String> {
    if row_idx >= sheet.rows.len() {
        return Err("Row out of bounds".to_string());
    }

    // Replace every {ColumnName} with the row's value for that column.
    // We do this by scanning for all {…} tokens.
    let placeholder_re = Regex::new(r"\{([^}]+)\}").unwrap();
    let row = &sheet.rows[row_idx];

    let mut resolved = String::new();
    let mut last_end = 0;

    for caps in placeholder_re.captures_iter(expr) {
        let full = caps.get(0).unwrap();
        let col_name = &caps[1];

        resolved.push_str(&expr[last_end..full.start()]);

        let Some(ci) = sheet.columns.iter().position(|c| c.name == col_name) else {
            return Err(format!("Unknown column: {col_name}"));
        };
        let value = row.get(ci).and_then(|cell| cell.as_f64()).unwrap_or(0.0);

        resolved.push_str(&format!("{}", value));
        last_end = full.end();
    }
    resolved.push_str(&expr[last_end..]);

    match eval(&resolved) {
        Ok(Value::Float(f)) => Ok(f),
        Ok(Value::Int(i)) => Ok(i as f64),
        Ok(other) => Err(format!("Expected number, got: {:?}", other)),
        Err(e) => Err(format!("Formula error: {}", e)),
    }
}

/// Re-evaluate all formula cells in the sheet.
///
/// - Columns with `ColumnType::Formula` and a `column.formula` expression use
///   the column-level Airtable-style evaluation (`{ColumnName}` placeholders).
/// - Individual `CellValue::Formula` cells (legacy per-cell mode) are still
///   evaluated using absolute cell references for backward compatibility.
pub fn evaluate_all_formulas(sheet: &mut Sheet) {
    // Pass 1: column-level formulas — overwrite every cell in the column.
    for col_idx in 0..sheet.columns.len() {
        if sheet.columns[col_idx].col_type != super::ColumnType::Formula {
            continue;
        }
        let Some(expr) = sheet.columns[col_idx].formula.clone() else {
            continue;
        };
        for row_idx in 0..sheet.rows.len() {
            let result = evaluate_column_formula(sheet, &expr, row_idx).ok();
            sheet.rows[row_idx][col_idx] = CellValue::Formula {
                expr: String::new(), // expression lives on the column, not the cell
                cached: result,
            };
        }
    }

    // Pass 2: legacy per-cell formulas (non-formula-typed columns, or cells
    // that had an expression stored before the column-level feature existed).
    let max_passes = 10;
    for _ in 0..max_passes {
        let mut changed = false;
        for r in 0..sheet.rows.len() {
            for c in 0..sheet.rows[r].len() {
                // Skip columns that are governed by a column-level formula.
                if sheet.columns[c].col_type == super::ColumnType::Formula
                    && sheet.columns[c].formula.is_some()
                {
                    continue;
                }
                if let CellValue::Formula { ref expr, cached } = sheet.rows[r][c].clone() {
                    if expr.is_empty() {
                        continue;
                    }
                    let new_val = evaluate_formula(sheet, expr).ok();
                    if new_val != cached {
                        sheet.rows[r][c] = CellValue::Formula {
                            expr: expr.clone(),
                            cached: new_val,
                        };
                        changed = true;
                    }
                }
            }
        }
        if !changed {
            break;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{CellValue, ColumnDef, ColumnType, Sheet};

    fn sheet_with(columns: Vec<(&str, ColumnType)>, rows: Vec<Vec<CellValue>>) -> Sheet {
        Sheet {
            columns: columns
                .into_iter()
                .map(|(name, col_type)| ColumnDef {
                    name: name.to_string(),
                    col_type,
                    width: 100.0,
                    formula: None,
                })
                .collect(),
            rows,
            file_path: None,
            sort: None,
            group_by: None,
        }
    }

    #[test]
    fn evaluate_column_formula_errors_on_unknown_placeholder() {
        let sheet = sheet_with(
            vec![("Value", ColumnType::Number)],
            vec![vec![CellValue::Number(10.0)]],
        );
        let err = evaluate_column_formula(&sheet, "{Valu}", 0).expect_err("should error");
        assert!(
            err.contains("Unknown column"),
            "expected unknown-column error, got: {err}"
        );
        assert!(
            err.contains("Valu"),
            "expected error to name the missing column, got: {err}"
        );
    }

    #[test]
    fn evaluate_column_formula_tolerates_non_numeric_cells() {
        let sheet = sheet_with(
            vec![
                ("Value", ColumnType::Number),
                ("Other", ColumnType::Number),
            ],
            vec![vec![CellValue::Number(5.0), CellValue::Empty]],
        );
        let val = evaluate_column_formula(&sheet, "{Value} + {Other}", 0).unwrap();
        assert_eq!(val, 5.0);
    }

    #[test]
    fn last_open_brace_partial_none_when_no_brace() {
        assert_eq!(last_open_brace_partial(""), None);
        assert_eq!(last_open_brace_partial("abc + 1"), None);
    }

    #[test]
    fn last_open_brace_partial_returns_empty_partial_for_bare_brace() {
        assert_eq!(last_open_brace_partial("{"), Some((0, "")));
        assert_eq!(last_open_brace_partial("1 + {"), Some((4, "")));
    }

    #[test]
    fn last_open_brace_partial_returns_partial_text() {
        assert_eq!(last_open_brace_partial("{abc"), Some((0, "abc")));
        assert_eq!(last_open_brace_partial("1 + {Val"), Some((4, "Val")));
    }

    #[test]
    fn last_open_brace_partial_none_when_closed() {
        assert_eq!(last_open_brace_partial("{abc}"), None);
        assert_eq!(last_open_brace_partial("{a} + {b}"), None);
    }

    #[test]
    fn last_open_brace_partial_finds_last_of_multiple_opens() {
        assert_eq!(last_open_brace_partial("{a} + {xy"), Some((6, "xy")));
        assert_eq!(last_open_brace_partial("{foo}{bar"), Some((5, "bar")));
    }

    #[test]
    fn match_suggestions_case_insensitive_starts_with() {
        let names = vec!["Value", "Volume", "Price", "Quantity"];
        let got = match_suggestions(&names, "v", None);
        assert_eq!(got, vec![(0, "Value"), (1, "Volume")]);

        let got = match_suggestions(&names, "V", None);
        assert_eq!(got, vec![(0, "Value"), (1, "Volume")]);

        let got = match_suggestions(&names, "pri", None);
        assert_eq!(got, vec![(2, "Price")]);
    }

    #[test]
    fn match_suggestions_empty_partial_returns_all() {
        let names = vec!["A", "B", "C"];
        let got = match_suggestions(&names, "", None);
        assert_eq!(got, vec![(0, "A"), (1, "B"), (2, "C")]);
    }

    #[test]
    fn match_suggestions_excludes_self_column() {
        let names = vec!["Value", "Volume", "Price"];
        let got = match_suggestions(&names, "v", Some(0));
        assert_eq!(got, vec![(1, "Volume")]);
    }

    #[test]
    fn match_suggestions_returns_empty_when_nothing_matches() {
        let names = vec!["A", "B"];
        let got = match_suggestions(&names, "xyz", None);
        assert!(got.is_empty());
    }

    #[test]
    fn apply_suggestion_replaces_partial_and_closes_brace() {
        assert_eq!(apply_suggestion("{Val", 0, "Value"), "{Value}");
        assert_eq!(apply_suggestion("1 + {Val", 4, "Value"), "1 + {Value}");
        assert_eq!(apply_suggestion("{A} + {", 6, "Beta"), "{A} + {Beta}");
    }

    #[test]
    fn apply_suggestion_handles_empty_partial() {
        assert_eq!(apply_suggestion("{", 0, "Value"), "{Value}");
    }
}
