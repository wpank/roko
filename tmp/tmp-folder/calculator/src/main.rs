use std::io::{self, Write};

fn main() {
    if let Err(err) = run() {
        eprintln!("error: {err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    println!("CLI calculator");
    println!("Enter expressions like `1 + 2`, `3 * (4 - 1)`, or `quit` to exit.");

    let stdin = io::stdin();
    let mut input = String::new();

    loop {
        print!("> ");
        io::stdout().flush().map_err(|e| e.to_string())?;
        input.clear();

        if stdin.read_line(&mut input).map_err(|e| e.to_string())? == 0 {
            break;
        }

        let line = input.trim();
        if line.is_empty() {
            continue;
        }
        if matches!(line, "quit" | "exit") {
            break;
        }

        match evaluate(line) {
            Ok(value) => println!("{value}"),
            Err(err) => println!("error: {err}"),
        }
    }

    Ok(())
}

fn evaluate(input: &str) -> Result<f64, String> {
    let mut parser = Parser::new(input);
    let value = parser.parse_expression()?;
    parser.skip_ws();
    if parser.is_eof() {
        Ok(value)
    } else {
        Err(format!("unexpected character '{}'", parser.peek_char().unwrap_or('\0')))
    }
}

struct Parser<'a> {
    chars: std::str::Chars<'a>,
    lookahead: Option<char>,
}

impl<'a> Parser<'a> {
    fn new(input: &'a str) -> Self {
        let mut chars = input.chars();
        let lookahead = chars.next();
        Self { chars, lookahead }
    }

    fn is_eof(&self) -> bool {
        self.lookahead.is_none()
    }

    fn peek_char(&self) -> Option<char> {
        self.lookahead
    }

    fn bump(&mut self) -> Option<char> {
        let current = self.lookahead;
        self.lookahead = self.chars.next();
        current
    }

    fn skip_ws(&mut self) {
        while matches!(self.lookahead, Some(c) if c.is_whitespace()) {
            self.bump();
        }
    }

    fn parse_expression(&mut self) -> Result<f64, String> {
        let mut value = self.parse_term()?;
        loop {
            self.skip_ws();
            match self.peek_char() {
                Some('+') => {
                    self.bump();
                    value += self.parse_term()?;
                }
                Some('-') => {
                    self.bump();
                    value -= self.parse_term()?;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_term(&mut self) -> Result<f64, String> {
        let mut value = self.parse_factor()?;
        loop {
            self.skip_ws();
            match self.peek_char() {
                Some('*') => {
                    self.bump();
                    value *= self.parse_factor()?;
                }
                Some('/') => {
                    self.bump();
                    let rhs = self.parse_factor()?;
                    value /= rhs;
                }
                _ => return Ok(value),
            }
        }
    }

    fn parse_factor(&mut self) -> Result<f64, String> {
        self.skip_ws();
        match self.peek_char() {
            Some('+') => {
                self.bump();
                self.parse_factor()
            }
            Some('-') => {
                self.bump();
                Ok(-self.parse_factor()?)
            }
            Some('(') => {
                self.bump();
                let value = self.parse_expression()?;
                self.skip_ws();
                match self.bump() {
                    Some(')') => Ok(value),
                    _ => Err("expected ')'".to_string()),
                }
            }
            Some(c) if c.is_ascii_digit() || c == '.' => self.parse_number(),
            Some(c) => Err(format!("unexpected character '{c}'")),
            None => Err("unexpected end of input".to_string()),
        }
    }

    fn parse_number(&mut self) -> Result<f64, String> {
        self.skip_ws();
        let mut buf = String::new();
        let mut seen_digit = false;
        let mut seen_dot = false;
        while let Some(c) = self.peek_char() {
            if c.is_ascii_digit() {
                seen_digit = true;
                buf.push(c);
                self.bump();
            } else if c == '.' && !seen_dot {
                seen_dot = true;
                buf.push(c);
                self.bump();
            } else {
                break;
            }
        }
        if !seen_digit {
            return Err("expected number".to_string());
        }
        buf.parse::<f64>().map_err(|_| format!("invalid number '{buf}'"))
    }
}

#[cfg(test)]
mod tests {
    use super::evaluate;

    #[test]
    fn respects_operator_precedence() {
        assert_eq!(evaluate("1 + 2 * 3").unwrap(), 7.0);
    }

    #[test]
    fn handles_parentheses_and_unary_minus() {
        assert_eq!(evaluate("-(3 + 2) * 4").unwrap(), -20.0);
    }

    #[test]
    fn rejects_trailing_input() {
        assert!(evaluate("1 + 2 foo").is_err());
    }
}
