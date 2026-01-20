use crate::diagnostics::SourceSpan;
use crate::errors::CompileError;

#[derive(Debug, Clone)]
pub enum Expr {
    Field(String, SourceSpan),
    Literal(f64, SourceSpan),
    StringLiteral(String, SourceSpan),
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
    Grouping(Box<Expr>, SourceSpan),
}

#[derive(Debug, Clone)]
pub enum BinaryOpType {
    Add,
    Sub,
    Mul,
    Div,
}

impl Expr {
    pub fn span(&self) -> SourceSpan {
        match self {
            Expr::Field(_, s) => *s,
            Expr::Literal(_, s) => *s,
            Expr::StringLiteral(_, s) => *s,
            Expr::FunctionCall { span, .. } => *span,
            Expr::BinaryOp { span, .. } => *span,
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
    Plus,
    Minus,
    Star,
    Slash,
    LParen,
    RParen,
    Comma,
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
            '[' => {
                chars.next(); // eat [
                let mut content = String::new();
                let mut len = 1;
                let mut closed = false;
                while let Some(ch) = chars.next() {
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
            '"' => {
                chars.next(); // eat "
                let mut content = String::new();
                let mut len = 1;
                let mut closed = false;
                while let Some(ch) = chars.next() {
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
            _ if c.is_digit(10) => {
                let mut content = String::new();
                let mut len = 0;
                while let Some(&ch) = chars.peek() {
                    if ch.is_digit(10) || ch == '.' {
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
            _ if c.is_alphabetic() => {
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
                tokens.push(Token {
                    kind: TokenKind::Identifier(content),
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

    fn expression(&mut self) -> Result<Expr, CompileError> {
        self.term()
    }

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

    fn unary(&mut self) -> Result<Expr, CompileError> {
        self.primary()
    }

    fn primary(&mut self) -> Result<Expr, CompileError> {
        // Safe peeking
        if self.is_at_end() {
            return Err(CompileError::ParseError {
                src: self.src.to_string(),
                span: (self.tokens.last().unwrap().span).into(),
                message: "Unexpected end of expression".to_string(),
            });
        }

        let token = self.peek().clone();
        match token.kind {
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
                            span: self.peek().span.into(),
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
                    Err(CompileError::ParseError {
                        src: self.src.to_string(),
                        span: token.span.into(),
                        message: format!("Unexpected identifier '{}'. Fields use []. Functions use NAME().", name),
                    })
                }
            }
            TokenKind::LParen => {
                self.advance();
                let expr = self.expression()?;
                if !self.match_token(&[TokenKind::RParen]) {
                    return Err(CompileError::ParseError {
                        src: self.src.to_string(),
                        span: self.peek().span.into(),
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
                span: token.span.into(),
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
        match (&self.peek().kind, kind) {
            (TokenKind::Plus, TokenKind::Plus) => true,
            (TokenKind::Minus, TokenKind::Minus) => true,
            (TokenKind::Star, TokenKind::Star) => true,
            (TokenKind::Slash, TokenKind::Slash) => true,
            (TokenKind::LParen, TokenKind::LParen) => true,
            (TokenKind::RParen, TokenKind::RParen) => true,
            (TokenKind::Comma, TokenKind::Comma) => true,

            _ => false,
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
        match self.peek().kind {
            TokenKind::Eof => true,
            _ => false,
        }
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }
}
