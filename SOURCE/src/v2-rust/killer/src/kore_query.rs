// ============================================================================
// KORE Query Engine (Gap #4) — SQL-like queries on KORE files
// ============================================================================
//
// Supports: SELECT col1,col2 FROM file WHERE col3 > 100 GROUP BY col1
// Leverages KORE's column pruning + predicate pushdown for maximum speed.
//
// Usage:
//   let result = kore_query("SELECT category, SUM(total) FROM data.kore WHERE quantity > 10 GROUP BY category");
//
use crate::kore_v2::{KoreReader, KVal};
use std::collections::HashMap;

// ── Query AST ────────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub enum AggFunc { Count, Sum, Avg, Min, Max, None }

#[derive(Debug, Clone)]
pub struct SelectCol {
    pub name: String,
    pub agg: AggFunc,
    pub alias: Option<String>,
}

#[derive(Debug, Clone)]
pub enum FilterOp { Eq, Neq, Gt, Lt, Gte, Lte, Contains }

#[derive(Debug, Clone)]
pub struct WhereClause {
    pub col: String,
    pub op: FilterOp,
    pub val: String,
}

#[derive(Debug, Clone)]
pub struct KoreQuery {
    pub select: Vec<SelectCol>,
    pub from: String,
    pub where_clauses: Vec<WhereClause>,
    pub group_by: Vec<String>,
    pub order_by: Option<(String, bool)>,  // (col, ascending)
    pub limit: Option<usize>,
}

// ── Query Result ─────────────────────────────────────────────────────────────
#[derive(Debug, Clone)]
pub struct QueryResult {
    pub columns: Vec<String>,
    pub rows: Vec<Vec<KVal>>,
    pub elapsed_ms: f64,
}

impl QueryResult {
    pub fn to_csv(&self) -> String {
        let mut out = self.columns.join(",");
        out.push('\n');
        for row in &self.rows {
            let vals: Vec<String> = row.iter().map(|v| v.display()).collect();
            out.push_str(&vals.join(","));
            out.push('\n');
        }
        out
    }

    pub fn display_table(&self, max_rows: usize) -> String {
        let mut out = String::new();
        // Header
        let widths: Vec<usize> = self.columns.iter().enumerate().map(|(ci, name)| {
            let mut w = name.len();
            for row in self.rows.iter().take(max_rows) {
                if ci < row.len() { w = w.max(row[ci].display().len()); }
            }
            w.min(40)
        }).collect();

        for (ci, name) in self.columns.iter().enumerate() {
            out.push_str(&format!("{:width$}", name, width = widths[ci] + 2));
        }
        out.push('\n');
        for w in &widths { out.push_str(&"-".repeat(*w + 2)); }
        out.push('\n');

        for row in self.rows.iter().take(max_rows) {
            for (ci, val) in row.iter().enumerate() {
                let s = val.display();
                let w = widths.get(ci).copied().unwrap_or(10);
                out.push_str(&format!("{:width$}", s, width = w + 2));
            }
            out.push('\n');
        }
        if self.rows.len() > max_rows {
            out.push_str(&format!("... {} more rows\n", self.rows.len() - max_rows));
        }
        out.push_str(&format!("\n({} rows, {:.1}ms)\n", self.rows.len(), self.elapsed_ms));
        out
    }
}

// ── SQL Parser ───────────────────────────────────────────────────────────────
pub fn parse_query(sql: &str) -> Result<KoreQuery, String> {
    let sql = sql.trim().trim_end_matches(';');
    let upper = sql.to_uppercase();

    // Extract SELECT columns
    let select_start = if upper.starts_with("SELECT ") { 7 } else {
        return Err("Query must start with SELECT".to_string());
    };
    let from_pos = upper.find(" FROM ")
        .ok_or("Missing FROM clause")?;
    let select_str = &sql[select_start..from_pos];

    let select: Vec<SelectCol> = select_str.split(',').map(|s| {
        let s = s.trim();
        let upper_s = s.to_uppercase();
        if upper_s.starts_with("COUNT(") {
            let inner = s[6..].trim_end_matches(')').trim();
            SelectCol { name: inner.to_string(), agg: AggFunc::Count, alias: Some(format!("COUNT({})", inner)) }
        } else if upper_s.starts_with("SUM(") {
            let inner = s[4..].trim_end_matches(')').trim();
            SelectCol { name: inner.to_string(), agg: AggFunc::Sum, alias: Some(format!("SUM({})", inner)) }
        } else if upper_s.starts_with("AVG(") {
            let inner = s[4..].trim_end_matches(')').trim();
            SelectCol { name: inner.to_string(), agg: AggFunc::Avg, alias: Some(format!("AVG({})", inner)) }
        } else if upper_s.starts_with("MIN(") {
            let inner = s[4..].trim_end_matches(')').trim();
            SelectCol { name: inner.to_string(), agg: AggFunc::Min, alias: Some(format!("MIN({})", inner)) }
        } else if upper_s.starts_with("MAX(") {
            let inner = s[4..].trim_end_matches(')').trim();
            SelectCol { name: inner.to_string(), agg: AggFunc::Max, alias: Some(format!("MAX({})", inner)) }
        } else {
            SelectCol { name: s.to_string(), agg: AggFunc::None, alias: None }
        }
    }).collect();

    // Extract FROM
    let from_end = upper[from_pos + 6..].find(" WHERE ")
        .or_else(|| upper[from_pos + 6..].find(" GROUP "))
        .or_else(|| upper[from_pos + 6..].find(" ORDER "))
        .or_else(|| upper[from_pos + 6..].find(" LIMIT "))
        .map(|p| from_pos + 6 + p)
        .unwrap_or(sql.len());
    let from = sql[from_pos + 6..from_end].trim().to_string();

    // Extract WHERE clauses
    let mut where_clauses = Vec::new();
    if let Some(where_pos) = upper.find(" WHERE ") {
        let where_end = upper[where_pos + 7..].find(" GROUP ")
            .or_else(|| upper[where_pos + 7..].find(" ORDER "))
            .or_else(|| upper[where_pos + 7..].find(" LIMIT "))
            .map(|p| where_pos + 7 + p)
            .unwrap_or(sql.len());
        let where_str = &sql[where_pos + 7..where_end];
        // Split by AND
        for clause in where_str.split(" AND ") {
            let clause = clause.trim();
            let (col, op, val) = parse_where_clause(clause)?;
            where_clauses.push(WhereClause { col, op, val });
        }
    }

    // Extract GROUP BY
    let group_by = if let Some(gb_pos) = upper.find(" GROUP BY ") {
        let gb_end = upper[gb_pos + 10..].find(" ORDER ")
            .or_else(|| upper[gb_pos + 10..].find(" LIMIT "))
            .map(|p| gb_pos + 10 + p)
            .unwrap_or(sql.len());
        sql[gb_pos + 10..gb_end].split(',').map(|s| s.trim().to_string()).collect()
    } else {
        Vec::new()
    };

    // Extract ORDER BY
    let order_by = if let Some(ob_pos) = upper.find(" ORDER BY ") {
        let ob_end = upper[ob_pos + 10..].find(" LIMIT ")
            .map(|p| ob_pos + 10 + p)
            .unwrap_or(sql.len());
        let ob_str = sql[ob_pos + 10..ob_end].trim();
        let asc = !ob_str.to_uppercase().ends_with(" DESC");
        let col = ob_str.split_whitespace().next().unwrap_or("").to_string();
        Some((col, asc))
    } else {
        None
    };

    // Extract LIMIT
    let limit = if let Some(l_pos) = upper.find(" LIMIT ") {
        sql[l_pos + 7..].trim().parse::<usize>().ok()
    } else {
        None
    };

    Ok(KoreQuery { select, from, where_clauses, group_by, order_by, limit })
}

fn parse_where_clause(clause: &str) -> Result<(String, FilterOp, String), String> {
    let ops = [(">=", FilterOp::Gte), ("<=", FilterOp::Lte), ("!=", FilterOp::Neq),
               (">", FilterOp::Gt), ("<", FilterOp::Lt), ("=", FilterOp::Eq),
               ("CONTAINS", FilterOp::Contains)];
    for (sym, op) in &ops {
        if let Some(pos) = clause.to_uppercase().find(sym) {
            let col = clause[..pos].trim().to_string();
            let val = clause[pos + sym.len()..].trim().trim_matches('\'').trim_matches('"').to_string();
            return Ok((col, op.clone(), val));
        }
    }
    Err(format!("Cannot parse WHERE clause: {}", clause))
}

// ── Query Executor ───────────────────────────────────────────────────────────
pub fn kore_query(sql: &str) -> Result<QueryResult, String> {
    let t0 = std::time::Instant::now();
    let query = parse_query(sql)?;

    let reader = KoreReader::open(&query.from)?;

    // Determine which columns we actually need (pruning!)
    let mut needed_cols: Vec<String> = Vec::new();
    for sc in &query.select {
        if sc.name != "*" && !needed_cols.contains(&sc.name) { needed_cols.push(sc.name.clone()); }
    }
    for wc in &query.where_clauses {
        if !needed_cols.contains(&wc.col) { needed_cols.push(wc.col.clone()); }
    }
    for gb in &query.group_by {
        if !needed_cols.contains(gb) { needed_cols.push(gb.clone()); }
    }

    // Handle SELECT *
    let select_all = query.select.iter().any(|s| s.name == "*");
    if select_all {
        needed_cols = reader.columns.iter().map(|c| c.name.clone()).collect();
    }

    // Column pruning: only read what we need
    let col_refs: Vec<&str> = needed_cols.iter().map(|s| s.as_str()).collect();
    let col_data = reader.read_columns(&col_refs);

    let nrows = col_data.values().next().map(|v| v.len()).unwrap_or(0);

    // Apply WHERE filters
    let mut mask = vec![true; nrows];
    for wc in &query.where_clauses {
        if let Some(col_vals) = col_data.get(&wc.col) {
            for i in 0..nrows {
                if !mask[i] { continue; }
                mask[i] = eval_filter(&col_vals[i], &wc.op, &wc.val);
            }
        }
    }

    // GROUP BY execution
    if !query.group_by.is_empty() {
        return execute_group_by(&query, &col_data, &mask, &reader, t0);
    }

    // Simple SELECT (no GROUP BY)
    let out_cols: Vec<SelectCol> = if select_all {
        reader.columns.iter().map(|c| SelectCol { name: c.name.clone(), agg: AggFunc::None, alias: None }).collect()
    } else {
        query.select.clone()
    };

    let col_names: Vec<String> = out_cols.iter()
        .map(|s| s.alias.clone().unwrap_or_else(|| s.name.clone()))
        .collect();

    let mut rows: Vec<Vec<KVal>> = Vec::new();
    for i in 0..nrows {
        if !mask[i] { continue; }
        let row: Vec<KVal> = out_cols.iter().map(|sc| {
            col_data.get(&sc.name).and_then(|v| v.get(i)).cloned().unwrap_or(KVal::Null)
        }).collect();
        rows.push(row);
    }

    // ORDER BY
    if let Some((ref col, asc)) = query.order_by {
        let ci = col_names.iter().position(|c| c == col).unwrap_or(0);
        rows.sort_by(|a, b| {
            let cmp = compare_kval(a.get(ci).unwrap_or(&KVal::Null), b.get(ci).unwrap_or(&KVal::Null));
            if asc { cmp } else { cmp.reverse() }
        });
    }

    // LIMIT
    if let Some(limit) = query.limit {
        rows.truncate(limit);
    }

    Ok(QueryResult {
        columns: col_names,
        rows,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
    })
}

fn eval_filter(val: &KVal, op: &FilterOp, target: &str) -> bool {
    match op {
        FilterOp::Eq => val.display() == target,
        FilterOp::Neq => val.display() != target,
        FilterOp::Gt => {
            if let Ok(t) = target.parse::<f64>() { val.as_f64() > t }
            else { val.as_str() > target }
        }
        FilterOp::Lt => {
            if let Ok(t) = target.parse::<f64>() { val.as_f64() < t }
            else { val.as_str() < target }
        }
        FilterOp::Gte => {
            if let Ok(t) = target.parse::<f64>() { val.as_f64() >= t }
            else { val.as_str() >= target }
        }
        FilterOp::Lte => {
            if let Ok(t) = target.parse::<f64>() { val.as_f64() <= t }
            else { val.as_str() <= target }
        }
        FilterOp::Contains => val.as_str().contains(target),
    }
}

fn compare_kval(a: &KVal, b: &KVal) -> std::cmp::Ordering {
    match (a, b) {
        (KVal::Int(x), KVal::Int(y)) => x.cmp(y),
        (KVal::Float(x), KVal::Float(y)) => x.partial_cmp(y).unwrap_or(std::cmp::Ordering::Equal),
        (KVal::Str(x), KVal::Str(y)) => x.cmp(y),
        _ => a.display().cmp(&b.display()),
    }
}

fn execute_group_by(
    query: &KoreQuery,
    col_data: &HashMap<String, Vec<KVal>>,
    mask: &[bool],
    _reader: &KoreReader,
    t0: std::time::Instant,
) -> Result<QueryResult, String> {
    let nrows = mask.len();

    // Build group keys
    let mut groups: HashMap<String, Vec<usize>> = HashMap::new();
    for i in 0..nrows {
        if !mask[i] { continue; }
        let key: Vec<String> = query.group_by.iter().map(|gb| {
            col_data.get(gb).and_then(|v| v.get(i)).map(|v| v.display()).unwrap_or_default()
        }).collect();
        let key_str = key.join("|");
        groups.entry(key_str).or_default().push(i);
    }

    // Compute aggregates per group
    let col_names: Vec<String> = query.select.iter()
        .map(|s| s.alias.clone().unwrap_or_else(|| s.name.clone()))
        .collect();

    let mut rows: Vec<Vec<KVal>> = Vec::new();
    for (_key, indices) in &groups {
        let mut row = Vec::new();
        for sc in &query.select {
            match sc.agg {
                AggFunc::None => {
                    let val = col_data.get(&sc.name)
                        .and_then(|v| v.get(indices[0]))
                        .cloned()
                        .unwrap_or(KVal::Null);
                    row.push(val);
                }
                AggFunc::Count => {
                    row.push(KVal::Int(indices.len() as i64));
                }
                AggFunc::Sum => {
                    let sum: f64 = indices.iter().map(|&i| {
                        col_data.get(&sc.name).and_then(|v| v.get(i)).map(|v| v.as_f64()).unwrap_or(0.0)
                    }).sum();
                    row.push(KVal::Float(sum));
                }
                AggFunc::Avg => {
                    let sum: f64 = indices.iter().map(|&i| {
                        col_data.get(&sc.name).and_then(|v| v.get(i)).map(|v| v.as_f64()).unwrap_or(0.0)
                    }).sum();
                    row.push(KVal::Float(sum / indices.len().max(1) as f64));
                }
                AggFunc::Min => {
                    let min = indices.iter().filter_map(|&i| {
                        col_data.get(&sc.name).and_then(|v| v.get(i))
                    }).min_by(|a, b| compare_kval(a, b)).cloned().unwrap_or(KVal::Null);
                    row.push(min);
                }
                AggFunc::Max => {
                    let max = indices.iter().filter_map(|&i| {
                        col_data.get(&sc.name).and_then(|v| v.get(i))
                    }).max_by(|a, b| compare_kval(a, b)).cloned().unwrap_or(KVal::Null);
                    row.push(max);
                }
            }
        }
        rows.push(row);
    }

    // ORDER BY
    if let Some((ref col, asc)) = query.order_by {
        let ci = col_names.iter().position(|c| c == col).unwrap_or(0);
        rows.sort_by(|a, b| {
            let cmp = compare_kval(a.get(ci).unwrap_or(&KVal::Null), b.get(ci).unwrap_or(&KVal::Null));
            if asc { cmp } else { cmp.reverse() }
        });
    }

    // LIMIT
    if let Some(limit) = query.limit {
        rows.truncate(limit);
    }

    Ok(QueryResult {
        columns: col_names,
        rows,
        elapsed_ms: t0.elapsed().as_secs_f64() * 1000.0,
    })
}

// ============================================================================
//  Tests
// ============================================================================
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_simple_select() {
        let q = parse_query("SELECT name, age FROM people.kore WHERE age > 25 LIMIT 10").unwrap();
        assert_eq!(q.select.len(), 2);
        assert_eq!(q.from, "people.kore");
        assert_eq!(q.where_clauses.len(), 1);
        assert_eq!(q.limit, Some(10));
    }

    #[test]
    fn test_parse_group_by() {
        let q = parse_query("SELECT category, SUM(total) FROM sales.kore GROUP BY category ORDER BY category").unwrap();
        assert_eq!(q.group_by, vec!["category"]);
        assert!(matches!(q.select[1].agg, AggFunc::Sum));
    }

    #[test]
    fn test_parse_agg_functions() {
        let q = parse_query("SELECT COUNT(*), AVG(price), MIN(date), MAX(total) FROM data.kore").unwrap();
        assert_eq!(q.select.len(), 4);
        assert!(matches!(q.select[0].agg, AggFunc::Count));
        assert!(matches!(q.select[1].agg, AggFunc::Avg));
        assert!(matches!(q.select[2].agg, AggFunc::Min));
        assert!(matches!(q.select[3].agg, AggFunc::Max));
    }
}
