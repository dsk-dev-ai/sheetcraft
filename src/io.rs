use crate::engine::Sheet;
use crate::workbook::Workbook;

pub fn load(path: &str) -> Result<Sheet, String> {
    load_workbook(path)?
        .first()
        .cloned()
        .ok_or_else(|| "workbook has no worksheets".to_string())
}

pub fn load_workbook(path: &str) -> Result<Workbook, String> {
    match extension(path).as_deref() {
        Some("csv") => {
            let input = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
            Ok(Workbook::from_csv(&input))
        }
        Some("xlsx") => Workbook::from_xlsx(path),
        Some(other) => Err(format!("unsupported input format: {other}")),
        None => Err("input path has no extension".to_string()),
    }
}

pub fn save(sheet: &Sheet, path: &str) -> Result<(), String> {
    match extension(path).as_deref() {
        Some("csv") => std::fs::write(path, sheet.to_csv()).map_err(|error| error.to_string()),
        Some("xlsx") => sheet.to_xlsx(path),
        Some(other) => Err(format!("unsupported output format: {other}")),
        None => Err("output path has no extension".to_string()),
    }
}

pub fn save_workbook(workbook: &Workbook, path: &str) -> Result<(), String> {
    match extension(path).as_deref() {
        Some("csv") => std::fs::write(path, workbook.to_csv()?).map_err(|error| error.to_string()),
        Some("xlsx") => workbook.to_xlsx(path),
        Some(other) => Err(format!("unsupported output format: {other}")),
        None => Err("output path has no extension".to_string()),
    }
}

fn extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}
