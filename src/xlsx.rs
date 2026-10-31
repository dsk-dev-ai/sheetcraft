use std::collections::HashMap;

use calamine::{Data, Range, Reader, open_workbook_auto};
use rust_xlsxwriter::{Color as XlsxColor, Format, Workbook, Worksheet};

use crate::cell::CellRef;
use crate::engine::Sheet;
use crate::format::CellFormat;
use crate::formula;

impl Sheet {
    pub fn from_xlsx(path: &str) -> Result<Self, String> {
        let mut workbook = open_workbook_auto(path).map_err(|error| error.to_string())?;
        let names = workbook.sheet_names().to_owned();
        let name = names
            .first()
            .ok_or_else(|| "workbook has no worksheets".to_string())?;
        let range = workbook
            .worksheet_range(name)
            .map_err(|error| error.to_string())?;
        let formulas = workbook.worksheet_formula(name).unwrap_or_default();
        let (start_row, start_column) = range.start().unwrap_or((0, 0));

        let mut sheet = Self::new();
        for (row_index, row) in range.rows().enumerate() {
            for (column_index, data) in row.iter().enumerate() {
                let row = start_row + row_index as u32;
                let column = start_column + column_index as u32;
                let imported = formula_at(&formulas, row, column)
                    .and_then(importable_formula)
                    .or_else(|| data_to_raw(data));
                let Some(raw) = imported else {
                    continue;
                };
                sheet.set(CellRef::new(row + 1, column + 1), raw);
            }
        }
        Ok(sheet)
    }

    pub fn to_xlsx(&self, path: &str) -> Result<(), String> {
        let mut workbook = Workbook::new();
        let worksheet = workbook.add_worksheet();
        let mut formats: HashMap<CellFormat, Format> = HashMap::new();
        if let Some((rows, columns)) = self.bounds() {
            if rows > 1_048_576 || columns > 16_384 {
                return Err("sheet is larger than the XLSX limit".to_string());
            }
            for row in 1..=rows {
                for column in 1..=columns {
                    let reference = CellRef::new(row, column);
                    let Some(raw) = self.raw(&reference) else {
                        continue;
                    };
                    let cell_format = self.format(&reference);
                    let format = if cell_format.is_default() {
                        None
                    } else {
                        Some(
                            &*formats
                                .entry(cell_format)
                                .or_insert_with(|| to_xlsx_format(cell_format)),
                        )
                    };
                    write_cell(worksheet, row - 1, (column - 1) as u16, raw, format)?;
                }
            }
        }
        workbook.save(path).map_err(|error| error.to_string())
    }
}

fn formula_at(formulas: &Range<String>, row: u32, column: u32) -> Option<&str> {
    let (start_row, start_column) = formulas.start()?;
    if row < start_row || column < start_column {
        return None;
    }
    formulas
        .get(((row - start_row) as usize, (column - start_column) as usize))
        .map(String::as_str)
        .filter(|formula| !formula.is_empty())
}

fn importable_formula(formula: &str) -> Option<String> {
    let formula = formula.strip_prefix('=').unwrap_or(formula);
    let expr = formula::parse(formula).ok()?;
    if formula::is_supported(&expr) {
        Some(format!("={formula}"))
    } else {
        None
    }
}

fn data_to_raw(data: &Data) -> Option<String> {
    match data {
        Data::Empty => None,
        Data::Int(value) => Some(value.to_string()),
        Data::Float(value) => Some(value.to_string()),
        Data::String(value) => Some(value.clone()),
        Data::Bool(value) => Some(if *value {
            "TRUE".to_string()
        } else {
            "FALSE".to_string()
        }),
        Data::DateTime(value) => Some(value.to_string()),
        Data::DateTimeIso(value) => Some(value.clone()),
        Data::DurationIso(value) => Some(value.clone()),
        Data::Error(value) => Some(value.to_string()),
    }
}

fn write_cell(
    worksheet: &mut Worksheet,
    row: u32,
    column: u16,
    raw: &str,
    format: Option<&Format>,
) -> Result<(), String> {
    if let Some(formula) = raw.strip_prefix('=')
        && formula::parse(formula).is_ok()
    {
        let result = match format {
            Some(format) => worksheet.write_formula_with_format(row, column, formula, format),
            None => worksheet.write_formula(row, column, formula),
        };
        result.map_err(|error| error.to_string())?;
        return Ok(());
    }

    let trimmed = raw.trim();
    let result = if trimmed.eq_ignore_ascii_case("TRUE") {
        match format {
            Some(format) => worksheet.write_boolean_with_format(row, column, true, format),
            None => worksheet.write_boolean(row, column, true),
        }
    } else if trimmed.eq_ignore_ascii_case("FALSE") {
        match format {
            Some(format) => worksheet.write_boolean_with_format(row, column, false, format),
            None => worksheet.write_boolean(row, column, false),
        }
    } else if let Ok(number) = trimmed.parse::<f64>() {
        match format {
            Some(format) => worksheet.write_number_with_format(row, column, number, format),
            None => worksheet.write_number(row, column, number),
        }
    } else {
        match format {
            Some(format) => worksheet.write_string_with_format(row, column, raw, format),
            None => worksheet.write_string(row, column, raw),
        }
    };
    result.map_err(|error| error.to_string())?;
    Ok(())
}

fn to_xlsx_format(cell_format: CellFormat) -> Format {
    let mut format = Format::new();
    if cell_format.bold {
        format = format.set_bold();
    }
    if let Some(color) = cell_format.text_color {
        format = format.set_font_color(XlsxColor::RGB(color.to_hex()));
    }
    if let Some(color) = cell_format.fill_color {
        format = format.set_background_color(XlsxColor::RGB(color.to_hex()));
    }
    if let Some(number_format) = cell_format.number_format.excel_format() {
        format = format.set_num_format(number_format);
    }
    format
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;
    use crate::format::{CellFormat, NumberFormat, Rgb};

    fn temp_path(name: &str) -> std::path::PathBuf {
        std::env::temp_dir().join(format!("sheetcraft-{}-{name}.xlsx", std::process::id()))
    }

    #[test]
    fn converts_numeric_cells() {
        assert_eq!(data_to_raw(&Data::Int(42)), Some("42".to_string()));
        assert_eq!(data_to_raw(&Data::Float(2.5)), Some("2.5".to_string()));
    }

    #[test]
    fn converts_text_and_boolean_cells() {
        assert_eq!(
            data_to_raw(&Data::String("hello".to_string())),
            Some("hello".to_string())
        );
        assert_eq!(data_to_raw(&Data::Bool(true)), Some("TRUE".to_string()));
        assert_eq!(data_to_raw(&Data::Bool(false)), Some("FALSE".to_string()));
    }

    #[test]
    fn skips_empty_cells() {
        assert_eq!(data_to_raw(&Data::Empty), None);
    }

    #[test]
    fn keeps_supported_formulas() {
        assert_eq!(
            importable_formula("SUM(A1:A3)"),
            Some("=SUM(A1:A3)".to_string())
        );
        assert_eq!(importable_formula("VLOOKUP(A1,B:C,2)"), None);
    }

    #[test]
    fn exports_formatted_cells() {
        let path = temp_path("formatted");
        let path_text = path.to_string_lossy().to_string();
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "1234.5").unwrap();
        sheet.set_format(
            CellRef::new(1, 1),
            CellFormat {
                bold: true,
                text_color: Some(Rgb::new(248, 113, 113)),
                fill_color: Some(Rgb::new(6, 78, 59)),
                number_format: NumberFormat::Currency { decimals: 2 },
            },
        );
        sheet.set_a1("A2", "=A1*2").unwrap();
        sheet.set_format(
            CellRef::new(2, 1),
            CellFormat {
                bold: true,
                number_format: NumberFormat::Currency { decimals: 2 },
                ..CellFormat::default()
            },
        );

        sheet.to_xlsx(&path_text).unwrap();
        let imported = Sheet::from_xlsx(&path_text).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(
            imported.value(&CellRef::new(1, 1)),
            CellValue::Number(1234.5)
        );
        assert_eq!(
            imported.value(&CellRef::new(2, 1)),
            CellValue::Number(2469.0)
        );
    }

    #[test]
    fn exports_and_reimports_formulas() {
        let path = temp_path("round-trip");
        let path = path.to_string_lossy().to_string();
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "2").unwrap();
        sheet.set_a1("A2", "3").unwrap();
        sheet.set_a1("A3", "=A1+A2").unwrap();
        sheet.set_a1("B1", "=SUM(A1:A2)").unwrap();
        sheet.set_a1("B2", "TRUE").unwrap();
        sheet.set_a1("B3", "text").unwrap();

        sheet.to_xlsx(&path).unwrap();
        let imported = Sheet::from_xlsx(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(imported.value(&CellRef::new(3, 1)), CellValue::Number(5.0));
        assert_eq!(imported.value(&CellRef::new(1, 2)), CellValue::Number(5.0));
        assert_eq!(
            imported.value(&CellRef::new(2, 2)),
            CellValue::Boolean(true)
        );
        assert_eq!(
            imported.value(&CellRef::new(3, 2)),
            CellValue::Text("text".to_string())
        );
    }
}
