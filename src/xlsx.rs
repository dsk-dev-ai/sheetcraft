use calamine::{Data, Reader, open_workbook_auto};

use crate::cell::CellRef;
use crate::engine::Sheet;

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
        let (start_row, start_column) = range.start().unwrap_or((0, 0));

        let mut sheet = Self::new();
        for (row_index, row) in range.rows().enumerate() {
            for (column_index, data) in row.iter().enumerate() {
                let Some(raw) = data_to_raw(data) else {
                    continue;
                };
                let reference = CellRef::new(
                    start_row + row_index as u32 + 1,
                    start_column + column_index as u32 + 1,
                );
                sheet.set(reference, raw);
            }
        }
        Ok(sheet)
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

#[cfg(test)]
mod tests {
    use super::*;

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
}
