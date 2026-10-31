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
        _ => usage(),
    }
}

fn usage() {
    println!("SheetCraft core engine");
    println!();
    println!("Usage:");
    println!("  sheetcraft demo");
    println!("  sheetcraft eval \"=SUM(A1:A3) * 2\"");
}

fn eval_formula(input: &str) {
    let mut sheet = Sheet::new();
    let target = CellRef::new(1, 26);
    let formula = input.strip_prefix('=').unwrap_or(input);
    sheet.set(target, format!("={formula}"));
    println!("{}", sheet.value(&target));
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
