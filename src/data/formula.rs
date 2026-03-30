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

/// Evaluate a single formula cell in the sheet.
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

/// Re-evaluate all formula cells in the sheet.
/// Simple approach: iterate multiple passes until values stabilize (handles dependencies).
pub fn evaluate_all_formulas(sheet: &mut Sheet) {
    let max_passes = 10;
    for _ in 0..max_passes {
        let mut changed = false;
        for r in 0..sheet.rows.len() {
            for c in 0..sheet.rows[r].len() {
                if let CellValue::Formula { ref expr, cached } = sheet.rows[r][c].clone() {
                    let new_val = evaluate_formula(sheet, &expr).ok();
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
