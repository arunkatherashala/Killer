/// Phase 40: Advanced Office Features
/// 
/// Implements advanced Excel/Office capabilities:
/// - Cell formulas (SUM, AVERAGE, IF, COUNT, MIN, MAX, CONCAT)
/// - Charts (Bar, Pie, Line)
/// - Styling (bold, italic, colors, fonts, number formatting, alignment)
/// - Integration with Phase 39 office format support
///
/// Author: Killer Language Dev Team
/// Version: 1.0.0
/// Date: 2026-03-19

use std::collections::HashMap;
use std::error::Error;
use std::fmt;

#[derive(Debug, Clone)]
pub enum CellFormula {
    Sum(Vec<String>),           // SUM(A1:A10)
    Average(Vec<String>),       // AVERAGE(B1:B5)
    If(String, String, String), // IF(C1>10, "Yes", "No")
    Count(Vec<String>),         // COUNT(D1:D20)
    CountIf(Vec<String>, String), // COUNTIF(E1:E10, ">5")
    Min(Vec<String>),           // MIN(F1:F15)
    Max(Vec<String>),           // MAX(G1:G20)
    Concat(Vec<String>),        // CONCAT(H1, " ", H2)
    AverageIf(Vec<String>, String, Vec<String>), // AVERAGEIF(I1:I10, ">100", J1:J10)
    Raw(String),                // Raw formula string
}

#[derive(Debug, Clone, PartialEq)]
pub enum CellStyle {
    Normal,
    Bold,
    Italic,
    BoldItalic,
}

#[derive(Debug, Clone)]
pub struct CellFormatting {
    pub style: CellStyle,
    pub font_color: Option<String>,     // "FF0000" for red
    pub background_color: Option<String>, // "FFFF00" for yellow
    pub number_format: Option<String>,   // "0.00" for decimals
    pub alignment: Option<String>,       // "center", "left", "right"
    pub font_size: Option<usize>,        // Default 12
    pub font_name: Option<String>,       // "Arial", "Calibri", etc.
}

impl Default for CellFormatting {
    fn default() -> Self {
        CellFormatting {
            style: CellStyle::Normal,
            font_color: None,
            background_color: None,
            number_format: None,
            alignment: None,
            font_size: Some(12),
            font_name: Some("Calibri".to_string()),
        }
    }
}

#[derive(Debug, Clone)]
pub enum ChartType {
    Bar,
    Pie,
    Line,
    Column,
    Scatter,
}

#[derive(Debug, Clone)]
pub struct ChartData {
    pub chart_type: ChartType,
    pub title: String,
    pub x_axis_label: String,
    pub y_axis_label: String,
    pub data_range: String,      // "A1:B10"
    pub series_range: String,     // "B1:B10"
    pub categories_range: String, // "A1:A10"
}

/// Formula Builder: Constructs and validates cell formulas
#[derive(Debug)]
pub struct FormulaBuilder {
    formulas: HashMap<String, CellFormula>,
}

impl FormulaBuilder {
    pub fn new() -> Self {
        FormulaBuilder {
            formulas: HashMap::new(),
        }
    }

    pub fn add_sum(&mut self, cell: &str, range: Vec<String>) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("Sum range cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Sum(range));
        Ok(())
    }

    pub fn add_average(&mut self, cell: &str, range: Vec<String>) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("Average range cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Average(range));
        Ok(())
    }

    pub fn add_if(
        &mut self,
        cell: &str,
        condition: String,
        true_val: String,
        false_val: String,
    ) -> Result<(), Box<dyn Error>> {
        if condition.is_empty() {
            return Err("If condition cannot be empty".into());
        }
        self.formulas.insert(
            cell.to_string(),
            CellFormula::If(condition, true_val, false_val),
        );
        Ok(())
    }

    pub fn add_count(&mut self, cell: &str, range: Vec<String>) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("Count range cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Count(range));
        Ok(())
    }

    pub fn add_countif(
        &mut self,
        cell: &str,
        range: Vec<String>,
        criteria: String,
    ) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("CountIf range cannot be empty".into());
        }
        if criteria.is_empty() {
            return Err("CountIf criteria cannot be empty".into());
        }
        self.formulas.insert(
            cell.to_string(),
            CellFormula::CountIf(range, criteria),
        );
        Ok(())
    }

    pub fn add_min(&mut self, cell: &str, range: Vec<String>) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("Min range cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Min(range));
        Ok(())
    }

    pub fn add_max(&mut self, cell: &str, range: Vec<String>) -> Result<(), Box<dyn Error>> {
        if range.is_empty() {
            return Err("Max range cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Max(range));
        Ok(())
    }

    pub fn add_concat(
        &mut self,
        cell: &str,
        values: Vec<String>,
    ) -> Result<(), Box<dyn Error>> {
        if values.is_empty() {
            return Err("Concat values cannot be empty".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Concat(values));
        Ok(())
    }

    pub fn add_raw(&mut self, cell: &str, formula: String) -> Result<(), Box<dyn Error>> {
        if formula.is_empty() {
            return Err("Formula string cannot be empty".into());
        }
        if !formula.starts_with('=') {
            return Err("Formula must start with '='".into());
        }
        self.formulas.insert(cell.to_string(), CellFormula::Raw(formula));
        Ok(())
    }

    pub fn get_formula(&self, cell: &str) -> Option<&CellFormula> {
        self.formulas.get(cell)
    }

    pub fn render_formula_string(&self, cell: &str) -> Result<String, Box<dyn Error>> {
        match self.get_formula(cell) {
            Some(CellFormula::Sum(range)) => Ok(format!("=SUM({})", range.join(","))),
            Some(CellFormula::Average(range)) => Ok(format!("=AVERAGE({})", range.join(","))),
            Some(CellFormula::If(cond, true_val, false_val)) => {
                Ok(format!("=IF({};\"{}\";\"{}\")", cond, true_val, false_val))
            }
            Some(CellFormula::Count(range)) => Ok(format!("=COUNT({})", range.join(","))),
            Some(CellFormula::CountIf(range, criteria)) => {
                Ok(format!("=COUNTIF({};\"{}\")", range.join(","), criteria))
            }
            Some(CellFormula::Min(range)) => Ok(format!("=MIN({})", range.join(","))),
            Some(CellFormula::Max(range)) => Ok(format!("=MAX({})", range.join(","))),
            Some(CellFormula::Concat(values)) => Ok(format!("=CONCAT({})", values.join(","))),
            Some(CellFormula::AverageIf(range, criteria, avg_range)) => {
                let mut combined = range.clone();
                combined.extend(avg_range.clone());
                Ok(format!(
                    "=AVERAGEIF({};\"{}\")",
                    combined.join(","),
                    criteria
                ))
            }
            Some(CellFormula::Raw(formula)) => Ok(formula.clone()),
            None => Err(format!("No formula found for cell {}", cell).into()),
        }
    }

    pub fn count_formulas(&self) -> usize {
        self.formulas.len()
    }

    pub fn clear(&mut self) {
        self.formulas.clear();
    }
}

/// Style Applier: Applies formatting to cells
#[derive(Debug)]
pub struct StyleApplier {
    styles: HashMap<String, CellFormatting>,
}

impl StyleApplier {
    pub fn new() -> Self {
        StyleApplier {
            styles: HashMap::new(),
        }
    }

    pub fn apply_bold(&mut self, cell: &str) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.style = CellStyle::Bold;
    }

    pub fn apply_italic(&mut self, cell: &str) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.style = CellStyle::Italic;
    }

    pub fn apply_bold_italic(&mut self, cell: &str) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.style = CellStyle::BoldItalic;
    }

    pub fn apply_font_color(&mut self, cell: &str, color: String) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.font_color = Some(color);
    }

    pub fn apply_background_color(&mut self, cell: &str, color: String) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.background_color = Some(color);
    }

    pub fn apply_number_format(&mut self, cell: &str, format: String) {
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.number_format = Some(format);
    }

    pub fn apply_alignment(&mut self, cell: &str, alignment: String) {
        let valid_alignments = vec!["left", "center", "right"];
        if !valid_alignments.contains(&alignment.as_str()) {
            return;
        }
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.alignment = Some(alignment);
    }

    pub fn apply_font_size(&mut self, cell: &str, size: usize) {
        if size < 8 || size > 72 {
            return;
        }
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.font_size = Some(size);
    }

    pub fn apply_font_name(&mut self, cell: &str, name: String) {
        let valid_fonts = vec!["Arial", "Calibri", "Times New Roman", "Courier New"];
        if !valid_fonts.contains(&name.as_str()) {
            return;
        }
        let mut fmt = self.styles.entry(cell.to_string()).or_insert_with(CellFormatting::default);
        fmt.font_name = Some(name);
    }

    pub fn get_style(&self, cell: &str) -> Option<&CellFormatting> {
        self.styles.get(cell)
    }

    pub fn render_style_css(&self, cell: &str) -> Result<String, Box<dyn Error>> {
        match self.get_style(cell) {
            Some(fmt) => {
                let mut css = String::new();
                match fmt.style {
                    CellStyle::Bold => css.push_str("font-weight:bold;"),
                    CellStyle::Italic => css.push_str("font-style:italic;"),
                    CellStyle::BoldItalic => css.push_str("font-weight:bold;font-style:italic;"),
                    _ => {}
                }
                if let Some(color) = &fmt.font_color {
                    css.push_str(&format!("color:#{};", color));
                }
                if let Some(bg) = &fmt.background_color {
                    css.push_str(&format!("background-color:#{};", bg));
                }
                if let Some(align) = &fmt.alignment {
                    css.push_str(&format!("text-align:{};", align));
                }
                if let Some(size) = fmt.font_size {
                    css.push_str(&format!("font-size:{}pt;", size));
                }
                if let Some(name) = &fmt.font_name {
                    css.push_str(&format!("font-family:{};", name));
                }
                Ok(css)
            }
            None => Err(format!("No style found for cell {}", cell).into()),
        }
    }

    pub fn count_styles(&self) -> usize {
        self.styles.len()
    }

    pub fn clear(&mut self) {
        self.styles.clear();
    }
}

/// Chart Builder: Creates charts for office documents
#[derive(Debug)]
pub struct ChartBuilder {
    charts: HashMap<String, ChartData>,
    chart_counter: usize,
}

impl ChartBuilder {
    pub fn new() -> Self {
        ChartBuilder {
            charts: HashMap::new(),
            chart_counter: 0,
        }
    }

    pub fn create_bar_chart(
        &mut self,
        title: &str,
        x_label: &str,
        y_label: &str,
        data_range: &str,
    ) -> Result<String, Box<dyn Error>> {
        if title.is_empty() || data_range.is_empty() {
            return Err("Chart title and data_range cannot be empty".into());
        }
        let chart_id = format!("chart_{}", self.chart_counter);
        self.chart_counter += 1;

        let chart = ChartData {
            chart_type: ChartType::Bar,
            title: title.to_string(),
            x_axis_label: x_label.to_string(),
            y_axis_label: y_label.to_string(),
            data_range: data_range.to_string(),
            series_range: "".to_string(),
            categories_range: "".to_string(),
        };
        self.charts.insert(chart_id.clone(), chart);
        Ok(chart_id)
    }

    pub fn create_pie_chart(
        &mut self,
        title: &str,
        data_range: &str,
    ) -> Result<String, Box<dyn Error>> {
        if title.is_empty() || data_range.is_empty() {
            return Err("Chart title and data_range cannot be empty".into());
        }
        let chart_id = format!("chart_{}", self.chart_counter);
        self.chart_counter += 1;

        let chart = ChartData {
            chart_type: ChartType::Pie,
            title: title.to_string(),
            x_axis_label: "".to_string(),
            y_axis_label: "".to_string(),
            data_range: data_range.to_string(),
            series_range: "".to_string(),
            categories_range: "".to_string(),
        };
        self.charts.insert(chart_id.clone(), chart);
        Ok(chart_id)
    }

    pub fn create_line_chart(
        &mut self,
        title: &str,
        x_label: &str,
        y_label: &str,
        data_range: &str,
    ) -> Result<String, Box<dyn Error>> {
        if title.is_empty() || data_range.is_empty() {
            return Err("Chart title and data_range cannot be empty".into());
        }
        let chart_id = format!("chart_{}", self.chart_counter);
        self.chart_counter += 1;

        let chart = ChartData {
            chart_type: ChartType::Line,
            title: title.to_string(),
            x_axis_label: x_label.to_string(),
            y_axis_label: y_label.to_string(),
            data_range: data_range.to_string(),
            series_range: "".to_string(),
            categories_range: "".to_string(),
        };
        self.charts.insert(chart_id.clone(), chart);
        Ok(chart_id)
    }

    pub fn get_chart(&self, chart_id: &str) -> Option<&ChartData> {
        self.charts.get(chart_id)
    }

    pub fn render_chart_xml(&self, chart_id: &str) -> Result<String, Box<dyn Error>> {
        match self.get_chart(chart_id) {
            Some(chart) => {
                let chart_type = match chart.chart_type {
                    ChartType::Bar => "barChart",
                    ChartType::Pie => "pieChart",
                    ChartType::Line => "lineChart",
                    ChartType::Column => "colChart",
                    ChartType::Scatter => "scatterChart",
                };
                Ok(format!(
                    r#"<c:{}: title="{}" data="{}"/>"#,
                    chart_type, chart.title, chart.data_range
                ))
            }
            None => Err(format!("Chart {} not found", chart_id).into()),
        }
    }

    pub fn count_charts(&self) -> usize {
        self.charts.len()
    }

    pub fn clear(&mut self) {
        self.charts.clear();
        self.chart_counter = 0;
    }
}

/// Advanced Office Features Coordinator
#[derive(Debug)]
pub struct AdvancedOfficeFeatures {
    formula_builder: FormulaBuilder,
    style_applier: StyleApplier,
    chart_builder: ChartBuilder,
}

impl AdvancedOfficeFeatures {
    pub fn new() -> Self {
        AdvancedOfficeFeatures {
            formula_builder: FormulaBuilder::new(),
            style_applier: StyleApplier::new(),
            chart_builder: ChartBuilder::new(),
        }
    }

    pub fn formulas(&mut self) -> &mut FormulaBuilder {
        &mut self.formula_builder
    }

    pub fn styles(&mut self) -> &mut StyleApplier {
        &mut self.style_applier
    }

    pub fn charts(&mut self) -> &mut ChartBuilder {
        &mut self.chart_builder
    }

    pub fn generate_excel_with_formulas(
        &self,
        base_content: &str,
        formula_cells: Vec<(String, String)>,
    ) -> Result<String, Box<dyn Error>> {
        let mut output = base_content.to_string();
        for (cell, formula_str) in formula_cells {
            output.push_str(&format!("\n[{}] {}", cell, formula_str));
        }
        Ok(output)
    }

    pub fn apply_formatting_batch(
        &mut self,
        cells: Vec<&str>,
        formatting: CellFormatting,
    ) -> Result<(), Box<dyn Error>> {
        for cell in cells {
            self.style_applier.styles.insert(cell.to_string(), formatting.clone());
        }
        Ok(())
    }

    pub fn summary(&self) -> String {
        format!(
            "Advanced Office Features:\n- Formulas: {}\n- Styles: {}\n- Charts: {}",
            self.formula_builder.count_formulas(),
            self.style_applier.count_styles(),
            self.chart_builder.count_charts()
        )
    }
}

// Tests
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_formula_builder_sum() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_sum("A1", vec!["A2".to_string(), "A3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_sum_empty_range() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_sum("A1", vec![]);
        assert!(result.is_err());
    }

    #[test]
    fn test_formula_builder_average() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_average("B1", vec!["B2".to_string(), "B3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_if() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_if("C1", "C2>10".to_string(), "Yes".to_string(), "No".to_string());
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_if_empty_condition() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_if("C1", "".to_string(), "Yes".to_string(), "No".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_formula_builder_count() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_count("D1", vec!["D2".to_string(), "D3".to_string(), "D4".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_countif() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_countif("E1", vec!["E2".to_string(), "E3".to_string()], ">5".to_string());
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_countif_empty_criteria() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_countif("E1", vec!["E2".to_string()], "".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_formula_builder_min() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_min("F1", vec!["F2".to_string(), "F3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_max() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_max("G1", vec!["G2".to_string(), "G3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_concat() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_concat("H1", vec!["H2".to_string(), "H3".to_string()]);
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_raw() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_raw("I1", "=I2+I3*2".to_string());
        assert!(result.is_ok());
        assert_eq!(builder.count_formulas(), 1);
    }

    #[test]
    fn test_formula_builder_raw_missing_equals() {
        let mut builder = FormulaBuilder::new();
        let result = builder.add_raw("I1", "I2+I3".to_string());
        assert!(result.is_err());
    }

    #[test]
    fn test_formula_render_sum() {
        let mut builder = FormulaBuilder::new();
        builder.add_sum("A1", vec!["A2".to_string(), "A3".to_string()]).unwrap();
        let formula = builder.render_formula_string("A1").unwrap();
        assert!(formula.contains("SUM"));
    }

    #[test]
    fn test_formula_builder_clear() {
        let mut builder = FormulaBuilder::new();
        builder.add_sum("A1", vec!["A2".to_string()]).unwrap();
        assert_eq!(builder.count_formulas(), 1);
        builder.clear();
        assert_eq!(builder.count_formulas(), 0);
    }

    #[test]
    fn test_style_applier_bold() {
        let mut applier = StyleApplier::new();
        applier.apply_bold("A1");
        let style = applier.get_style("A1").unwrap();
        assert_eq!(style.style, CellStyle::Bold);
    }

    #[test]
    fn test_style_applier_italic() {
        let mut applier = StyleApplier::new();
        applier.apply_italic("B1");
        let style = applier.get_style("B1").unwrap();
        assert_eq!(style.style, CellStyle::Italic);
    }

    #[test]
    fn test_style_applier_bold_italic() {
        let mut applier = StyleApplier::new();
        applier.apply_bold_italic("C1");
        let style = applier.get_style("C1").unwrap();
        assert_eq!(style.style, CellStyle::BoldItalic);
    }

    #[test]
    fn test_style_applier_font_color() {
        let mut applier = StyleApplier::new();
        applier.apply_font_color("D1", "FF0000".to_string());
        let style = applier.get_style("D1").unwrap();
        assert_eq!(style.font_color, Some("FF0000".to_string()));
    }

    #[test]
    fn test_style_applier_background_color() {
        let mut applier = StyleApplier::new();
        applier.apply_background_color("E1", "FFFF00".to_string());
        let style = applier.get_style("E1").unwrap();
        assert_eq!(style.background_color, Some("FFFF00".to_string()));
    }

    #[test]
    fn test_style_applier_number_format() {
        let mut applier = StyleApplier::new();
        applier.apply_number_format("F1", "0.00".to_string());
        let style = applier.get_style("F1").unwrap();
        assert_eq!(style.number_format, Some("0.00".to_string()));
    }

    #[test]
    fn test_style_applier_alignment_center() {
        let mut applier = StyleApplier::new();
        applier.apply_alignment("G1", "center".to_string());
        let style = applier.get_style("G1").unwrap();
        assert_eq!(style.alignment, Some("center".to_string()));
    }

    #[test]
    fn test_style_applier_alignment_invalid() {
        let mut applier = StyleApplier::new();
        applier.apply_alignment("G1", "invalid".to_string());
        assert!(applier.get_style("G1").is_none());
    }

    #[test]
    fn test_style_applier_font_size() {
        let mut applier = StyleApplier::new();
        applier.apply_font_size("H1", 16);
        let style = applier.get_style("H1").unwrap();
        assert_eq!(style.font_size, Some(16));
    }

    #[test]
    fn test_style_applier_font_size_invalid() {
        let mut applier = StyleApplier::new();
        applier.apply_font_size("H1", 100);
        assert!(applier.get_style("H1").is_none());
    }

    #[test]
    fn test_style_applier_font_name() {
        let mut applier = StyleApplier::new();
        applier.apply_font_name("I1", "Arial".to_string());
        let style = applier.get_style("I1").unwrap();
        assert_eq!(style.font_name, Some("Arial".to_string()));
    }

    #[test]
    fn test_style_applier_font_name_invalid() {
        let mut applier = StyleApplier::new();
        applier.apply_font_name("I1", "InvalidFont".to_string());
        assert!(applier.get_style("I1").is_none());
    }

    #[test]
    fn test_style_applier_css_rendering() {
        let mut applier = StyleApplier::new();
        applier.apply_bold("A1");
        applier.apply_font_color("A1", "FF0000".to_string());
        let css = applier.render_style_css("A1").unwrap();
        assert!(css.contains("font-weight:bold"));
        assert!(css.contains("color:#FF0000"));
    }

    #[test]
    fn test_style_applier_clear() {
        let mut applier = StyleApplier::new();
        applier.apply_bold("A1");
        assert_eq!(applier.count_styles(), 1);
        applier.clear();
        assert_eq!(applier.count_styles(), 0);
    }

    #[test]
    fn test_chart_builder_bar_chart() {
        let mut builder = ChartBuilder::new();
        let result = builder.create_bar_chart("Sales", "Month", "Revenue", "A1:B12");
        assert!(result.is_ok());
        let chart_id = result.unwrap();
        assert!(builder.get_chart(&chart_id).is_some());
    }

    #[test]
    fn test_chart_builder_pie_chart() {
        let mut builder = ChartBuilder::new();
        let result = builder.create_pie_chart("Market Share", "A1:B5");
        assert!(result.is_ok());
        let chart_id = result.unwrap();
        assert!(builder.get_chart(&chart_id).is_some());
    }

    #[test]
    fn test_chart_builder_line_chart() {
        let mut builder = ChartBuilder::new();
        let result = builder.create_line_chart("Trend", "Date", "Value", "A1:B30");
        assert!(result.is_ok());
        let chart_id = result.unwrap();
        assert!(builder.get_chart(&chart_id).is_some());
    }

    #[test]
    fn test_chart_builder_empty_title() {
        let mut builder = ChartBuilder::new();
        let result = builder.create_bar_chart("", "Month", "Revenue", "A1:B12");
        assert!(result.is_err());
    }

    #[test]
    fn test_chart_builder_empty_data_range() {
        let mut builder = ChartBuilder::new();
        let result = builder.create_bar_chart("Sales", "Month", "Revenue", "");
        assert!(result.is_err());
    }

    #[test]
    fn test_chart_builder_xml_rendering() {
        let mut builder = ChartBuilder::new();
        let chart_id = builder.create_bar_chart("Sales", "Month", "Revenue", "A1:B12").unwrap();
        let xml = builder.render_chart_xml(&chart_id).unwrap();
        assert!(xml.contains("barChart"));
        assert!(xml.contains("Sales"));
    }

    #[test]
    fn test_chart_builder_multiple_charts() {
        let mut builder = ChartBuilder::new();
        let id1 = builder.create_bar_chart("Chart1", "X", "Y", "A1:B10").unwrap();
        let id2 = builder.create_pie_chart("Chart2", "C1:D10").unwrap();
        assert_eq!(builder.count_charts(), 2);
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_chart_builder_clear() {
        let mut builder = ChartBuilder::new();
        builder.create_bar_chart("Chart", "X", "Y", "A1:B10").unwrap();
        assert_eq!(builder.count_charts(), 1);
        builder.clear();
        assert_eq!(builder.count_charts(), 0);
    }

    #[test]
    fn test_advanced_office_features_integration() {
        let mut features = AdvancedOfficeFeatures::new();
        
        // Add formula
        features.formulas().add_sum("A1", vec!["A2".to_string(), "A3".to_string()]).unwrap();
        
        // Add style
        features.styles().apply_bold("A1");
        
        // Add chart
        features.charts().create_bar_chart("Sales", "Month", "Revenue", "A1:B12").unwrap();
        
        let summary = features.summary();
        assert!(summary.contains("Formulas: 1"));
        assert!(summary.contains("Styles: 1"));
        assert!(summary.contains("Charts: 1"));
    }

    #[test]
    fn test_advanced_office_features_batch_formatting() {
        let mut features = AdvancedOfficeFeatures::new();
        let fmt = CellFormatting {
            style: CellStyle::Bold,
            font_color: Some("FF0000".to_string()),
            ..Default::default()
        };
        let cells = vec!["A1", "A2", "A3"];
        let result = features.apply_formatting_batch(cells, fmt);
        assert!(result.is_ok());
    }

    #[test]
    fn test_formula_builder_complex_workflow() {
        let mut builder = FormulaBuilder::new();
        builder.add_sum("A1", vec!["A2".to_string(), "A3".to_string()]).unwrap();
        builder.add_average("B1", vec!["B2".to_string(), "B3".to_string()]).unwrap();
        builder.add_if("C1", "A1>100".to_string(), "High".to_string(), "Low".to_string()).unwrap();
        
        assert_eq!(builder.count_formulas(), 3);
        assert!(builder.get_formula("A1").is_some());
        assert!(builder.get_formula("B1").is_some());
        assert!(builder.get_formula("C1").is_some());
    }

    #[test]
    fn test_style_applier_complex_formatting() {
        let mut applier = StyleApplier::new();
        applier.apply_bold("A1");
        applier.apply_font_color("A1", "FF0000".to_string());
        applier.apply_background_color("A1", "FFFF00".to_string());
        applier.apply_font_size("A1", 14);
        applier.apply_alignment("A1", "center".to_string());
        
        let style = applier.get_style("A1").unwrap();
        assert_eq!(style.style, CellStyle::Bold);
        assert_eq!(style.font_color, Some("FF0000".to_string()));
        assert_eq!(style.background_color, Some("FFFF00".to_string()));
        assert_eq!(style.font_size, Some(14));
        assert_eq!(style.alignment, Some("center".to_string()));
    }
}
