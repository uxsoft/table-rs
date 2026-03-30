use std::path::{Path, PathBuf};

use super::{CellValue, ColumnDef, ColumnType, Sheet, SheetMeta, parse_cell_value};

fn meta_path(csv_path: &Path) -> PathBuf {
    csv_path.with_extension("json")
}

pub fn load(path: &Path) -> Result<Sheet, Box<dyn std::error::Error>> {
    let mut rdr = csv::ReaderBuilder::new()
        .has_headers(true)
        .flexible(true)
        .from_path(path)?;

    let headers: Vec<String> = rdr
        .headers()?
        .iter()
        .map(|h| h.to_string())
        .collect();

    // Try to load sidecar meta
    let mp = meta_path(path);
    let meta: Option<SheetMeta> = if mp.exists() {
        let content = std::fs::read_to_string(&mp)?;
        serde_json::from_str(&content).ok()
    } else {
        None
    };

    let columns: Vec<ColumnDef> = if let Some(ref m) = meta {
        // Use meta columns, but ensure we match header count
        let mut cols = m.columns.clone();
        // If CSV has more columns than meta, add extra as Text
        while cols.len() < headers.len() {
            cols.push(ColumnDef {
                name: headers[cols.len()].clone(),
                col_type: ColumnType::Text,
                width: 150.0,
            });
        }
        cols.truncate(headers.len());
        cols
    } else {
        headers
            .iter()
            .map(|h| ColumnDef {
                name: h.clone(),
                col_type: ColumnType::Text,
                width: 150.0,
            })
            .collect()
    };

    let mut rows = Vec::new();
    for result in rdr.records() {
        let record = result?;
        let mut row: Vec<CellValue> = Vec::with_capacity(columns.len());
        for (i, col) in columns.iter().enumerate() {
            let raw = record.get(i).unwrap_or("");
            row.push(parse_cell_value(raw, &col.col_type));
        }
        // Pad row if shorter than columns
        while row.len() < columns.len() {
            row.push(CellValue::Empty);
        }
        rows.push(row);
    }

    Ok(Sheet {
        columns,
        rows,
        file_path: Some(path.to_path_buf()),
        sort: meta.as_ref().and_then(|m| m.sort.clone()),
        group_by: meta.as_ref().and_then(|m| m.group_by),
    })
}

pub fn save(sheet: &Sheet, path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let mut wtr = csv::Writer::from_path(path)?;

    // Write headers
    let headers: Vec<&str> = sheet.columns.iter().map(|c| c.name.as_str()).collect();
    wtr.write_record(&headers)?;

    // Write data rows
    for row in &sheet.rows {
        let fields: Vec<String> = row
            .iter()
            .enumerate()
            .map(|(i, cell)| cell_to_csv(cell, sheet.columns.get(i)))
            .collect();
        wtr.write_record(&fields)?;
    }
    wtr.flush()?;

    // Write sidecar meta
    let meta = SheetMeta {
        columns: sheet.columns.clone(),
        sort: sheet.sort.clone(),
        group_by: sheet.group_by,
    };
    let mp = meta_path(path);
    let json = serde_json::to_string_pretty(&meta)?;
    std::fs::write(mp, json)?;

    Ok(())
}

fn cell_to_csv(cell: &CellValue, _col: Option<&ColumnDef>) -> String {
    match cell {
        CellValue::Text(s) => s.clone(),
        CellValue::Number(n) => format!("{}", n),
        CellValue::Currency(n) => format!("{}", n),
        CellValue::Formula { expr, .. } => format!("={}", expr),
        CellValue::Empty => String::new(),
    }
}
