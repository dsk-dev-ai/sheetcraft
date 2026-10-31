use std::collections::{BTreeMap, HashSet};

use crate::cell::{CellError, CellRef, CellValue};
use crate::formula::{self, BinaryOp, Expr, UnaryOp};

#[derive(Default)]
pub struct Sheet {
    cells: BTreeMap<CellRef, String>,
}

impl Sheet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn set(&mut self, reference: CellRef, raw: impl Into<String>) {
        self.cells.insert(reference, raw.into());
    }

    pub fn clear(&mut self, reference: &CellRef) {
        self.cells.remove(reference);
    }

    pub fn set_a1(
        &mut self,
        reference: &str,
        raw: impl Into<String>,
    ) -> Result<(), crate::cell::CellRefError> {
        let reference = CellRef::parse(reference)?;
        self.set(reference, raw);
        Ok(())
    }

    pub fn raw(&self, reference: &CellRef) -> Option<&str> {
        self.cells.get(reference).map(String::as_str)
    }

    pub fn value(&self, reference: &CellRef) -> CellValue {
        let mut visiting = HashSet::new();
        self.eval_ref(reference, &mut visiting)
    }

    pub fn from_csv(input: &str) -> Self {
        let mut sheet = Self::new();
        for (row, fields) in crate::csv::parse(input).iter().enumerate() {
            for (column, field) in fields.iter().enumerate() {
                if !field.is_empty() {
                    let reference = CellRef::new(row as u32 + 1, column as u32 + 1);
                    sheet.set(reference, field.clone());
                }
            }
        }
        sheet
    }

    pub fn to_csv(&self) -> String {
        self.to_csv_with(|reference| self.raw(reference).unwrap_or_default().to_string())
    }

    pub fn to_csv_values(&self) -> String {
        self.to_csv_with(|reference| self.value(reference).to_string())
    }

    fn to_csv_with<F>(&self, mut cell_text: F) -> String
    where
        F: FnMut(&CellRef) -> String,
    {
        let Some((rows, columns)) = self.bounds() else {
            return String::new();
        };
        let mut output = Vec::with_capacity(rows as usize);
        for row in 1..=rows {
            let mut fields = Vec::with_capacity(columns as usize);
            for column in 1..=columns {
                let reference = CellRef::new(row, column);
                fields.push(cell_text(&reference));
            }
            output.push(fields);
        }
        crate::csv::write(&output)
    }

    pub fn bounds(&self) -> Option<(u32, u32)> {
        let mut rows = 0;
        let mut columns = 0;
        for reference in self.cells.keys() {
            rows = rows.max(reference.row);
            columns = columns.max(reference.col);
        }
        if rows == 0 || columns == 0 {
            None
        } else {
            Some((rows, columns))
        }
    }

    fn eval_ref(&self, reference: &CellRef, visiting: &mut HashSet<CellRef>) -> CellValue {
        if visiting.contains(reference) {
            return CellValue::Error(CellError::Cycle);
        }
        visiting.insert(*reference);
        let value = self.eval_cell(reference, visiting);
        visiting.remove(reference);
        value
    }

    fn eval_cell(&self, reference: &CellRef, visiting: &mut HashSet<CellRef>) -> CellValue {
        let Some(raw) = self.cells.get(reference) else {
            return CellValue::Empty;
        };
        if let Some(formula) = raw.strip_prefix('=') {
            return match formula::parse(formula) {
                Ok(expr) => self.eval_expr(&expr, visiting),
                Err(_) => CellValue::Error(CellError::Parse),
            };
        }
        parse_literal(raw)
    }

    fn eval_expr(&self, expr: &Expr, visiting: &mut HashSet<CellRef>) -> CellValue {
        match expr {
            Expr::Number(value) => CellValue::Number(*value),
            Expr::Ref(reference) => self.eval_ref(reference, visiting),
            Expr::Range(_, _) => CellValue::Error(CellError::Value),
            Expr::Unary(op, inner) => {
                let value = match as_number(self.eval_expr(inner, visiting)) {
                    Ok(value) => value,
                    Err(error) => return error,
                };
                CellValue::Number(match op {
                    UnaryOp::Plus => value,
                    UnaryOp::Minus => -value,
                })
            }
            Expr::Binary(op, left, right) => {
                let left = match as_number(self.eval_expr(left, visiting)) {
                    Ok(value) => value,
                    Err(error) => return error,
                };
                let right = match as_number(self.eval_expr(right, visiting)) {
                    Ok(value) => value,
                    Err(error) => return error,
                };
                apply_binary(*op, left, right)
            }
            Expr::Call(name, args) => self.eval_call(name, args, visiting),
        }
    }

    fn eval_call(&self, name: &str, args: &[Expr], visiting: &mut HashSet<CellRef>) -> CellValue {
        match name {
            "SUM" => match self.collect_numbers(args, visiting) {
                Ok(values) => CellValue::Number(values.iter().sum()),
                Err(error) => error,
            },
            "AVERAGE" => match self.collect_numbers(args, visiting) {
                Ok(values) if values.is_empty() => CellValue::Error(CellError::DivZero),
                Ok(values) => CellValue::Number(values.iter().sum::<f64>() / values.len() as f64),
                Err(error) => error,
            },
            "MIN" => match self.collect_numbers(args, visiting) {
                Ok(values) => {
                    CellValue::Number(values.iter().copied().reduce(f64::min).unwrap_or(0.0))
                }
                Err(error) => error,
            },
            "MAX" => match self.collect_numbers(args, visiting) {
                Ok(values) => {
                    CellValue::Number(values.iter().copied().reduce(f64::max).unwrap_or(0.0))
                }
                Err(error) => error,
            },
            _ => CellValue::Error(CellError::Name),
        }
    }

    fn collect_numbers(
        &self,
        args: &[Expr],
        visiting: &mut HashSet<CellRef>,
    ) -> Result<Vec<f64>, CellValue> {
        let mut values = Vec::new();
        for arg in args {
            match arg {
                Expr::Range(start, end) => {
                    let (row_start, row_end) = ordered(start.row, end.row);
                    let (col_start, col_end) = ordered(start.col, end.col);
                    let count = (row_end - row_start + 1) as u64 * (col_end - col_start + 1) as u64;
                    if count > 1_000_000 {
                        return Err(CellValue::Error(CellError::Ref));
                    }
                    for row in row_start..=row_end {
                        for col in col_start..=col_end {
                            let value = self.eval_ref(&CellRef::new(row, col), visiting);
                            match value {
                                CellValue::Number(value) => values.push(value),
                                CellValue::Boolean(value) => {
                                    values.push(if value { 1.0 } else { 0.0 })
                                }
                                CellValue::Error(error) => return Err(CellValue::Error(error)),
                                _ => {}
                            }
                        }
                    }
                }
                other => {
                    let value = self.eval_expr(other, visiting);
                    match value {
                        CellValue::Number(value) => values.push(value),
                        CellValue::Boolean(value) => values.push(if value { 1.0 } else { 0.0 }),
                        CellValue::Error(error) => return Err(CellValue::Error(error)),
                        _ => {}
                    }
                }
            }
        }
        Ok(values)
    }
}

fn ordered(a: u32, b: u32) -> (u32, u32) {
    if a <= b { (a, b) } else { (b, a) }
}

fn as_number(value: CellValue) -> Result<f64, CellValue> {
    match value {
        CellValue::Empty => Ok(0.0),
        CellValue::Number(value) => Ok(value),
        CellValue::Boolean(value) => Ok(if value { 1.0 } else { 0.0 }),
        CellValue::Error(error) => Err(CellValue::Error(error)),
        CellValue::Text(_) => Err(CellValue::Error(CellError::Value)),
    }
}

fn apply_binary(op: BinaryOp, left: f64, right: f64) -> CellValue {
    match op {
        BinaryOp::Add => CellValue::Number(left + right),
        BinaryOp::Sub => CellValue::Number(left - right),
        BinaryOp::Mul => CellValue::Number(left * right),
        BinaryOp::Div => {
            if right == 0.0 {
                CellValue::Error(CellError::DivZero)
            } else {
                CellValue::Number(left / right)
            }
        }
        BinaryOp::Pow => CellValue::Number(left.powf(right)),
    }
}

fn parse_literal(raw: &str) -> CellValue {
    let trimmed = raw.trim();
    if trimmed.eq_ignore_ascii_case("TRUE") {
        return CellValue::Boolean(true);
    }
    if trimmed.eq_ignore_ascii_case("FALSE") {
        return CellValue::Boolean(false);
    }
    if let Ok(value) = trimmed.parse::<f64>() {
        return CellValue::Number(value);
    }
    CellValue::Text(raw.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn reference(a1: &str) -> CellRef {
        CellRef::parse(a1).unwrap()
    }

    #[test]
    fn evaluates_arithmetic() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "=1+2*3").unwrap();
        assert_eq!(sheet.value(&reference("A1")), CellValue::Number(7.0));
    }

    #[test]
    fn resolves_references_and_ranges() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "1").unwrap();
        sheet.set_a1("A2", "2").unwrap();
        sheet.set_a1("A3", "3").unwrap();
        sheet.set_a1("B1", "=SUM(A1:A3)").unwrap();
        sheet.set_a1("B2", "=B1*2").unwrap();
        assert_eq!(sheet.value(&reference("B2")), CellValue::Number(12.0));
    }

    #[test]
    fn evaluates_functions() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "4").unwrap();
        sheet.set_a1("A2", "8").unwrap();
        sheet.set_a1("B1", "=AVERAGE(A1:A2)").unwrap();
        sheet.set_a1("B2", "=MIN(A1:A2, 2)").unwrap();
        sheet.set_a1("B3", "=MAX(A1:A2, 10)").unwrap();
        assert_eq!(sheet.value(&reference("B1")), CellValue::Number(6.0));
        assert_eq!(sheet.value(&reference("B2")), CellValue::Number(2.0));
        assert_eq!(sheet.value(&reference("B3")), CellValue::Number(10.0));
    }

    #[test]
    fn detects_circular_references() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "=A2").unwrap();
        sheet.set_a1("A2", "=A1").unwrap();
        assert_eq!(
            sheet.value(&reference("A1")),
            CellValue::Error(CellError::Cycle)
        );
    }

    #[test]
    fn reports_division_by_zero() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "=1/0").unwrap();
        assert_eq!(
            sheet.value(&reference("A1")),
            CellValue::Error(CellError::DivZero)
        );
    }

    #[test]
    fn treats_text_as_value_error_in_math() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "hello").unwrap();
        sheet.set_a1("A2", "=A1+1").unwrap();
        assert_eq!(
            sheet.value(&reference("A2")),
            CellValue::Error(CellError::Value)
        );
    }

    #[test]
    fn parses_literals() {
        let mut sheet = Sheet::new();
        sheet.set_a1("A1", "42.5").unwrap();
        sheet.set_a1("A2", "TRUE").unwrap();
        sheet.set_a1("A3", "text").unwrap();
        assert_eq!(sheet.value(&reference("A1")), CellValue::Number(42.5));
        assert_eq!(sheet.value(&reference("A2")), CellValue::Boolean(true));
        assert_eq!(
            sheet.value(&reference("A3")),
            CellValue::Text("text".to_string())
        );
    }

    #[test]
    fn imports_and_evaluates_csv() {
        let input = "name,value\nwidget,4\nformula,=4*2\n";
        let sheet = Sheet::from_csv(input);
        assert_eq!(sheet.value(&reference("B3")), CellValue::Number(8.0));
        assert_eq!(
            sheet.value(&reference("A2")),
            CellValue::Text("widget".to_string())
        );
    }

    #[test]
    fn exports_raw_and_computed_csv() {
        let input = "name,value\nwidget,4\nformula,=4*2";
        let sheet = Sheet::from_csv(input);
        assert_eq!(sheet.to_csv(), input);
        assert_eq!(sheet.to_csv_values(), "name,value\nwidget,4\nformula,8");
    }

    #[test]
    fn reports_bounds() {
        let mut sheet = Sheet::new();
        assert_eq!(sheet.bounds(), None);
        sheet.set_a1("C7", "x").unwrap();
        assert_eq!(sheet.bounds(), Some((7, 3)));
    }

    #[test]
    fn csv_round_trip_escapes_fields() {
        let input = "\"a,b\",\"say \"\"hi\"\"\"\n\"multi\nline\",plain";
        let sheet = Sheet::from_csv(input);
        assert_eq!(sheet.to_csv(), input);
    }
}
