use crate::ast::{BinOp, Expr, Stmt, UnaryOp};

#[derive(Clone, Debug, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub line: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum TokenKind {
    Ident(String),
    Number(f64),
    String(String),
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrace,
    RBrace,
    Comma,
    Dot,
    Semi,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    EqEq,
    NotEq,
    EqEqEq,
    NotEqEq,
    Lt,
    Gt,
    Le,
    Ge,
    AndAnd,
    OrOr,
    Bang,
    Question,
    Colon,
    Eof,
}

pub fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let mut tokens = Vec::new();
    let bytes = source.as_bytes();
    let mut index = 0;
    let mut line = 1u32;
    while index < bytes.len() {
        let ch = bytes[index] as char;
        if ch == '\n' {
            line += 1;
            index += 1;
            continue;
        }
        if ch.is_whitespace() {
            index += 1;
            continue;
        }
        if ch == '/' && bytes.get(index + 1) == Some(&b'/') {
            index += 2;
            while index < bytes.len() && bytes[index] != b'\n' {
                index += 1;
            }
            continue;
        }
        if ch == '/' && bytes.get(index + 1) == Some(&b'*') {
            index += 2;
            while index + 1 < bytes.len() && !(bytes[index] == b'*' && bytes[index + 1] == b'/') {
                if bytes[index] == b'\n' {
                    line += 1;
                }
                index += 1;
            }
            index = (index + 2).min(bytes.len());
            continue;
        }
        let start_line = line;
        let mut push = |kind, index: &mut usize| {
            tokens.push(Token {
                kind,
                line: start_line,
            });
            *index += 1;
        };
        match ch {
            '(' => push(TokenKind::LParen, &mut index),
            ')' => push(TokenKind::RParen, &mut index),
            '[' => push(TokenKind::LBrack, &mut index),
            ']' => push(TokenKind::RBrack, &mut index),
            '{' => push(TokenKind::LBrace, &mut index),
            '}' => push(TokenKind::RBrace, &mut index),
            ',' => push(TokenKind::Comma, &mut index),
            ';' => push(TokenKind::Semi, &mut index),
            '?' => push(TokenKind::Question, &mut index),
            ':' => push(TokenKind::Colon, &mut index),
            '.' => {
                if bytes.get(index + 1).is_some_and(|b| b.is_ascii_digit()) {
                    let (number, next) = scan_number(source, index)?;
                    tokens.push(Token {
                        kind: TokenKind::Number(number),
                        line: start_line,
                    });
                    index = next;
                } else {
                    push(TokenKind::Dot, &mut index);
                }
            }
            '+' => push(TokenKind::Plus, &mut index),
            '-' => push(TokenKind::Minus, &mut index),
            '*' => push(TokenKind::Star, &mut index),
            '/' => push(TokenKind::Slash, &mut index),
            '%' => push(TokenKind::Percent, &mut index),
            '"' | '\'' => {
                let (text, next, next_line) = scan_string(source, index, line)?;
                tokens.push(Token {
                    kind: TokenKind::String(text),
                    line: start_line,
                });
                index = next;
                line = next_line;
            }
            '!' => {
                if bytes.get(index + 1) == Some(&b'=') {
                    if bytes.get(index + 2) == Some(&b'=') {
                        tokens.push(Token {
                            kind: TokenKind::NotEqEq,
                            line: start_line,
                        });
                        index += 3;
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::NotEq,
                            line: start_line,
                        });
                        index += 2;
                    }
                } else {
                    push(TokenKind::Bang, &mut index);
                }
            }
            '=' => {
                if bytes.get(index + 1) == Some(&b'=') {
                    if bytes.get(index + 2) == Some(&b'=') {
                        tokens.push(Token {
                            kind: TokenKind::EqEqEq,
                            line: start_line,
                        });
                        index += 3;
                    } else {
                        tokens.push(Token {
                            kind: TokenKind::EqEq,
                            line: start_line,
                        });
                        index += 2;
                    }
                } else {
                    push(TokenKind::Eq, &mut index);
                }
            }
            '<' => {
                if bytes.get(index + 1) == Some(&b'=') {
                    tokens.push(Token {
                        kind: TokenKind::Le,
                        line: start_line,
                    });
                    index += 2;
                } else {
                    push(TokenKind::Lt, &mut index);
                }
            }
            '>' => {
                if bytes.get(index + 1) == Some(&b'=') {
                    tokens.push(Token {
                        kind: TokenKind::Ge,
                        line: start_line,
                    });
                    index += 2;
                } else {
                    push(TokenKind::Gt, &mut index);
                }
            }
            '&' => {
                if bytes.get(index + 1) == Some(&b'&') {
                    tokens.push(Token {
                        kind: TokenKind::AndAnd,
                        line: start_line,
                    });
                    index += 2;
                } else {
                    return Err("unexpected &".into());
                }
            }
            '|' => {
                if bytes.get(index + 1) == Some(&b'|') {
                    tokens.push(Token {
                        kind: TokenKind::OrOr,
                        line: start_line,
                    });
                    index += 2;
                } else {
                    return Err("unexpected |".into());
                }
            }
            _ if ch.is_ascii_digit() => {
                let (number, next) = scan_number(source, index)?;
                tokens.push(Token {
                    kind: TokenKind::Number(number),
                    line: start_line,
                });
                index = next;
            }
            _ if is_ident_start(ch) => {
                let start = index;
                index += 1;
                while index < bytes.len() && is_ident_continue(bytes[index] as char) {
                    index += 1;
                }
                tokens.push(Token {
                    kind: TokenKind::Ident(source[start..index].to_string()),
                    line: start_line,
                });
            }
            _ => return Err(format!("unexpected character {ch}")),
        }
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        line,
    });
    Ok(tokens)
}

fn is_ident_start(ch: char) -> bool {
    ch.is_ascii_alphabetic() || ch == '_' || ch == '$'
}

fn is_ident_continue(ch: char) -> bool {
    ch.is_ascii_alphanumeric() || ch == '_' || ch == '$'
}

fn scan_number(source: &str, start: usize) -> Result<(f64, usize), String> {
    let bytes = source.as_bytes();
    let mut index = start;
    while index < bytes.len() && bytes[index].is_ascii_digit() {
        index += 1;
    }
    if index < bytes.len() && bytes[index] == b'.' {
        index += 1;
        while index < bytes.len() && bytes[index].is_ascii_digit() {
            index += 1;
        }
    }
    let text = &source[start..index];
    let number = text
        .parse::<f64>()
        .map_err(|_| format!("bad number {text}"))?;
    Ok((number, index))
}

fn scan_string(source: &str, start: usize, mut line: u32) -> Result<(String, usize, u32), String> {
    let bytes = source.as_bytes();
    let quote = bytes[start];
    let mut index = start + 1;
    let mut out = String::new();
    while index < bytes.len() {
        let ch = bytes[index];
        if ch == quote {
            return Ok((out, index + 1, line));
        }
        if ch == b'\n' {
            line += 1;
        }
        if ch == b'\\' {
            index += 1;
            if index >= bytes.len() {
                break;
            }
            let escaped = match bytes[index] {
                b'n' => '\n',
                b't' => '\t',
                b'\\' => '\\',
                b'\'' => '\'',
                b'"' => '"',
                other => other as char,
            };
            out.push(escaped);
            index += 1;
            continue;
        }
        out.push(ch as char);
        index += 1;
    }
    Err("unterminated string".into())
}

struct Parser {
    tokens: Vec<Token>,
    index: usize,
}

impl Parser {
    fn peek(&self) -> &Token {
        &self.tokens[self.index]
    }

    fn bump(&mut self) -> Token {
        let token = self.tokens[self.index].clone();
        if self.index + 1 < self.tokens.len() {
            self.index += 1;
        }
        token
    }

    fn eat(&mut self, kind: &TokenKind) -> bool {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(kind) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<(), String> {
        if std::mem::discriminant(&self.peek().kind) == std::mem::discriminant(&kind) {
            self.bump();
            Ok(())
        } else {
            Err(format!("expected {kind:?}, found {:?}", self.peek().kind))
        }
    }

    fn ident(&mut self) -> Result<String, String> {
        match &self.peek().kind {
            TokenKind::Ident(name) => {
                let name = name.clone();
                self.bump();
                Ok(name)
            }
            other => Err(format!("expected name, found {other:?}")),
        }
    }
}

pub fn parse_program(source: &str) -> Result<Vec<Stmt>, String> {
    let tokens = tokenize(source)?;
    let mut parser = Parser { tokens, index: 0 };
    let mut stmts = Vec::new();
    while !matches!(parser.peek().kind, TokenKind::Eof | TokenKind::RBrace) {
        if parser.eat(&TokenKind::Semi) {
            continue;
        }
        stmts.push(parser.stmt()?);
        parser.eat(&TokenKind::Semi);
    }
    Ok(stmts)
}

impl Parser {
    fn stmt(&mut self) -> Result<Stmt, String> {
        match &self.peek().kind {
            TokenKind::Ident(name) if name == "const" || name == "let" || name == "var" => {
                self.bump();
                let name = self.ident()?;
                self.expect(TokenKind::Eq)?;
                let init = self.assign()?;
                Ok(Stmt::Var { name, init })
            }
            TokenKind::Ident(name) if name == "function" => self.function(),
            TokenKind::Ident(name) if name == "if" => self.if_stmt(),
            TokenKind::Ident(name) if name == "return" => {
                self.bump();
                if matches!(
                    self.peek().kind,
                    TokenKind::Semi | TokenKind::RBrace | TokenKind::Eof
                ) {
                    return Ok(Stmt::Return(None));
                }
                Ok(Stmt::Return(Some(self.assign()?)))
            }
            TokenKind::LBrace => {
                self.bump();
                let mut body = Vec::new();
                while !matches!(self.peek().kind, TokenKind::RBrace | TokenKind::Eof) {
                    if self.eat(&TokenKind::Semi) {
                        continue;
                    }
                    body.push(self.stmt()?);
                    self.eat(&TokenKind::Semi);
                }
                self.expect(TokenKind::RBrace)?;
                Ok(Stmt::Block(body))
            }
            _ => Ok(Stmt::Expr(self.assign()?)),
        }
    }

    fn function(&mut self) -> Result<Stmt, String> {
        self.bump();
        let name = self.ident()?;
        self.expect(TokenKind::LParen)?;
        let mut params = Vec::new();
        if !matches!(self.peek().kind, TokenKind::RParen) {
            params.push(self.ident()?);
            while self.eat(&TokenKind::Comma) {
                if matches!(self.peek().kind, TokenKind::RParen) {
                    break;
                }
                params.push(self.ident()?);
            }
        }
        self.expect(TokenKind::RParen)?;
        let Stmt::Block(body) = self.stmt()? else {
            return Err("function body must be a block".into());
        };
        Ok(Stmt::Fn { name, params, body })
    }

    fn if_stmt(&mut self) -> Result<Stmt, String> {
        let if_line = self.peek().line;
        self.bump();
        self.expect(TokenKind::LParen)?;
        let cond = self.assign()?;
        self.expect(TokenKind::RParen)?;
        let yes = Box::new(self.stmt()?);
        self.eat(&TokenKind::Semi);
        if matches!(&self.peek().kind, TokenKind::Ident(name) if name == "else") {
            let else_line = self.peek().line;
            if else_line == if_line && !matches!(yes.as_ref(), Stmt::Block(_)) {
                return Err("Syntax Error: Unexpected token else".into());
            }
            self.bump();
            let no = Box::new(self.stmt()?);
            return Ok(Stmt::If {
                cond,
                yes,
                no: Some(no),
                if_line,
                else_line: Some(else_line),
            });
        }
        Ok(Stmt::If {
            cond,
            yes,
            no: None,
            if_line,
            else_line: None,
        })
    }

    fn assign(&mut self) -> Result<Expr, String> {
        let expr = self.ternary()?;
        if self.eat(&TokenKind::Eq) {
            let Expr::Ident(name) = expr else {
                return Err("assignment target must be a name".into());
            };
            let value = self.assign()?;
            return Ok(Expr::Assign {
                name,
                expr: Box::new(value),
            });
        }
        Ok(expr)
    }

    fn ternary(&mut self) -> Result<Expr, String> {
        let cond = self.or()?;
        if self.eat(&TokenKind::Question) {
            let yes = self.assign()?;
            self.expect(TokenKind::Colon)?;
            let no = self.ternary()?;
            return Ok(Expr::Ternary {
                cond: Box::new(cond),
                yes: Box::new(yes),
                no: Box::new(no),
            });
        }
        Ok(cond)
    }

    fn or(&mut self) -> Result<Expr, String> {
        let mut left = self.and()?;
        while self.eat(&TokenKind::OrOr) {
            let right = self.and()?;
            left = Expr::Binary {
                op: BinOp::Or,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Expr, String> {
        let mut left = self.eq()?;
        while self.eat(&TokenKind::AndAnd) {
            let right = self.eq()?;
            left = Expr::Binary {
                op: BinOp::And,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn eq(&mut self) -> Result<Expr, String> {
        let mut left = self.rel()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::EqEq => BinOp::Eq,
                TokenKind::NotEq => BinOp::NotEq,
                TokenKind::EqEqEq => BinOp::EqStrict,
                TokenKind::NotEqEq => BinOp::NotEqStrict,
                _ => break,
            };
            self.bump();
            let right = self.rel()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn rel(&mut self) -> Result<Expr, String> {
        let mut left = self.add()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Lt => BinOp::Lt,
                TokenKind::Gt => BinOp::Gt,
                TokenKind::Le => BinOp::Le,
                TokenKind::Ge => BinOp::Ge,
                _ => break,
            };
            self.bump();
            let right = self.add()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn add(&mut self) -> Result<Expr, String> {
        let mut left = self.mul()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Plus => BinOp::Add,
                TokenKind::Minus => BinOp::Sub,
                _ => break,
            };
            self.bump();
            let right = self.mul()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn mul(&mut self) -> Result<Expr, String> {
        let mut left = self.unary()?;
        loop {
            let op = match self.peek().kind {
                TokenKind::Star => BinOp::Mul,
                TokenKind::Slash => BinOp::Div,
                TokenKind::Percent => BinOp::Rem,
                _ => break,
            };
            self.bump();
            let right = self.unary()?;
            left = Expr::Binary {
                op,
                left: Box::new(left),
                right: Box::new(right),
            };
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Expr, String> {
        let op = match self.peek().kind {
            TokenKind::Bang => Some(UnaryOp::Not),
            TokenKind::Minus => Some(UnaryOp::Neg),
            TokenKind::Plus => Some(UnaryOp::Plus),
            _ => None,
        };
        if let Some(op) = op {
            self.bump();
            let expr = self.unary()?;
            return Ok(Expr::Unary {
                op,
                expr: Box::new(expr),
            });
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Result<Expr, String> {
        let mut expr = self.primary()?;
        loop {
            if self.eat(&TokenKind::LParen) {
                let args = self.args()?;
                expr = Expr::Call {
                    callee: Box::new(expr),
                    args,
                };
            } else if self.eat(&TokenKind::Dot) {
                let name = self.ident()?;
                expr = Expr::Member {
                    object: Box::new(expr),
                    name,
                };
            } else if self.eat(&TokenKind::LBrack) {
                let index = self.assign()?;
                self.expect(TokenKind::RBrack)?;
                expr = Expr::Index {
                    object: Box::new(expr),
                    index: Box::new(index),
                };
            } else {
                break;
            }
        }
        Ok(expr)
    }

    fn args(&mut self) -> Result<Vec<Expr>, String> {
        let mut args = Vec::new();
        if self.eat(&TokenKind::RParen) {
            return Ok(args);
        }
        args.push(self.assign()?);
        while self.eat(&TokenKind::Comma) {
            if matches!(self.peek().kind, TokenKind::RParen) {
                break;
            }
            args.push(self.assign()?);
        }
        self.expect(TokenKind::RParen)?;
        Ok(args)
    }

    fn primary(&mut self) -> Result<Expr, String> {
        match self.peek().kind.clone() {
            TokenKind::Number(value) => {
                self.bump();
                Ok(Expr::Number(value))
            }
            TokenKind::String(value) => {
                self.bump();
                Ok(Expr::String(value))
            }
            TokenKind::Ident(name) if name == "true" => {
                self.bump();
                Ok(Expr::Bool(true))
            }
            TokenKind::Ident(name) if name == "false" => {
                self.bump();
                Ok(Expr::Bool(false))
            }
            TokenKind::Ident(name) if name == "null" => {
                self.bump();
                Ok(Expr::Null)
            }
            TokenKind::Ident(name) => {
                self.bump();
                Ok(Expr::Ident(name))
            }
            TokenKind::LBrack => {
                self.bump();
                let mut items = Vec::new();
                if !self.eat(&TokenKind::RBrack) {
                    items.push(self.assign()?);
                    while self.eat(&TokenKind::Comma) {
                        if matches!(self.peek().kind, TokenKind::RBrack) {
                            break;
                        }
                        items.push(self.assign()?);
                    }
                    self.expect(TokenKind::RBrack)?;
                }
                Ok(Expr::Array(items))
            }
            TokenKind::LParen => {
                self.bump();
                let expr = self.assign()?;
                self.expect(TokenKind::RParen)?;
                Ok(expr)
            }
            other => Err(format!("unexpected {other:?}")),
        }
    }
}
