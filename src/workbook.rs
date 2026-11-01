use crate::engine::Sheet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Workbook {
    sheets: Vec<SheetEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct SheetEntry {
    name: String,
    sheet: Sheet,
}

impl Default for Workbook {
    fn default() -> Self {
        Self::new()
    }
}

impl Workbook {
    pub fn new() -> Self {
        Self::from_sheet("Sheet1", Sheet::new())
    }

    pub fn empty() -> Self {
        Self { sheets: Vec::new() }
    }

    pub fn from_sheet(name: impl Into<String>, sheet: Sheet) -> Self {
        Self {
            sheets: vec![SheetEntry {
                name: name.into(),
                sheet,
            }],
        }
    }

    pub fn from_csv(input: &str) -> Self {
        Self::from_sheet("Sheet1", Sheet::from_csv(input))
    }

    pub fn len(&self) -> usize {
        self.sheets.len()
    }

    pub fn is_empty(&self) -> bool {
        self.sheets.is_empty()
    }

    pub fn name(&self, index: usize) -> Option<&str> {
        self.sheets.get(index).map(|entry| entry.name.as_str())
    }

    pub fn sheet(&self, index: usize) -> Option<&Sheet> {
        self.sheets.get(index).map(|entry| &entry.sheet)
    }

    pub fn sheet_mut(&mut self, index: usize) -> Option<&mut Sheet> {
        self.sheets.get_mut(index).map(|entry| &mut entry.sheet)
    }

    pub fn first(&self) -> Option<&Sheet> {
        self.sheet(0)
    }

    pub fn push(&mut self, name: impl Into<String>, sheet: Sheet) {
        self.sheets.push(SheetEntry {
            name: name.into(),
            sheet,
        });
    }

    pub fn remove(&mut self, index: usize) -> Option<(String, Sheet)> {
        if self.sheets.len() <= 1 || index >= self.sheets.len() {
            return None;
        }
        let entry = self.sheets.remove(index);
        Some((entry.name, entry.sheet))
    }

    pub fn rename(&mut self, index: usize, name: impl Into<String>) -> bool {
        let name = name.into();
        if name.trim().is_empty() {
            return false;
        }
        match self.sheets.get_mut(index) {
            Some(entry) => {
                entry.name = name;
                true
            }
            None => false,
        }
    }

    pub fn next_sheet_name(&self) -> String {
        for number in 1.. {
            let candidate = format!("Sheet{number}");
            if !self.sheets.iter().any(|entry| entry.name == candidate) {
                return candidate;
            }
        }
        unreachable!()
    }

    pub fn iter(&self) -> impl Iterator<Item = (&str, &Sheet)> {
        self.sheets
            .iter()
            .map(|entry| (entry.name.as_str(), &entry.sheet))
    }

    pub fn to_csv(&self) -> Result<String, String> {
        match self.sheets.as_slice() {
            [entry] => Ok(entry.sheet.to_csv()),
            [] => Ok(String::new()),
            _ => Err("CSV export supports one sheet; save as XLSX instead".to_string()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn manages_sheets_and_names() {
        let mut workbook = Workbook::new();
        assert_eq!(workbook.len(), 1);
        assert_eq!(workbook.name(0), Some("Sheet1"));
        assert_eq!(workbook.next_sheet_name(), "Sheet2");
        workbook.push("Data", Sheet::new());
        assert_eq!(workbook.len(), 2);
        assert_eq!(workbook.next_sheet_name(), "Sheet2");
        assert!(workbook.rename(1, "Notes"));
        assert_eq!(workbook.name(1), Some("Notes"));
        assert!(!workbook.rename(1, "  "));
        assert!(!workbook.rename(99, "Missing"));
        let removed = workbook.remove(1).unwrap();
        assert_eq!(removed.0, "Notes");
        assert_eq!(workbook.len(), 1);
        assert!(workbook.remove(0).is_none());
    }

    #[test]
    fn exports_csv_only_for_single_sheet_workbooks() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "value").unwrap();
        let workbook = Workbook::from_sheet("Sheet1", sheet);
        assert_eq!(workbook.to_csv().unwrap(), "value");

        let mut workbook = Workbook::new();
        workbook.push("Second", Sheet::new());
        assert!(workbook.to_csv().is_err());
    }
}
