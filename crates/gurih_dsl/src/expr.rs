use crate::diagnostics::SourceSpan;
use crate::errors::CompileError;

#[derive(Debug, Clone)]
pub enum Expr {
    Field(String, SourceSpan),
    Literal(f64, SourceSpan),
    StringLiteral(String, SourceSpan),
    BoolLiteral(bool, SourceSpan),
    FunctionCall {
        name: String,
        args: Vec<Expr>,
        span: SourceSpan,
    },
    BinaryOp {
        left: Box<Expr>,
        op: BinaryOpType,
        right: Box<Expr>,
        span: SourceSpan,
    },
    UnaryOp {
        op: UnaryOpType,
        expr: Box<Expr>,
        span: SourceSpan,
    },
    Grouping(Box<Expr>, SourceSpan),
}

#[derive(Debug, Clone)]
pub enum BinaryOpType {
    Add,
    Sub,
    Mul,
    Div,
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
}

#[derive(Debug, Clone)]
pub enum UnaryOpType {
    Not,
    Neg,
}

impl Expr {
    pub fn span(&self) -> SourceSpan {
        match self {
            Expr::Field(_, s) => *s,
            Expr::Literal(_, s) => *s,
            Expr::StringLiteral(_, s) => *s,
            Expr::BoolLiteral(_, s) => *s,
            Expr::FunctionCall { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
            Expr::UnaryOp { span, .. } => *span,
            Expr::Grouping(_, s) => *s,
        }
    }
}

pub fn parse_expression(src: &str, start_offset: usize) -> Result<Expr, CompileError> {
    let tokens = tokenize(src, start_offset)?;
    let mut parser = Parser::new(tokens, src);
    parser.parse()
}

#[derive(Debug, Clone)]
struct Token {
    kind: TokenKind,
    span: SourceSpan,
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Identifier(String),
    Field(String),
    Number(f64),
    String(String),
    True,
    False,
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
    Dot,
    EqEq,
    BangEq,
    Gt,
    Lt,
    GtEq,
    LtEq,
    And,
    Or,
    Bang,
    Eof,
}

fn tokenize(src: &str, start_offset: usize) -> Result<Vec<Token>, CompileError> {
    let mut tokens = vec![];
    let mut chars = src.chars().peekable();
    let mut current_pos = 0; // relative to src start

    while let Some(&c) = chars.peek() {
        let abs_start = start_offset + current_pos;
        match c {
            ' ' | '\t' | '\n' | '\r' => {
                chars.next();
                current_pos += c.len_utf8();
            }
            '+' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Plus,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '-' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Minus,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '*' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Star,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '/' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Slash,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '(' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::LParen,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            ')' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::RParen,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            ',' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Comma,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '.' => {
                chars.next();
                tokens.push(Token {
                    kind: TokenKind::Dot,
                    span: (abs_start, 1).into(),
                });
                current_pos += 1;
            }
            '!' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::BangEq,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Bang,
                        span: (abs_start, 1).into(),
                    });
                    current_pos += 1;
                }
            }
            '=' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::EqEq,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, 1).into(),
                        message: "Unexpected '=', did you mean '=='?".to_string(),
                    });
                }
            }
            '>' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::GtEq,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Gt,
                        span: (abs_start, 1).into(),
                    });
                    current_pos += 1;
                }
            }
            '<' => {
                chars.next();
                if let Some(&'=') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::LtEq,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    tokens.push(Token {
                        kind: TokenKind::Lt,
                        span: (abs_start, 1).into(),
                    });
                    current_pos += 1;
                }
            }
            '&' => {
                chars.next();
                if let Some(&'&') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::And,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, 1).into(),
                        message: "Unexpected character '&', did you mean '&&'?".to_string(),
                    });
                }
            }
            '|' => {
                chars.next();
                if let Some(&'|') = chars.peek() {
                    chars.next();
                    tokens.push(Token {
                        kind: TokenKind::Or,
                        span: (abs_start, 2).into(),
                    });
                    current_pos += 2;
                } else {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, 1).into(),
                        message: "Unexpected character '|', did you mean '||'?".to_string(),
                    });
                }
            }
            '[' => {
                chars.next(); // eat [
                let mut content = String::new();
                let mut len = 1;
                let mut closed = false;
                for ch in chars.by_ref() {
                    len += ch.len_utf8();
                    if ch == ']' {
                        closed = true;
                        break;
                    }
                    content.push(ch);
                }

                if !closed {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, len).into(),
                        message: "Unclosed field bracket".to_string(),
                    });
                }

                tokens.push(Token {
                    kind: TokenKind::Field(content),
                    span: (abs_start, len).into(),
                });
                current_pos += len;
            }
            '\'' => {
                chars.next(); // eat '
                let mut content = String::new();
                let mut len = 1;
                let mut closed = false;
                for ch in chars.by_ref() {
                    len += ch.len_utf8();
                    if ch == '\'' {
                        closed = true;
                        break;
                    }
                    content.push(ch);
                }
                if !closed {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, len).into(),
                        message: "Unclosed string literal".to_string(),
                    });
                }
                tokens.push(Token {
                    kind: TokenKind::String(content),
                    span: (abs_start, len).into(),
                });
                current_pos += len;
            }
            '"' => {
                chars.next(); // eat "
                let mut content = String::new();
                let mut len = 1;
                let mut closed = false;
                for ch in chars.by_ref() {
                    len += ch.len_utf8();
                    if ch == '"' {
                        closed = true;
                        break;
                    }
                    content.push(ch);
                }
                if !closed {
                    return Err(CompileError::ParseError {
                        src: src.to_string(),
                        span: (abs_start, len).into(),
                        message: "Unclosed string literal".to_string(),
                    });
                }
                tokens.push(Token {
                    kind: TokenKind::String(content),
                    span: (abs_start, len).into(),
                });
                current_pos += len;
            }
            _ if c.is_ascii_digit() => {
                let mut content = String::new();
                let mut len = 0;
                while let Some(&ch) = chars.peek() {
                    if ch.is_ascii_digit() || ch == '.' {
                        chars.next();
                        content.push(ch);
                        len += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let val: f64 = content.parse().map_err(|_| CompileError::ParseError {
                    src: src.to_string(),
                    span: (abs_start, len).into(),
                    message: "Invalid number".to_string(),
                })?;
                tokens.push(Token {
                    kind: TokenKind::Number(val),
                    span: (abs_start, len).into(),
                });
                current_pos += len;
            }
            _ if c.is_alphabetic() || c == '_' => {
                let mut content = String::new();
                let mut len = 0;
                while let Some(&ch) = chars.peek() {
                    if ch.is_alphanumeric() || ch == '_' {
                        chars.next();
                        content.push(ch);
                        len += ch.len_utf8();
                    } else {
                        break;
                    }
                }
                let kind = match content.as_str() {
                    "true" => TokenKind::True,
                    "false" => TokenKind::False,
                    _ => TokenKind::Identifier(content),
                };
                tokens.push(Token {
                    kind,
                    span: (abs_start, len).into(),
                });
                current_pos += len;
            }
            _ => {
                return Err(CompileError::ParseError {
                    src: src.to_string(),
                    span: (abs_start, 1).into(),
                    message: format!("Unexpected character: {}", c),
                });
            }
        }
    }

    tokens.push(Token {
        kind: TokenKind::Eof,
        span: (start_offset + current_pos, 0).into(),
    });

    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    current: usize,
    src: &'a str,
}

impl<'a> Parser<'a> {
    fn new(tokens: Vec<Token>, src: &'a str) -> Self {
        Self {
            tokens,
            current: 0,
            src,
        }
    }

    fn parse(&mut self) -> Result<Expr, CompileError> {
        self.expression()
    }

    // expression -> logic_or
    fn expression(&mut self) -> Result<Expr, CompileError> {
        self.logic_or()
    }

    // logic_or -> logic_and ( "||" logic_and )*
    fn logic_or(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.logic_and()?;

        while self.match_token(&[TokenKind::Or]) {
            let right = self.logic_and()?;
            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op: BinaryOpType::Or,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    // logic_and -> equality ( "&&" equality )*
    fn logic_and(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.equality()?;

        while self.match_token(&[TokenKind::And]) {
            let right = self.equality()?;
            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op: BinaryOpType::And,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    // equality -> comparison ( ( "!=" | "==" ) comparison )*
    fn equality(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.comparison()?;

        while self.match_token(&[TokenKind::BangEq, TokenKind::EqEq]) {
            let op_token = self.previous().clone();
            let op = match op_token.kind {
                TokenKind::BangEq => BinaryOpType::Neq,
                TokenKind::EqEq => BinaryOpType::Eq,
                _ => unreachable!(),
            };
            let right = self.comparison()?;
            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    // comparison -> term ( ( ">" | ">=" | "<" | "<=" ) term )*
    fn comparison(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.term()?;

        while self.match_token(&[TokenKind::Gt, TokenKind::GtEq, TokenKind::Lt, TokenKind::LtEq]) {
            let op_token = self.previous().clone();
            let op = match op_token.kind {
                TokenKind::Gt => BinaryOpType::Gt,
                TokenKind::GtEq => BinaryOpType::Gte,
                TokenKind::Lt => BinaryOpType::Lt,
                TokenKind::LtEq => BinaryOpType::Lte,
                _ => unreachable!(),
            };
            let right = self.term()?;
            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }
        Ok(expr)
    }

    // term -> factor ( ( "-" | "+" ) factor )*
    fn term(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.factor()?;

        while self.match_token(&[TokenKind::Plus, TokenKind::Minus]) {
            let op_token = self.previous().clone();
            let op = match op_token.kind {
                TokenKind::Plus => BinaryOpType::Add,
                TokenKind::Minus => BinaryOpType::Sub,
                _ => unreachable!(),
            };
            let right = self.factor()?;

            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    // factor -> unary ( ( "/" | "*" ) unary )*
    fn factor(&mut self) -> Result<Expr, CompileError> {
        let mut expr = self.unary()?;

        while self.match_token(&[TokenKind::Slash, TokenKind::Star]) {
            let op_token = self.previous().clone();
            let op = match op_token.kind {
                TokenKind::Star => BinaryOpType::Mul,
                TokenKind::Slash => BinaryOpType::Div,
                _ => unreachable!(),
            };
            let right = self.unary()?;

            let start = expr.span().offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            expr = Expr::BinaryOp {
                left: Box::new(expr),
                op,
                right: Box::new(right),
                span,
            };
        }

        Ok(expr)
    }

    // unary -> ( "!" | "-" ) unary | call
    fn unary(&mut self) -> Result<Expr, CompileError> {
        if self.match_token(&[TokenKind::Bang, TokenKind::Minus]) {
            let op_token = self.previous().clone();
            let op = match op_token.kind {
                TokenKind::Bang => UnaryOpType::Not,
                TokenKind::Minus => UnaryOpType::Neg,
                _ => unreachable!(),
            };
            let right = self.unary()?;
            let start = op_token.span.offset();
            let end = right.span().offset() + right.span().len();
            let span = (start, end - start).into();

            return Ok(Expr::UnaryOp {
                op,
                expr: Box::new(right),
                span,
            });
        }
        self.call()
    }

    // call -> primary ( "(" arguments? ")" | "." identifier )*
    // Handled in primary currently for simplicity but better here if we want chain
    // For now, let's keep it simple: primary handles function call, but we need to handle dot access for fields like `period.is_closed`
    fn call(&mut self) -> Result<Expr, CompileError> {
        // We parse a primary, then check for dots
        let mut expr = self.primary()?;

        loop {
            if self.match_token(&[TokenKind::Dot]) {
                // expecting identifier
                if self.check_kind(&TokenKind::Identifier("".to_string())) {
                    // Actually, check_kind takes a reference, and Identifier carries data.
                    // We need to verify if next is Identifier.
                    let next = self.peek().clone(); // Clone to avoid borrow issue
                    if let TokenKind::Identifier(name) = next.kind {
                        let name = name.clone();
                        let span_end = next.span.offset() + next.span.len();
                        self.advance();

                        // If previous expr was a Field or Identifier, we merge them.
                        // `period` -> `period.is_closed`
                        // If it was function call `SUM(x).y`? That's valid too.
                        // But for now we only support field path strings in `Field`.
                        // Wait, `gurih_ir::Expression::Field` is just a string.
                        // So we should construct the string "a.b".

                        match expr {
                            Expr::Field(prev_name, prev_span) => {
                                let new_name = format!("{}.{}", prev_name, name);
                                let start = prev_span.offset();
                                let span = (start, span_end - start).into();
                                expr = Expr::Field(new_name, span);
                            }
                            Expr::FunctionCall { .. } => {
                                // Not supported by IR currently (FunctionCall return object?)
                                // But let's allow it in AST?
                                // No, the IR expects Field to be a symbol.
                                return Err(CompileError::ParseError {
                                    src: self.src.to_string(),
                                    span: next.span,
                                    message:
                                        "Chained field access on function call result not supported in this version."
                                            .to_string(),
                                });
                            }
                            _ => {
                                return Err(CompileError::ParseError {
                                    src: self.src.to_string(),
                                    span: next.span,
                                    message: "Dot access only allowed on fields/identifiers.".to_string(),
                                });
                            }
                        }
                    } else {
                        return Err(CompileError::ParseError {
                            src: self.src.to_string(),
                            span: next.span,
                            message: "Expect identifier after '.'".to_string(),
                        });
                    }
                } else {
                    return Err(CompileError::ParseError {
                        src: self.src.to_string(),
                        span: self.peek().span,
                        message: "Expect identifier after '.'".to_string(),
                    });
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        if self.is_at_end() {
            return Err(CompileError::ParseError {
                src: self.src.to_string(),
                span: (self.tokens.last().unwrap().span),
                message: "Unexpected end of expression".to_string(),
            });
        }

        let token = self.peek().clone();
        match token.kind {
            TokenKind::True => {
                self.advance();
                Ok(Expr::BoolLiteral(true, token.span))
            }
            TokenKind::False => {
                self.advance();
                Ok(Expr::BoolLiteral(false, token.span))
            }
            TokenKind::Number(n) => {
                self.advance();
                Ok(Expr::Literal(n, token.span))
            }
            TokenKind::String(s) => {
                self.advance();
                Ok(Expr::StringLiteral(s, token.span))
            }
            TokenKind::Field(f) => {
                self.advance();
                Ok(Expr::Field(f, token.span))
            }
            TokenKind::Identifier(name) => {
                self.advance();
                // Check if it's a function call
                if self.check(TokenKind::LParen) {
                    self.advance(); // eat (
                    let mut args = vec![];
                    if !self.check(TokenKind::RParen) {
                        loop {
                            args.push(self.expression()?);
                            if !self.match_token(&[TokenKind::Comma]) {
                                break;
                            }
                        }
                    }

                    if !self.match_token(&[TokenKind::RParen]) {
                        return Err(CompileError::ParseError {
                            src: self.src.to_string(),
                            span: self.peek().span,
                            message: "Expect ')' after function arguments.".to_string(),
                        });
                    }
                    let span = (
                        token.span.offset(),
                        self.previous().span.offset() + self.previous().span.len() - token.span.offset(),
                    )
                        .into(); // Basic span union
                    Ok(Expr::FunctionCall { name, args, span })
                } else {
                    // Just an identifier, treat as field
                    Ok(Expr::Field(name, token.span))
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression()?;
                if !self.match_token(&[TokenKind::RParen]) {
                    return Err(CompileError::ParseError {
                        src: self.src.to_string(),
                        span: self.peek().span,
                        message: "Expect ')' after expression.".to_string(),
                    });
                }
                let span = (
                    token.span.offset(),
                    self.previous().span.offset() + self.previous().span.len() - token.span.offset(),
                )
                    .into();
                Ok(Expr::Grouping(Box::new(expr), span))
            }
            _ => Err(CompileError::ParseError {
                src: self.src.to_string(),
                span: token.span,
                message: format!("Expect expression, found {:?}", token.kind),
            }),
        }
    }

    // Helpers
    fn match_token(&mut self, types: &[TokenKind]) -> bool {
        for t in types {
            if self.check_kind(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn check_kind(&self, kind: &TokenKind) -> bool {
        if self.is_at_end() {
            return false;
        }
        let next = &self.peek().kind;
        match (next, kind) {
            (TokenKind::Identifier(_), TokenKind::Identifier(_)) => true, // Treat all identifiers as same kind for check
            (k1, k2) => std::mem::discriminant(k1) == std::mem::discriminant(k2),
        }
    }

    fn check(&self, kind: TokenKind) -> bool {
        self.check_kind(&kind)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().kind, TokenKind::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}
