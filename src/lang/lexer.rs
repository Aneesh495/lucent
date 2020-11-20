use super::{Diagnostic, Op, Span};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Number(String),
    Name(String),
    Op(Op),
    Open,
    Close,
    ListOpen,
    ListClose,
    Comma,
    Define,
    End,
}

pub fn lex(source: &str) -> Result<Vec<(Token, Span)>, Diagnostic> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if tokens.len() >= 384 {
            return Err(Diagnostic::at(
                Span {
                    start: i,
                    end: source.len(),
                },
                "Too many tokens",
            ));
        }
        let start = i;
        let c = source[i..].chars().next().unwrap();
        i += c.len_utf8();
        let token = match c {
            c if c.is_whitespace() => continue,
            '0'..='9' | '.' => {
                i = number(bytes, start).map_err(|end| {
                    Diagnostic::at(Span { start, end }, "Malformed decimal literal")
                })?;
                Token::Number(source[start..i].into())
            }
            'a'..='z' | 'A'..='Z' | '_' => {
                while bytes
                    .get(i)
                    .is_some_and(|b| b.is_ascii_alphanumeric() || *b == b'_')
                {
                    i += 1;
                }
                Token::Name(source[start..i].into())
            }
            'π' => Token::Name("pi".into()),
            '(' => Token::Open,
            ')' => Token::Close,
            '[' => Token::ListOpen,
            ']' => Token::ListClose,
            ',' => Token::Comma,
            '+' => Token::Op(Op::Add),
            '-' | '−' => Token::Op(Op::Sub),
            '*' | '·' | '×' => Token::Op(Op::Mul),
            '/' | '÷' => Token::Op(Op::Div),
            '^' => Token::Op(Op::Pow),
            '=' => Token::Op(Op::Eq),
            '≤' => Token::Op(Op::Le),
            '≥' => Token::Op(Op::Ge),
            '≠' => Token::Op(Op::Ne),
            '<' | '>' | '!' | ':' if bytes.get(i) == Some(&b'=') => {
                i += 1;
                match c {
                    '<' => Token::Op(Op::Le),
                    '>' => Token::Op(Op::Ge),
                    '!' => Token::Op(Op::Ne),
                    _ => Token::Define,
                }
            }
            '<' => Token::Op(Op::Lt),
            '>' => Token::Op(Op::Gt),
            _ => {
                return Err(Diagnostic::at(
                    Span { start, end: i },
                    format!("Unexpected character {c:?}"),
                ));
            }
        };
        tokens.push((token, Span { start, end: i }));
    }
    tokens.push((Token::End, Span { start: i, end: i }));
    Ok(tokens)
}

fn number(source: &[u8], start: usize) -> Result<usize, usize> {
    fn digits(source: &[u8], mut i: usize) -> usize {
        while source.get(i).is_some_and(u8::is_ascii_digit) {
            i += 1;
        }
        i
    }

    let mut i = digits(source, start);
    if source.get(i) == Some(&b'.') {
        i = digits(source, i + 1);
    }
    if i == start + 1 && source[start] == b'.' {
        return Err(i);
    }
    if matches!(source.get(i), Some(b'e' | b'E')) {
        let exponent = i + 1 + usize::from(matches!(source.get(i + 1), Some(b'+' | b'-')));
        let end = digits(source, exponent);
        if end > exponent {
            i = end;
        } else if exponent > i + 1 {
            return Err(exponent);
        }
    }
    if source.get(i) == Some(&b'.') {
        return Err(i + 1);
    }
    Ok(i)
}
