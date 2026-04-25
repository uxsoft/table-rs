pub mod csv_io;
pub mod formula;
pub mod operations;
pub mod recent;

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ColumnType {
    Text,
    Number,
    Currency(String), // currency symbol e.g. "$", "€"
    Formula,
}

impl std::fmt::Display for ColumnType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ColumnType::Text => write!(f, "Text"),
            ColumnType::Number => write!(f, "Number"),
            ColumnType::Currency(sym) => write!(f, "Currency ({})", sym),
            ColumnType::Formula => write!(f, "Formula"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ColumnDef {
    pub name: String,
    pub col_type: ColumnType,
    pub width: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub formula: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellValue {
    Text(String),
    Number(f64),
    Currency(f64),
    Formula {
        expr: String,
        cached: Option<f64>,
    },
    Empty,
}

impl CellValue {
    pub fn display_value(&self, currency_symbol: &str) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => {
                if *n == n.floor() {
                    format!("{}", *n as i64)
                } else {
                    format!("{:.2}", n)
                }
            }
            CellValue::Currency(n) => format!("{}{:.2}", currency_symbol, n),
            CellValue::Formula { cached: Some(v), .. } => {
                if *v == v.floor() {
                    format!("{}", *v as i64)
                } else {
                    format!("{:.2}", v)
                }
            }
            CellValue::Formula { cached: None, .. } => "#EVAL".to_string(),
            CellValue::Empty => String::new(),
        }
    }

    pub fn edit_value(&self) -> String {
        match self {
            CellValue::Text(s) => s.clone(),
            CellValue::Number(n) => format!("{}", n),
            CellValue::Currency(n) => format!("{}", n),
            CellValue::Formula { expr, .. } => format!("={}", expr),
            CellValue::Empty => String::new(),
        }
    }

    pub fn as_f64(&self) -> Option<f64> {
        match self {
            CellValue::Number(n) => Some(*n),
            CellValue::Currency(n) => Some(*n),
            CellValue::Formula { cached: Some(v), .. } => Some(*v),
            _ => None,
        }
    }

    pub fn sort_key(&self) -> String {
        match self {
            CellValue::Text(s) => s.to_lowercase(),
            CellValue::Number(n) => format!("{:020.6}", n),
            CellValue::Currency(n) => format!("{:020.6}", n),
            CellValue::Formula { cached: Some(v), .. } => format!("{:020.6}", v),
            CellValue::Formula { cached: None, .. } => String::new(),
            CellValue::Empty => String::new(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SortConfig {
    pub column: usize,
    pub direction: SortDirection,
}

#[derive(Debug, Clone)]
pub struct Group {
    pub key: String,
    pub row_indices: Vec<usize>,
    pub collapsed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SheetMeta {
    pub columns: Vec<ColumnDef>,
    #[serde(default)]
    pub sort: Option<SortConfig>,
    #[serde(default)]
    pub group_by: Option<usize>,
}

#[derive(Debug, Clone)]
pub struct Sheet {
    pub columns: Vec<ColumnDef>,
    pub rows: Vec<Vec<CellValue>>,
    pub file_path: Option<PathBuf>,
    pub sort: Option<SortConfig>,
    pub group_by: Option<usize>,
}

impl Sheet {
    pub fn new_empty() -> Self {
        Sheet {
            columns: vec![
                ColumnDef {
                    name: "Name".into(),
                    col_type: ColumnType::Text,
                    width: 150.0,
                    formula: None,
                },
                ColumnDef {
                    name: "Value".into(),
                    col_type: ColumnType::Number,
                    width: 100.0,
                    formula: None,
                },
                ColumnDef {
                    name: "Price".into(),
                    col_type: ColumnType::Currency("$".into()),
                    width: 120.0,
                    formula: None,
                },
            ],
            rows: vec![
                vec![
                    CellValue::Text("Item A".into()),
                    CellValue::Number(10.0),
                    CellValue::Currency(9.99),
                ],
                vec![
                    CellValue::Text("Item B".into()),
                    CellValue::Number(25.0),
                    CellValue::Currency(19.50),
                ],
                vec![
                    CellValue::Text("Item C".into()),
                    CellValue::Number(10.0),
                    CellValue::Currency(5.75),
                ],
            ],
            file_path: None,
            sort: None,
            group_by: None,
        }
    }

    pub fn add_row(&mut self) {
        let row = self
            .columns
            .iter()
            .map(|_| CellValue::Empty)
            .collect();
        self.rows.push(row);
    }

    pub fn add_column(&mut self, name: String, col_type: ColumnType) {
        let width = match &col_type {
            ColumnType::Text => 150.0,
            ColumnType::Number => 100.0,
            ColumnType::Currency(_) => 120.0,
            ColumnType::Formula => 120.0,
        };
        self.columns.push(ColumnDef {
            name,
            col_type,
            width,
            formula: None,
        });
        for row in &mut self.rows {
            row.push(CellValue::Empty);
        }
    }

    pub fn set_cell(&mut self, row: usize, col: usize, value: String) {
        if row >= self.rows.len() || col >= self.columns.len() {
            return;
        }
        let col_type = &self.columns[col].col_type;
        let cell = parse_cell_value(&value, col_type);
        self.rows[row][col] = cell;
    }

    pub fn col_count(&self) -> usize {
        self.columns.len()
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn delete_row(&mut self, index: usize) {
        if index < self.rows.len() {
            self.rows.remove(index);
        }
    }

    pub fn insert_row_at(&mut self, index: usize, row: Vec<CellValue>) {
        let insert_at = index.min(self.rows.len());
        self.rows.insert(insert_at, row);
    }

    pub fn insert_blank_row_at(&mut self, index: usize) {
        let row = self.columns.iter().map(|_| CellValue::Empty).collect();
        self.insert_row_at(index, row);
    }

    pub fn insert_row_after(&mut self, index: usize, row: Vec<CellValue>) {
        self.insert_row_at(index + 1, row);
    }

    pub fn insert_column_at(&mut self, index: usize, name: String, col_type: ColumnType) {
        let width = match &col_type {
            ColumnType::Text => 150.0,
            ColumnType::Number => 100.0,
            ColumnType::Currency(_) => 120.0,
            ColumnType::Formula => 120.0,
        };
        let insert_at = index.min(self.columns.len());
        self.columns.insert(
            insert_at,
            ColumnDef {
                name,
                col_type,
                width,
                formula: None,
            },
        );
        for row in &mut self.rows {
            let row_at = insert_at.min(row.len());
            row.insert(row_at, CellValue::Empty);
        }
    }

    pub fn delete_column(&mut self, index: usize) {
        if index >= self.columns.len() || self.columns.len() <= 1 {
            return;
        }
        self.columns.remove(index);
        for row in &mut self.rows {
            if index < row.len() {
                row.remove(index);
            }
        }
    }
}

pub fn parse_cell_value(input: &str, col_type: &ColumnType) -> CellValue {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return CellValue::Empty;
    }

    // Formula detection: starts with '='
    if trimmed.starts_with('=') {
        return CellValue::Formula {
            expr: trimmed[1..].to_string(),
            cached: None,
        };
    }

    match col_type {
        ColumnType::Text => CellValue::Text(trimmed.to_string()),
        ColumnType::Number => {
            if let Ok(n) = trimmed.parse::<f64>() {
                CellValue::Number(n)
            } else {
                CellValue::Text(trimmed.to_string())
            }
        }
        ColumnType::Currency(_) => {
            // Strip common currency symbols before parsing
            let cleaned: String = trimmed
                .chars()
                .filter(|c| c.is_ascii_digit() || *c == '.' || *c == '-')
                .collect();
            if let Ok(n) = cleaned.parse::<f64>() {
                CellValue::Currency(n)
            } else {
                CellValue::Text(trimmed.to_string())
            }
        }
        ColumnType::Formula => CellValue::Formula {
            expr: trimmed.to_string(),
            cached: None,
        },
    }
}

pub fn col_index_to_letter(index: usize) -> String {
    let mut result = String::new();
    let mut n = index;
    loop {
        result.insert(0, (b'A' + (n % 26) as u8) as char);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    result
}

pub fn letter_to_col_index(letter: &str) -> Option<usize> {
    let mut index = 0usize;
    for (i, c) in letter.chars().rev().enumerate() {
        if !c.is_ascii_uppercase() {
            return None;
        }
        let val = (c as usize) - ('A' as usize);
        if i == 0 {
            index += val;
        } else {
            index += (val + 1) * 26usize.pow(i as u32);
        }
    }
    Some(index)
}
