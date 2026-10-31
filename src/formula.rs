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
}
