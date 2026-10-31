use crate::engine::Sheet;

pub fn load(path: &str) -> Result<Sheet, String> {
    match extension(path).as_deref() {
        Some("csv") => {
            let input = std::fs::read_to_string(path).map_err(|error| error.to_string())?;
            Ok(Sheet::from_csv(&input))
        }
        Some("xlsx") => Sheet::from_xlsx(path),
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

fn extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}
