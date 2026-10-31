use calamine::{Data, Range, Reader, open_workbook_auto};
use rust_xlsxwriter::{Workbook, Worksheet};

use crate::cell::CellRef;
use crate::engine::Sheet;
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
                    write_cell(worksheet, row - 1, (column - 1) as u16, raw)?;
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

fn write_cell(worksheet: &mut Worksheet, row: u32, column: u16, raw: &str) -> Result<(), String> {
    if let Some(formula) = raw.strip_prefix('=')
        && formula::parse(formula).is_ok()
    {
        worksheet
            .write_formula(row, column, formula)
            .map_err(|error| error.to_string())?;
        return Ok(());
    }

    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("TRUE") {
        worksheet
            .write_boolean(row, column, true)
            .map_err(|error| error.to_string())?;
    } else if trimmed.eq_ignore_ascii_case("FALSE") {
        worksheet
            .write_boolean(row, column, false)
            .map_err(|error| error.to_string())?;
    } else if let Ok(number) = trimmed.parse::<f64>() {
        worksheet
            .write_number(row, column, number)
            .map_err(|error| error.to_string())?;
    } else {
        worksheet
            .write_string(row, column, raw)
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cell::CellValue;

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
