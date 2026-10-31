pub fn parse(input: &str) -> Vec<Vec<String>> {
    let mut rows = Vec::new();
    let mut row = Vec::new();
    let mut field = String::new();
    let mut chars = input.chars().peekable();
    let mut in_quotes = false;
    let mut field_started = false;

    while let Some(ch) = chars.next() {
        if in_quotes {
            match ch {
                '"' => {
                    if chars.peek() == Some(&'"') {
                        chars.next();
                        field.push('"');
                    } else {
                        in_quotes = false;
                    }
                }
                other => field.push(other),
            }
            continue;
        }

        match ch {
            '"' if !field_started => {
                in_quotes = true;
                field_started = true;
            }
            ',' => {
                row.push(std::mem::take(&mut field));
                field_started = false;
            }
            '\r' => {
                if chars.peek() == Some(&'\n') {
                    chars.next();
                }
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                field_started = false;
            }
            '\n' => {
                row.push(std::mem::take(&mut field));
                rows.push(std::mem::take(&mut row));
                field_started = false;
            }
            other => {
                field.push(other);
                field_started = true;
            }
        }
    }

    if field_started || !field.is_empty() || !row.is_empty() {
        row.push(field);
    }
    if !row.is_empty() {
        rows.push(row);
    }
    rows
}

pub fn escape_field(field: &str) -> String {
    if field.contains([',', '"', '\n', '\r']) {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

pub fn write(rows: &[Vec<String>]) -> String {
    let mut output = String::new();
    for (index, row) in rows.iter().enumerate() {
        if index > 0 {
            output.push('\n');
        }
        for (column, field) in row.iter().enumerate() {
            if column > 0 {
                output.push(',');
            }
            output.push_str(&escape_field(field));
        }
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_plain_rows() {
        assert_eq!(
            parse("a,b,c\n1,2,3"),
            vec![
                vec!["a".to_string(), "b".to_string(), "c".to_string()],
                vec!["1".to_string(), "2".to_string(), "3".to_string()],
            ]
        );
    }

    #[test]
    fn parses_quoted_fields() {
        assert_eq!(
            parse("\"hello, world\",\"say \"\"hi\"\"\"\n"),
            vec![vec!["hello, world".to_string(), "say \"hi\"".to_string()]]
        );
    }

    #[test]
    fn parses_newlines_inside_quotes() {
        assert_eq!(
            parse("\"line one\nline two\",x"),
            vec![vec!["line one\nline two".to_string(), "x".to_string()]]
        );
    }

    #[test]
    fn handles_crlf_and_empty_fields() {
        assert_eq!(
            parse("a,,c\r\n,,\r\n"),
            vec![
                vec!["a".to_string(), String::new(), "c".to_string()],
                vec![String::new(), String::new(), String::new()],
            ]
        );
    }

    #[test]
    fn escapes_and_round_trips() {
        let rows = vec![
            vec!["plain".to_string(), "with,comma".to_string()],
            vec!["with\nnewline".to_string(), "with \"quote\"".to_string()],
        ];
        let encoded = write(&rows);
        assert_eq!(parse(&encoded), rows);
    }

    #[test]
    fn empty_input_is_empty() {
        assert!(parse("").is_empty());
    }
}
