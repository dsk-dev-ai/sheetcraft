use std::io::Read;

use sheetcraft::{CellRef, CellValue, Sheet};

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("demo") => demo(),
        Some("eval") => {
            if args.len() < 2 {
                usage();
                std::process::exit(1);
            }
            eval_formula(&args[1..].join(" "));
        }
        Some("csv") => {
            if args.len() < 2 {
                usage();
                std::process::exit(1);
            }
            match read_input(&args[1]) {
                Ok(input) => preview_sheet(&Sheet::from_csv(&input)),
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("xlsx") => {
            if args.len() < 2 {
                usage();
                std::process::exit(1);
            }
            match Sheet::from_xlsx(&args[1]) {
                Ok(sheet) => preview_sheet(&sheet),
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            }
        }
        Some("convert") => {
            if args.len() < 3 {
                usage();
                std::process::exit(1);
            }
            match load_sheet(&args[1]).and_then(|sheet| save_sheet(&sheet, &args[2])) {
                Ok(()) => println!("wrote {}", args[2]),
                Err(error) => {
                    eprintln!("error: {error}");
                    std::process::exit(1);
                }
            }
        }
        _ => usage(),
    }
}

fn usage() {
    println!("SheetCraft core engine");
    println!();
    println!("Usage:");
    println!("  sheetcraft demo");
    println!("  sheetcraft eval \"=SUM(A1:A3) * 2\"");
    println!("  sheetcraft csv <file.csv>      preview a CSV file, use - for stdin");
    println!("  sheetcraft xlsx <file.xlsx>    preview the first worksheet of an XLSX file");
    println!("  sheetcraft convert <in> <out>  convert between .csv and .xlsx");
}

fn extension(path: &str) -> Option<String> {
    std::path::Path::new(path)
        .extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| extension.to_ascii_lowercase())
}

fn load_sheet(path: &str) -> Result<Sheet, String> {
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

fn save_sheet(sheet: &Sheet, path: &str) -> Result<(), String> {
    match extension(path).as_deref() {
        Some("csv") => std::fs::write(path, sheet.to_csv()).map_err(|error| error.to_string()),
        Some("xlsx") => sheet.to_xlsx(path),
        Some(other) => Err(format!("unsupported output format: {other}")),
        None => Err("output path has no extension".to_string()),
    }
}

fn read_input(path: &str) -> std::io::Result<String> {
    if path == "-" {
        let mut input = String::new();
        std::io::stdin().read_to_string(&mut input)?;
        Ok(input)
    } else {
        std::fs::read_to_string(path)
    }
}

fn eval_formula(input: &str) {
    let mut sheet = Sheet::new();
    let target = CellRef::new(1, 26);
    let formula = input.strip_prefix('=').unwrap_or(input);
    sheet.set(target, format!("={formula}"));
    println!("{}", sheet.value(&target));
}

fn preview_sheet(sheet: &Sheet) {
    let (rows, columns) = sheet.bounds().unwrap_or((0, 0));
    println!("{rows} rows x {columns} columns");
    if rows == 0 {
        return;
    }

    let shown_rows = rows.min(10);
    let shown_columns = columns.min(8);
    print!("{:<4}", "");
    for column in 1..=shown_columns {
        print!("{:<14}", CellRef::column_name(column));
    }
    println!();
    for row in 1..=shown_rows {
        print!("{row:<4}");
        for column in 1..=shown_columns {
            let value = sheet.value(&CellRef::new(row, column));
            let text = match value {
                CellValue::Empty => String::new(),
                other => other.to_string(),
            };
            print!("{text:<14}");
        }
        println!();
    }
    if rows > shown_rows || columns > shown_columns {
        println!("...");
    }
}

fn demo() {
    let mut sheet = Sheet::new();
    let cells = [
        ("A1", "1"),
        ("A2", "2"),
        ("A3", "3"),
        ("B1", "=SUM(A1:A3)"),
        ("B2", "=B1*2"),
        ("C1", "=AVERAGE(A1:A3)"),
        ("D1", "=2^10"),
    ];
    for (reference, raw) in cells {
        sheet.set_a1(reference, raw).unwrap();
    }

    println!("{:<4} {:<14} {:<14} {:<14} {:<14}", "", "A", "B", "C", "D");
    for row in 1..=3 {
        print!("{row:<4}");
        for col in 1..=4 {
            let reference = CellRef::new(row, col);
            let value = sheet.value(&reference);
            let cell = match value {
                CellValue::Empty => String::new(),
                other => other.to_string(),
            };
            print!("{cell:<14}");
        }
        println!();
    }
}
