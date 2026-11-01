use std::fmt;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Rgb {
    pub r: u8,
    pub g: u8,
    pub b: u8,
}

pub const BORDER_COLOR: Rgb = Rgb::new(148, 163, 184);

impl Rgb {
    pub const fn new(r: u8, g: u8, b: u8) -> Self {
        Self { r, g, b }
    }

    pub const fn to_hex(self) -> u32 {
        ((self.r as u32) << 16) | ((self.g as u32) << 8) | self.b as u32
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum NumberFormat {
    #[default]
    General,
    Number {
        decimals: u8,
    },
    Currency {
        decimals: u8,
    },
    Percent {
        decimals: u8,
    },
}

impl NumberFormat {
    pub fn apply(self, value: f64) -> String {
        match self {
            Self::General => value.to_string(),
            Self::Number { decimals } => format_fixed(value, decimals),
            Self::Currency { decimals } => {
                let text = format_fixed(value, decimals);
                match text.strip_prefix('-') {
                    Some(rest) => format!("-${rest}"),
                    None => format!("${text}"),
                }
            }
            Self::Percent { decimals } => {
                format!("{}%", format_fixed(value * 100.0, decimals))
            }
        }
    }

    pub fn excel_format(self) -> Option<String> {
        let zeros = |decimals: u8| "0".repeat(decimals.min(10) as usize);
        match self {
            Self::General => None,
            Self::Number { decimals } => {
                let decimals = decimals.min(10);
                if decimals == 0 {
                    Some("#,##0".to_string())
                } else {
                    Some(format!("#,##0.{}", zeros(decimals)))
                }
            }
            Self::Currency { decimals } => {
                let decimals = decimals.min(10);
                if decimals == 0 {
                    Some("$#,##0".to_string())
                } else {
                    Some(format!("$#,##0.{}", zeros(decimals)))
                }
            }
            Self::Percent { decimals } => {
                let decimals = decimals.min(10);
                if decimals == 0 {
                    Some("0%".to_string())
                } else {
                    Some(format!("0.{}%", zeros(decimals)))
                }
            }
        }
    }
}

impl fmt::Display for NumberFormat {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let text = match self {
            Self::General => "General".to_string(),
            Self::Number { decimals } => format!("{:.prec$}", 0, prec = *decimals as usize),
            Self::Currency { decimals } => format!("${:.prec$}", 0, prec = *decimals as usize),
            Self::Percent { decimals } => format!("{:.prec$}%", 0, prec = *decimals as usize),
        };
        f.write_str(&text)
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct Borders {
    pub top: bool,
    pub right: bool,
    pub bottom: bool,
    pub left: bool,
}

impl Borders {
    pub const ALL: Self = Self {
        top: true,
        right: true,
        bottom: true,
        left: true,
    };

    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub enum BorderPreset {
    #[default]
    None,
    All,
    Outer,
    Bottom,
}

impl BorderPreset {
    pub const fn label(self) -> &'static str {
        match self {
            Self::None => "No borders",
            Self::All => "All borders",
            Self::Outer => "Outer borders",
            Self::Bottom => "Bottom border",
        }
    }

    pub const fn borders(
        self,
        is_top: bool,
        is_left: bool,
        is_bottom: bool,
        is_right: bool,
    ) -> Borders {
        match self {
            Self::None => Borders {
                top: false,
                right: false,
                bottom: false,
                left: false,
            },
            Self::All => Borders::ALL,
            Self::Outer => Borders {
                top: is_top,
                right: is_right,
                bottom: is_bottom,
                left: is_left,
            },
            Self::Bottom => Borders {
                top: false,
                right: false,
                bottom: true,
                left: false,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Hash)]
pub struct CellFormat {
    pub bold: bool,
    pub text_color: Option<Rgb>,
    pub fill_color: Option<Rgb>,
    pub number_format: NumberFormat,
    pub borders: Borders,
}

impl CellFormat {
    pub fn is_default(self) -> bool {
        self == Self::default()
    }
}

fn format_fixed(value: f64, decimals: u8) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let text = format!("{value:.prec$}", prec = decimals as usize);
    let (integer, fraction) = text.split_once('.').unwrap_or((&text, ""));
    let (sign, digits) = match integer.strip_prefix('-') {
        Some(digits) => ("-", digits),
        None => ("", integer),
    };
    let mut grouped = String::with_capacity(text.len() + text.len() / 3);
    grouped.push_str(sign);
    for (index, ch) in digits.chars().enumerate() {
        if index > 0 && (digits.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(ch);
    }
    if !fraction.is_empty() {
        grouped.push('.');
        grouped.push_str(fraction);
    }
    grouped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn formats_numbers_with_grouping() {
        assert_eq!(
            NumberFormat::Number { decimals: 2 }.apply(1234.5),
            "1,234.50"
        );
        assert_eq!(
            NumberFormat::Number { decimals: 0 }.apply(-1234.0),
            "-1,234"
        );
        assert_eq!(NumberFormat::Number { decimals: 1 }.apply(7.26), "7.3");
    }

    #[test]
    fn formats_currency_and_percent() {
        assert_eq!(
            NumberFormat::Currency { decimals: 2 }.apply(1234.5),
            "$1,234.50"
        );
        assert_eq!(
            NumberFormat::Currency { decimals: 2 }.apply(-20.0),
            "-$20.00"
        );
        assert_eq!(NumberFormat::Percent { decimals: 0 }.apply(0.25), "25%");
        assert_eq!(NumberFormat::Percent { decimals: 1 }.apply(0.125), "12.5%");
    }

    #[test]
    fn builds_border_presets() {
        assert!(
            BorderPreset::None
                .borders(true, true, true, true)
                .is_default()
        );
        assert_eq!(
            BorderPreset::All.borders(false, false, false, false),
            Borders::ALL
        );
        assert_eq!(
            BorderPreset::Outer.borders(true, false, true, false),
            Borders {
                top: true,
                right: false,
                bottom: true,
                left: false,
            }
        );
        assert_eq!(
            BorderPreset::Bottom.borders(false, false, false, false),
            Borders {
                top: false,
                right: false,
                bottom: true,
                left: false,
            }
        );
    }

    #[test]
    fn builds_excel_number_formats() {
        assert_eq!(
            NumberFormat::Number { decimals: 2 }.excel_format(),
            Some("#,##0.00".to_string())
        );
        assert_eq!(
            NumberFormat::Currency { decimals: 0 }.excel_format(),
            Some("$#,##0".to_string())
        );
        assert_eq!(
            NumberFormat::Percent { decimals: 1 }.excel_format(),
            Some("0.0%".to_string())
        );
        assert_eq!(NumberFormat::General.excel_format(), None);
    }
}
