use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct CellRef {
    pub row: u32,
    pub col: u32,
}

impl CellRef {
    pub fn new(row: u32, col: u32) -> Self {
        Self { row, col }
    }

    pub fn parse(input: &str) -> Result<Self, CellRefError> {
        let mut split = None;
        for (index, ch) in input.char_indices() {
            if ch.is_ascii_digit() {
                split = Some(index);
                break;
            }
        }
        let Some(index) = split else {
            return Err(CellRefError::Invalid(input.to_string()));
        };
        let (letters, digits) = input.split_at(index);
        if letters.is_empty()
            || digits.is_empty()
            || !letters.chars().all(|c| c.is_ascii_alphabetic())
            || !digits.chars().all(|c| c.is_ascii_digit())
        {
            return Err(CellRefError::Invalid(input.to_string()));
        }
        let mut col = 0u32;
        for ch in letters.chars() {
            let value = ch.to_ascii_uppercase() as u32 - 'A' as u32 + 1;
            col = col
                .checked_mul(26)
                .and_then(|v| v.checked_add(value))
                .ok_or_else(|| CellRefError::Invalid(input.to_string()))?;
        }
        let row = digits
            .parse::<u32>()
            .map_err(|_| CellRefError::Invalid(input.to_string()))?;
        if row == 0 {
            return Err(CellRefError::Invalid(input.to_string()));
        }
        Ok(Self { row, col })
    }

    pub fn column_name(mut col: u32) -> String {
        let mut name = Vec::new();
        while col > 0 {
            let rem = ((col - 1) % 26) as u8;
            name.push((b'A' + rem) as char);
            col = (col - 1) / 26;
        }
        name.iter().rev().collect()
    }
}

impl fmt::Display for CellRef {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}{}", Self::column_name(self.col), self.row)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CellRefError {
    Invalid(String),
}

impl fmt::Display for CellRefError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Invalid(value) => write!(f, "invalid cell reference: {value}"),
        }
    }
}

impl std::error::Error for CellRefError {}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CellError {
    DivZero,
    Value,
    Ref,
    Cycle,
    Name,
    Parse,
}

impl fmt::Display for CellError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::DivZero => "#DIV/0!",
            Self::Value => "#VALUE!",
            Self::Ref => "#REF!",
            Self::Cycle => "#CYCLE!",
            Self::Name => "#NAME?",
            Self::Parse => "#PARSE!",
        };
        f.write_str(text)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum CellValue {
    Empty,
    Number(f64),
    Text(String),
    Boolean(bool),
    Error(CellError),
}

impl CellValue {
    pub fn is_error(&self) -> bool {
        matches!(self, Self::Error(_))
    }
}

impl fmt::Display for CellValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Empty => Ok(()),
            Self::Number(value) => {
                if value.is_finite() && value.fract() == 0.0 && value.abs() < 1e15 {
                    write!(f, "{}", *value as i64)
                } else {
                    write!(f, "{value}")
                }
            }
            Self::Text(value) => f.write_str(value),
            Self::Boolean(value) => write!(f, "{}", if *value { "TRUE" } else { "FALSE" }),
            Self::Error(error) => write!(f, "{error}"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_reference() {
        assert_eq!(CellRef::parse("A1").unwrap(), CellRef::new(1, 1));
        assert_eq!(CellRef::parse("Z10").unwrap(), CellRef::new(10, 26));
    }

    #[test]
    fn parses_multi_letter_column() {
        assert_eq!(CellRef::parse("AA1").unwrap(), CellRef::new(1, 27));
        assert_eq!(CellRef::parse("AB12").unwrap(), CellRef::new(12, 28));
    }

    #[test]
    fn rejects_invalid_references() {
        assert!(CellRef::parse("").is_err());
        assert!(CellRef::parse("A0").is_err());
        assert!(CellRef::parse("1A").is_err());
        assert!(CellRef::parse("A").is_err());
    }

    #[test]
    fn round_trips_display() {
        for reference in ["A1", "Z9", "AA27", "XFD1048576"] {
            assert_eq!(CellRef::parse(reference).unwrap().to_string(), reference);
        }
    }
}
