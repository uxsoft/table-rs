use super::{Group, Sheet, SortDirection};

/// Sort sheet rows in place based on sort config.
pub fn sort_rows(sheet: &mut Sheet) {
    if let Some(ref sort) = sheet.sort.clone() {
        let col = sort.column;
        let dir = sort.direction;
        sheet.rows.sort_by(|a, b| {
            let ka = a.get(col).map(|c| c.sort_key()).unwrap_or_default();
            let kb = b.get(col).map(|c| c.sort_key()).unwrap_or_default();
            match dir {
                SortDirection::Ascending => ka.cmp(&kb),
                SortDirection::Descending => kb.cmp(&ka),
            }
        });
    }
}

/// Group rows by a column value. Returns groups with row indices into the sheet's row vec.
pub fn group_rows(sheet: &Sheet) -> Option<Vec<Group>> {
    let col = sheet.group_by?;

    let mut groups: Vec<Group> = Vec::new();
    let mut group_map: std::collections::BTreeMap<String, Vec<usize>> =
        std::collections::BTreeMap::new();

    let col_def = sheet.columns.get(col);
    let sym = match col_def.map(|d| &d.col_type) {
        Some(crate::data::ColumnType::Currency(s)) => s.as_str(),
        _ => "",
    };
    let format = col_def
        .map(|d| d.format.clone())
        .unwrap_or_default();
    for (i, row) in sheet.rows.iter().enumerate() {
        let key = row
            .get(col)
            .map(|c| c.display_value(sym, &format))
            .unwrap_or_default();
        let key = if key.is_empty() {
            "(empty)".to_string()
        } else {
            key
        };
        group_map.entry(key).or_default().push(i);
    }

    for (key, row_indices) in group_map {
        groups.push(Group {
            key,
            row_indices,
            collapsed: false,
        });
    }

    Some(groups)
}

/// Compute aggregate summary for a group: sum, count of numeric values in a column.
pub fn group_aggregate(sheet: &Sheet, group: &Group, col: usize) -> Option<(f64, usize)> {
    let mut sum = 0.0;
    let mut count = 0usize;
    for &ri in &group.row_indices {
        if let Some(row) = sheet.rows.get(ri) {
            if let Some(cell) = row.get(col) {
                if let Some(v) = cell.as_f64() {
                    sum += v;
                    count += 1;
                }
            }
        }
    }
    if count > 0 {
        Some((sum, count))
    } else {
        None
    }
}
