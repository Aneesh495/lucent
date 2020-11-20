use super::{
    Diagnostic, Expr, Kind, Op, Span, Statement,
    lexer::{Token, lex},
};

pub fn parse(source: &str) -> Result<Statement, Diagnostic> {
    if source.len() > 8192 {
        return Err(Diagnostic::at(
            Span {
                start: 0,
                end: source.len(),
            },
            "Expression exceeds 8192 bytes",
        ));
    }
    let mut parser = Parser {
        tokens: lex(source)?,
        at: 0,
    };
    let first = parser.expression(0, 0)?;
    let statement = if parser.eat(&Token::Define).is_some() {
        let span = first.span;
        let (name, params) = match first.kind {
            Kind::Name(name) => (name, None),
            Kind::Call(name, args) => {
                let mut names = Vec::new();
                for arg in args {
                    let Kind::Name(param) = arg.kind else {
                        return Err(Diagnostic::at(arg.span, "A parameter must be a name"));
                    };
                    if names.contains(&param) {
                        return Err(Diagnostic::at(arg.span, "Duplicate parameter"));
                    }
                    names.push(param);
                }
                (name, Some(names))
            }
            _ => {
                return Err(Diagnostic::at(
                    span,
                    "Expected a name or function signature before :=",
                ));
            }
        };
        let body = parser.expression(0, 0)?;
        check_depth(&body, 0)?;
        Statement::Definition {
            name,
            params,
            span: span.through(body.span),
            body,
        }
    } else {
        check_depth(&first, 0)?;
        Statement::Expression(first)
    };
    parser.require(&Token::End, "Unexpected trailing input")?;
    Ok(statement)
}

struct Parser {
    tokens: Vec<(Token, Span)>,
    at: usize,
}

impl Parser {
    fn peek(&self) -> &(Token, Span) {
        &self.tokens[self.at]
    }

    fn next(&mut self) -> (Token, Span) {
        let value = self.peek().clone();
        if value.0 != Token::End {
            self.at += 1;
        }
        value
    }

    fn eat(&mut self, token: &Token) -> Option<Span> {
        if &self.peek().0 == token {
            Some(self.next().1)
        } else {
            None
        }
    }

    fn require(&mut self, token: &Token, message: &str) -> Result<Span, Diagnostic> {
        self.eat(token)
            .ok_or_else(|| Diagnostic::at(self.peek().1, message))
    }

    fn expression(&mut self, min: u8, depth: usize) -> Result<Expr, Diagnostic> {
        if depth > 48 {
            return Err(Diagnostic::at(
                self.peek().1,
                "Expression nesting exceeds 48",
            ));
        }
        let (token, span) = self.next();
        let mut lhs = match token {
            Token::Number(text) => Expr::literal(text, span),
            Token::Name(name) if self.eat(&Token::Open).is_some() => {
                let (args, end) = self.arguments(Token::Close, depth)?;
                Expr::new(Kind::Call(name, args), span.through(end))
            }
            Token::Name(name) => Expr::new(Kind::Name(name), span),
            Token::Op(Op::Sub) => {
                let inner = self.expression(7, depth + 1)?;
                let span = span.through(inner.span);
                Expr::negated(inner, span)
            }
            Token::Op(Op::Add) => self.expression(7, depth + 1)?,
            Token::Open => {
                let mut inner = self.expression(0, depth + 1)?;
                let end = self.require(&Token::Close, "Expected )")?;
                inner.span = span.through(end);
                inner
            }
            Token::ListOpen => {
                let (items, end) = self.arguments(Token::ListClose, depth)?;
                Expr::new(Kind::List(items), span.through(end))
            }
            _ => return Err(Diagnostic::at(span, "Expected an expression")),
        };
        loop {
            let (op, implicit) = match self.peek().0 {
                Token::Op(op) => (op, false),
                Token::Name(_) | Token::Open => (Op::Mul, true),
                _ => break,
            };
            let (left, right) = match op {
                Op::Add | Op::Sub => (3, 4),
                Op::Mul | Op::Div => (5, 6),
                Op::Pow => (8, 8),
                _ => (1, 2),
            };
            if left < min {
                break;
            }
            if op.compares() && matches!(lhs.kind, Kind::Binary(a, ..) if a.compares()) {
                return Err(Diagnostic::at(
                    self.peek().1,
                    "Chained comparisons are unsupported",
                ));
            }
            if !implicit {
                self.next();
            }
            let rhs = self.expression(right, depth + 1)?;
            let span = lhs.span.through(rhs.span);
            lhs = Expr::binary(op, lhs, rhs, span);
        }
        Ok(lhs)
    }

    fn arguments(&mut self, close: Token, depth: usize) -> Result<(Vec<Expr>, Span), Diagnostic> {
        let mut values = Vec::new();
        if let Some(end) = self.eat(&close) {
            return Ok((values, end));
        }
        loop {
            values.push(self.expression(0, depth + 1)?);
            if let Some(end) = self.eat(&close) {
                return Ok((values, end));
            }
            self.require(&Token::Comma, "Expected a comma or closing delimiter")?;
        }
    }
}

fn check_depth(expr: &Expr, depth: usize) -> Result<(), Diagnostic> {
    if depth > 48 {
        return Err(Diagnostic::at(
            expr.span,
            "Expression tree exceeds depth 48",
        ));
    }
    match &expr.kind {
        Kind::Binary(_, a, b) => {
            check_depth(a, depth + 1)?;
            check_depth(b, depth + 1)?;
        }
        Kind::Negative(a) => check_depth(a, depth + 1)?,
        Kind::Call(_, args) | Kind::List(args) => {
            for arg in args {
                check_depth(arg, depth + 1)?;
            }
        }
        _ => {}
    }
    Ok(())
}
