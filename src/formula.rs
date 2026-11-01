use std::fmt;

use crate::cell::{CellRef, CellRefError};

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Number(f64),
    Ref(CellRef),
    Range(CellRef, CellRef),
    Unary(UnaryOp, Box<Expr>),
    Binary(BinaryOp, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Reference {
    Cell(CellRef),
    Range(CellRef, CellRef),
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UnaryOp {
    Plus,
    Minus,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
}

#[derive(Clone, Debug, PartialEq)]
enum Token {
    Number(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
    Comma,
    Colon,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FormulaError {
    pub message: String,
}

impl FormulaError {
    fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for FormulaError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.message)
    }
}

impl std::error::Error for FormulaError {}

impl From<CellRefError> for FormulaError {
    fn from(value: CellRefError) -> Self {
        Self::new(value.to_string())
    }
}

pub fn parse(input: &str) -> Result<Expr, FormulaError> {
    let tokens = tokenize(input)?;
    if tokens.is_empty() {
        return Err(FormulaError::new("empty formula"));
    }
    let mut parser = Parser { tokens, pos: 0 };
    let expr = parser.parse_expr()?;
    if parser.pos != parser.tokens.len() {
        return Err(FormulaError::new("unexpected trailing tokens"));
    }
    Ok(expr)
}

/// Collect the cells and ranges referenced by a formula.
///
/// Returns an empty list when the input is not a formula or does not parse.
pub fn references(formula: &str) -> Vec<Reference> {
    let Some(formula) = formula.strip_prefix('=') else {
        return Vec::new();
    };
    let Ok(expr) = parse(formula) else {
        return Vec::new();
    };
    let mut references = Vec::new();
    collect_references(&expr, &mut references);
    references
}

/// Shift references whose rows fall inside a sorted block.
///
/// Formulas that cannot be parsed are returned unchanged. References outside
/// the block stay fixed, matching rows moving inside the selected range.
pub fn translate_rows(input: &str, top: u32, bottom: u32, delta: isize) -> String {
    let Some(body) = input.strip_prefix('=') else {
        return input.to_string();
    };
    let Ok(expr) = parse(body) else {
        return input.to_string();
    };
    if !is_supported(&expr) {
        return input.to_string();
    }

    let chars: Vec<char> = body.chars().collect();
    let mut output = String::from("=");
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len()
                && (chars[index].is_ascii_alphanumeric()
                    || chars[index] == '_'
                    || chars[index] == '.')
            {
                index += 1;
            }
            let token: String = chars[start..index].iter().collect();
            let mut after = index;
            while after < chars.len() && chars[after].is_whitespace() {
                after += 1;
            }
            let is_call = after < chars.len() && chars[after] == '(';
            if !is_call
                && let Ok(reference) = CellRef::parse(&token)
                && (top..=bottom).contains(&reference.row)
            {
                let row = reference.row as isize + delta;
                if (1..=1_048_576).contains(&row) {
                    output.push_str(&CellRef::new(row as u32, reference.col).to_string());
                    continue;
                }
            }
            output.push_str(&token);
        } else {
            output.push(ch);
            index += 1;
        }
    }
    output
}

fn collect_references(expr: &Expr, references: &mut Vec<Reference>) {
    match expr {
        Expr::Number(_) => {}
        Expr::Ref(reference) => references.push(Reference::Cell(*reference)),
        Expr::Range(start, end) => references.push(Reference::Range(*start, *end)),
        Expr::Unary(_, inner) => collect_references(inner, references),
        Expr::Binary(_, left, right) => {
            collect_references(left, references);
            collect_references(right, references);
        }
        Expr::Call(_, args) => {
            for arg in args {
                collect_references(arg, references);
            }
        }
    }
}

fn tokenize(input: &str) -> Result<Vec<Token>, FormulaError> {
    let chars: Vec<char> = input.chars().collect();
    let mut tokens = Vec::new();
    let mut index = 0;
    while index < chars.len() {
        let ch = chars[index];
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch.is_ascii_digit() || ch == '.' {
            let start = index;
            let mut seen_dot = ch == '.';
            index += 1;
            while index < chars.len() {
                let next = chars[index];
                if next.is_ascii_digit() {
                    index += 1;
                } else if next == '.' && !seen_dot {
                    seen_dot = true;
                    index += 1;
                } else {
                    break;
                }
            }
            let text: String = chars[start..index].iter().collect();
            let value = text
                .parse::<f64>()
                .map_err(|_| FormulaError::new(format!("invalid number: {text}")))?;
            tokens.push(Token::Number(value));
            continue;
        }
        if ch.is_ascii_alphabetic() || ch == '_' {
            let start = index;
            index += 1;
            while index < chars.len() {
                let next = chars[index];
                if next.is_ascii_alphanumeric() || next == '_' || next == '.' {
                    index += 1;
                } else {
                    break;
                }
            }
            tokens.push(Token::Ident(chars[start..index].iter().collect()));
            continue;
        }
        let token = match ch {
            '+' => Token::Plus,
            '-' => Token::Minus,
            '*' => Token::Star,
            '/' => Token::Slash,
            '^' => Token::Caret,
            '(' => Token::LParen,
            ')' => Token::RParen,
            ',' => Token::Comma,
            ':' => Token::Colon,
            _ => return Err(FormulaError::new(format!("unexpected character: {ch}"))),
        };
        tokens.push(token);
        index += 1;
    }
    Ok(tokens)
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.tokens.get(self.pos)
    }

    fn next(&mut self) -> Option<Token> {
        let token = self.tokens.get(self.pos).cloned();
        if token.is_some() {
            self.pos += 1;
        }
        token
    }

    fn eat(&mut self, token: &Token) -> bool {
        if self.peek() == Some(token) {
            self.pos += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, token: Token) -> Result<(), FormulaError> {
        if self.eat(&token) {
            Ok(())
        } else {
            Err(FormulaError::new(format!("expected {token:?}")))
        }
    }

    fn parse_expr(&mut self) -> Result<Expr, FormulaError> {
        self.parse_additive()
    }

    fn parse_additive(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.parse_multiplicative()?;
        loop {
            let op = if self.eat(&Token::Plus) {
                BinaryOp::Add
            } else if self.eat(&Token::Minus) {
                BinaryOp::Sub
            } else {
                break;
            };
            let right = self.parse_multiplicative()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_multiplicative(&mut self) -> Result<Expr, FormulaError> {
        let mut left = self.parse_unary()?;
        loop {
            let op = if self.eat(&Token::Star) {
                BinaryOp::Mul
            } else if self.eat(&Token::Slash) {
                BinaryOp::Div
            } else {
                break;
            };
            let right = self.parse_unary()?;
            left = Expr::Binary(op, Box::new(left), Box::new(right));
        }
        Ok(left)
    }

    fn parse_unary(&mut self) -> Result<Expr, FormulaError> {
        if self.eat(&Token::Minus) {
            return Ok(Expr::Unary(UnaryOp::Minus, Box::new(self.parse_unary()?)));
        }
        if self.eat(&Token::Plus) {
            return Ok(Expr::Unary(UnaryOp::Plus, Box::new(self.parse_unary()?)));
        }
        self.parse_power()
    }

    fn parse_power(&mut self) -> Result<Expr, FormulaError> {
        let left = self.parse_primary()?;
        if self.eat(&Token::Caret) {
            let right = self.parse_unary()?;
            return Ok(Expr::Binary(BinaryOp::Pow, Box::new(left), Box::new(right)));
        }
        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, FormulaError> {
        match self.next() {
            Some(Token::Number(value)) => Ok(Expr::Number(value)),
            Some(Token::Ident(name)) => {
                if self.eat(&Token::LParen) {
                    return self.parse_call(name);
                }
                let start = CellRef::parse(&name)?;
                if self.eat(&Token::Colon) {
                    match self.next() {
                        Some(Token::Ident(end)) => Ok(Expr::Range(start, CellRef::parse(&end)?)),
                        _ => Err(FormulaError::new("expected a cell reference after ':'")),
                    }
                } else {
                    Ok(Expr::Ref(start))
                }
            }
            Some(Token::LParen) => {
                let expr = self.parse_expr()?;
                self.expect(Token::RParen)?;
                Ok(expr)
            }
            other => Err(FormulaError::new(format!("unexpected token: {other:?}"))),
        }
    }

    fn parse_call(&mut self, name: String) -> Result<Expr, FormulaError> {
        let mut args = Vec::new();
        if self.eat(&Token::RParen) {
            return Ok(Expr::Call(name.to_ascii_uppercase(), args));
        }
        loop {
            args.push(self.parse_expr()?);
            if self.eat(&Token::Comma) {
                continue;
            }
            self.expect(Token::RParen)?;
            break;
        }
        Ok(Expr::Call(name.to_ascii_uppercase(), args))
    }
}

pub const SUPPORTED_FUNCTIONS: &[&str] = &["SUM", "AVERAGE", "MIN", "MAX"];

pub fn is_supported(expr: &Expr) -> bool {
    match expr {
        Expr::Number(_) | Expr::Ref(_) | Expr::Range(_, _) => true,
        Expr::Unary(_, inner) => is_supported(inner),
        Expr::Binary(_, left, right) => is_supported(left) && is_supported(right),
        Expr::Call(name, args) => {
            SUPPORTED_FUNCTIONS.contains(&name.as_str()) && args.iter().all(is_supported)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse_text(input: &str) -> Expr {
        parse(input).expect("formula should parse")
    }

    #[test]
    fn parses_precedence() {
        assert_eq!(
            parse_text("1+2*3"),
            Expr::Binary(
                BinaryOp::Add,
                Box::new(Expr::Number(1.0)),
                Box::new(Expr::Binary(
                    BinaryOp::Mul,
                    Box::new(Expr::Number(2.0)),
                    Box::new(Expr::Number(3.0)),
                )),
            )
        );
    }

    #[test]
    fn parses_parentheses_and_references() {
        let expr = parse_text("(A1+B2)*2");
        assert_eq!(
            expr,
            Expr::Binary(
                BinaryOp::Mul,
                Box::new(Expr::Binary(
                    BinaryOp::Add,
                    Box::new(Expr::Ref(CellRef::new(1, 1))),
                    Box::new(Expr::Ref(CellRef::new(2, 2))),
                )),
                Box::new(Expr::Number(2.0)),
            )
        );
    }

    #[test]
    fn parses_ranges_and_calls() {
        let expr = parse_text("sum(A1:B3, 4)");
        assert_eq!(
            expr,
            Expr::Call(
                "SUM".to_string(),
                vec![
                    Expr::Range(CellRef::new(1, 1), CellRef::new(3, 2)),
                    Expr::Number(4.0),
                ],
            )
        );
    }

    #[test]
    fn parses_power_right_associative() {
        let expr = parse_text("2^3^2");
        assert_eq!(
            expr,
            Expr::Binary(
                BinaryOp::Pow,
                Box::new(Expr::Number(2.0)),
                Box::new(Expr::Binary(
                    BinaryOp::Pow,
                    Box::new(Expr::Number(3.0)),
                    Box::new(Expr::Number(2.0)),
                )),
            )
        );
    }

    #[test]
    fn rejects_invalid_formulas() {
        assert!(parse("1+").is_err());
        assert!(parse("SUM(A1").is_err());
        assert!(parse("A0").is_err());
        assert!(parse("1 $ 2").is_err());
    }

    #[test]
    fn collects_references_from_formulas() {
        assert_eq!(
            references("=SUM(A1:B2) * C3"),
            vec![
                Reference::Range(CellRef::new(1, 1), CellRef::new(2, 2)),
                Reference::Cell(CellRef::new(3, 3)),
            ]
        );
    }

    #[test]
    fn ignores_text_and_invalid_formulas() {
        assert!(references("A1").is_empty());
        assert!(references("=SUM(A1").is_empty());
    }

    #[test]
    fn translates_references_inside_sorted_rows() {
        assert_eq!(translate_rows("=B2*C2", 2, 4, 3), "=B5*C5");
        assert_eq!(translate_rows("=SUM(B1:B2)+B3", 2, 4, 1), "=SUM(B1:B3)+B4");
        assert_eq!(translate_rows("=$B$2*C2", 2, 4, 3), "=$B$2*C2");
        assert_eq!(translate_rows("=SUM(A1:A3)", 2, 4, 1), "=SUM(A1:A4)");
    }
}
