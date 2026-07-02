use errors::{coded_lexer_error, NyraError, Position, Span};

use ast::{FloatKind, IntKind};

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    // Keywords
    Let,
    Mut,
    Fn,
    If,
    Else,
    While,
    Break,
    Continue,
    Return,
    True,
    False,
    Print,
    Import,
    Module,
    Struct,
    Union,
    Impl,
    SelfKw,
    For,
    Const,
    Extern,
    Export,
    Pub,
    Priv,
    Inst,
    Enum,
    Match,
    Spawn,
    Benchmark,
    Parallel,
    Progress,
    In,
    Test,
    Async,
    Await,
    Trait,
    Macro,
    Defer,
    Dyn,
    Unsafe,
    Asm,
    As,
    Move,
    Clone,
    Lifetime(String),

    // Literals / ident
    Identifier(String),
    Number(i64),
    /// Integer literal with type suffix (`255u8`).
    NumberSuffix(i64, IntKind),
    /// IEEE-754 float literal bits (`f64::to_bits`) + explicit width.
    Float {
        bits: u64,
        kind: FloatKind,
    },
    /// Unicode scalar literal (`'a'`, `'\n'`, `'\u{1F600}'`).
    CharLit(u32),
    StringLit(String),
    /// Line doc comment (`/// text`) attached to the following declaration.
    DocComment(String),
    TemplateLit(TemplateLitToken),

    // Types (for annotations)
    TypeInt(IntKind),
    TypeF32,
    TypeF64,
    TypeChar,
    TypeBool,
    TypeString,
    TypeBytes,
    TypePtr,
    TypeVoid,

    // Operators
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    AndAnd,
    OrOr,
    Shl,
    Shr,
    BitOr,
    BitXor,
    Equal,
    EqualEqual,
    BangEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    Bang,
    Ampersand,
    Arrow,
    FatArrow,
    /// `?` — optional chain prefix (also `??` and `?.`).
    Question,
    /// `??` — nullish coalescing.
    NullishCoalesce,
    /// `?.` — optional chaining.
    QuestionDot,
    /// `#[derive(Copy, ...)]`
    AttrDerive(Vec<String>),
    /// `#[no_escape]` — parameter must not escape the function.
    AttrNoEscape,
    /// `#[inline]` — function should be inlined at call sites.
    AttrInline,
    /// `#[hot]` — likely executed; LLVM `inlinehint`.
    AttrHot,
    /// `#[cold]` — unlikely executed; LLVM `cold`.
    AttrCold,
    /// `#[comptime]` — function is evaluated at compile time (see `comptime` file directive).
    AttrComptime,
    LParen,
    RParen,
    LBrace,
    RBrace,
    Colon,
    ColonColon,
    Semicolon,
    Comma,
    Dot,
    DotDot,
    /// `...` — JS-style spread in array / object literals.
    DotDotDot,
    LBracket,
    RBracket,

    // Structural
    Newline,
    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TemplateLitToken {
    pub parts: Vec<TemplateLitPart>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum TemplateLitPart {
    Text(String),
    Interp(String),
}

fn word_to_token_kind(name: &str) -> TokenKind {
    match name {
        "let" => TokenKind::Let,
        "mut" => TokenKind::Mut,
        "fn" => TokenKind::Fn,
        "if" => TokenKind::If,
        "else" => TokenKind::Else,
        "while" => TokenKind::While,
        "break" => TokenKind::Break,
        "continue" => TokenKind::Continue,
        "return" => TokenKind::Return,
        "true" => TokenKind::True,
        "false" => TokenKind::False,
        "print" => TokenKind::Print,
        "import" => TokenKind::Import,
        "module" => TokenKind::Module,
        "struct" => TokenKind::Struct,
        "union" => TokenKind::Union,
        "impl" => TokenKind::Impl,
        "self" => TokenKind::SelfKw,
        "for" => TokenKind::For,
        "const" => TokenKind::Const,
        "extern" => TokenKind::Extern,
        "export" => TokenKind::Export,
        "pub" => TokenKind::Pub,
        "priv" => TokenKind::Priv,
        "inst" => TokenKind::Inst,
        "enum" => TokenKind::Enum,
        "match" => TokenKind::Match,
        "spawn" => TokenKind::Spawn,
        "benchmark" => TokenKind::Benchmark,
        "parallel" => TokenKind::Parallel,
        "progress" => TokenKind::Progress,
        "in" => TokenKind::In,
        "test" => TokenKind::Test,
        "async" => TokenKind::Async,
        "await" => TokenKind::Await,
        "trait" => TokenKind::Trait,
        "macro" => TokenKind::Macro,
        "defer" => TokenKind::Defer,
        "dyn" => TokenKind::Dyn,
        "unsafe" => TokenKind::Unsafe,
        "asm" => TokenKind::Asm,
        "as" => TokenKind::As,
        "move" => TokenKind::Move,
        "clone" => TokenKind::Clone,
        "no_std" => TokenKind::Identifier("no_std".into()),
        name if IntKind::parse_name(name).is_some() => {
            TokenKind::TypeInt(IntKind::parse_name(name).unwrap())
        }
        "f32" => TokenKind::TypeF32,
        "f64" => TokenKind::TypeF64,
        "char" => TokenKind::TypeChar,
        "bool" => TokenKind::TypeBool,
        "string" => TokenKind::TypeString,
        "bytes" => TokenKind::TypeBytes,
        "ptr" => TokenKind::TypePtr,
        "void" => TokenKind::TypeVoid,
        _ => TokenKind::Identifier(name.to_string()),
    }
}

/// Whether an identifier must be written with a leading `@` (reserved word or type name).
pub fn needs_raw_prefix(name: &str) -> bool {
    !matches!(word_to_token_kind(name), TokenKind::Identifier(_))
}

/// Emit source text for an identifier, prefixing `@` when required.
pub fn display_identifier(name: &str) -> String {
    if needs_raw_prefix(name) {
        format!("@{name}")
    } else {
        name.to_string()
    }
}

pub struct Lexer {
    file: String,
    chars: Vec<char>,
    pos: usize,
    line: usize,
    column: usize,
    errors: Vec<NyraError>,
}

impl Lexer {
    pub fn new(source: &str, file: impl Into<String>) -> Self {
        Self {
            file: file.into(),
            chars: source.chars().collect(),
            pos: 0,
            line: 1,
            column: 1,
            errors: vec![],
        }
    }

    pub fn tokenize(mut self) -> (Vec<Token>, Vec<NyraError>) {
        let mut tokens = Vec::new();
        loop {
            self.skip_whitespace_only();
            while self.at_doc_comment() {
                let start = self.current_position();
                let text = self.lex_doc_comment();
                let end = self.current_position();
                tokens.push(Token {
                    kind: TokenKind::DocComment(text),
                    span: Span::new(self.file.clone(), start, end),
                });
                self.skip_whitespace_only();
            }
            while self.at_line_comment() {
                self.advance();
                self.advance();
                while let Some(c) = self.peek() {
                    if c == '\n' {
                        break;
                    }
                    self.advance();
                }
            }
            while self.at_block_comment() {
                let start = self.current_position();
                self.advance();
                self.advance();
                self.skip_block_comment(start);
            }
            self.skip_whitespace_only();
            let start = self.current_position();
            let kind = match self.peek() {
                None => TokenKind::Eof,
                Some(c) => match c {
                    '\'' => self.read_lifetime_or_char(),
                    '+' => {
                        self.advance();
                        TokenKind::Plus
                    }
                    '-' => {
                        self.advance();
                        if self.peek() == Some('>') {
                            self.advance();
                            TokenKind::Arrow
                        } else {
                            TokenKind::Minus
                        }
                    }
                    '*' => {
                        self.advance();
                        TokenKind::Star
                    }
                    '/' => {
                        self.advance();
                        TokenKind::Slash
                    }
                    '%' => {
                        self.advance();
                        TokenKind::Percent
                    }
                    '=' => {
                        self.advance();
                        match self.peek() {
                            Some('=') => {
                                self.advance();
                                TokenKind::EqualEqual
                            }
                            Some('>') => {
                                self.advance();
                                TokenKind::FatArrow
                            }
                            _ => TokenKind::Equal,
                        }
                    }
                    '!' => {
                        self.advance();
                        if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::BangEqual
                        } else {
                            TokenKind::Bang
                        }
                    }
                    '?' => {
                        self.advance();
                        match self.peek() {
                            Some('?') => {
                                self.advance();
                                TokenKind::NullishCoalesce
                            }
                            Some('.') => {
                                self.advance();
                                TokenKind::QuestionDot
                            }
                            _ => TokenKind::Question,
                        }
                    }
                    '&' => {
                        self.advance();
                        if self.peek() == Some('&') {
                            self.advance();
                            TokenKind::AndAnd
                        } else {
                            TokenKind::Ampersand
                        }
                    }
                    '|' => {
                        self.advance();
                        if self.peek() == Some('|') {
                            self.advance();
                            TokenKind::OrOr
                        } else {
                            TokenKind::BitOr
                        }
                    }
                    '<' => {
                        self.advance();
                        if self.peek() == Some('<') {
                            self.advance();
                            TokenKind::Shl
                        } else if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::LessEqual
                        } else {
                            TokenKind::Less
                        }
                    }
                    '>' => {
                        self.advance();
                        if self.peek() == Some('>') {
                            self.advance();
                            TokenKind::Shr
                        } else if self.peek() == Some('=') {
                            self.advance();
                            TokenKind::GreaterEqual
                        } else {
                            TokenKind::Greater
                        }
                    }
                    '^' => {
                        self.advance();
                        TokenKind::BitXor
                    }
                    '(' => {
                        self.advance();
                        TokenKind::LParen
                    }
                    ')' => {
                        self.advance();
                        TokenKind::RParen
                    }
                    '{' => {
                        self.advance();
                        TokenKind::LBrace
                    }
                    '}' => {
                        self.advance();
                        TokenKind::RBrace
                    }
                    ':' => {
                        self.advance();
                        if self.peek() == Some(':') {
                            self.advance();
                            TokenKind::ColonColon
                        } else {
                            TokenKind::Colon
                        }
                    }
                    ';' => {
                        self.advance();
                        TokenKind::Semicolon
                    }
                    ',' => {
                        self.advance();
                        TokenKind::Comma
                    }
                    '.' => {
                        self.advance();
                        if self.peek() == Some('.') {
                            self.advance();
                            if self.peek() == Some('.') {
                                self.advance();
                                TokenKind::DotDotDot
                            } else {
                                TokenKind::DotDot
                            }
                        } else {
                            TokenKind::Dot
                        }
                    }
                    '[' => {
                        self.advance();
                        TokenKind::LBracket
                    }
                    ']' => {
                        self.advance();
                        TokenKind::RBracket
                    }
                    '"' => self.read_string(),
                    '`' => self.read_template_literal(),
                    '#' => self.read_attribute_or_error(start),
                    '@' => self.read_raw_identifier(start),
                    '\n' => {
                        self.advance();
                        TokenKind::Newline
                    }
                    c if c.is_ascii_digit() => self.read_number(),
                    c if c.is_ascii_alphabetic() || c == '_' => self.read_identifier_or_keyword(),
                    _ => {
                        let span = self.span_from(start);
                        self.errors.push(coded_lexer_error(
                            span,
                            format!("Invalid character '{c}'"),
                        ));
                        self.advance();
                        continue;
                    }
                },
            };

            let end = self.current_position();
            let span = Span::new(self.file.clone(), start, end);
            let is_eof = kind == TokenKind::Eof;
            let is_newline = kind == TokenKind::Newline;
            tokens.push(Token { kind, span });

            if is_eof {
                break;
            }
            if is_newline {
                continue;
            }
        }

        // Collapse duplicate newlines, ensure single Eof
        let mut cleaned = Vec::new();
        let mut last_newline = false;
        for t in tokens {
            match &t.kind {
                TokenKind::Newline => {
                    if !last_newline {
                        cleaned.push(t);
                        last_newline = true;
                    }
                }
                TokenKind::Eof => {
                    cleaned.push(t);
                    break;
                }
                _ => {
                    last_newline = false;
                    cleaned.push(t);
                }
            }
        }
        if cleaned.last().map(|t| &t.kind) != Some(&TokenKind::Eof) {
            let pos = cleaned
                .last()
                .map(|t| t.span.end)
                .unwrap_or_default();
            cleaned.push(Token {
                kind: TokenKind::Eof,
                span: Span::new(self.file.clone(), pos, pos),
            });
        }

        let errors = std::mem::take(&mut self.errors);
        (cleaned, errors)
    }

    fn read_number(&mut self) -> TokenKind {
        let start = self.current_position();
        let (mut value, mut has_digit) = if self.peek() == Some('0') {
            self.advance();
            if matches!(self.peek(), Some('x' | 'X')) {
                self.advance();
                return self.read_hex_int(start);
            }
            (0i64, true)
        } else {
            (0i64, false)
        };
        let mut overflow = false;
        loop {
            match self.peek() {
                Some(c) if c.is_ascii_digit() => {
                    has_digit = true;
                    let digit = (c as u8 - b'0') as i64;
                    if !overflow {
                        match value.checked_mul(10).and_then(|v| v.checked_add(digit)) {
                            Some(v) => value = v,
                            None => {
                                overflow = true;
                                self.errors.push(coded_lexer_error(
                                    self.span_from(start),
                                    "Integer literal overflow",
                                ));
                            }
                        }
                    }
                    self.advance();
                }
                Some('_') => {
                    if !has_digit {
                        break;
                    }
                    match self.peek_next() {
                        Some(next) if next.is_ascii_digit() => {
                            self.advance();
                        }
                        _ => {
                            self.errors.push(coded_lexer_error(
                                self.span_from(start),
                                "Numeric separators '_' must appear between digits (not at the end or doubled)",
                            ));
                            break;
                        }
                    }
                }
                _ => break,
            }
        }
        if self.peek() == Some('.') && self.peek_next().is_some_and(|c| c.is_ascii_digit()) {
            self.advance(); // .
            let mut frac = 0.0f64;
            let mut place = 0.1f64;
            while let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    frac += (c as u8 - b'0') as f64 * place;
                    place *= 0.1;
                    self.advance();
                } else {
                    break;
                }
            }
            let f = value as f64 + frac;
            let kind = self.try_read_float_suffix().unwrap_or(FloatKind::F64);
            return TokenKind::Float {
                bits: f.to_bits(),
                kind,
            };
        }
        if let Some(kind) = self.try_read_float_suffix() {
            let f = value as f64;
            return TokenKind::Float {
                bits: f.to_bits(),
                kind,
            };
        }
        if let Some(kind) = self.try_read_int_suffix() {
            return TokenKind::NumberSuffix(value, kind);
        }
        TokenKind::Number(value)
    }

    fn read_hex_int(&mut self, start: Position) -> TokenKind {
        let mut value = 0i64;
        let mut has_digit = false;
        let mut overflow = false;
        loop {
            let digit = match self.peek() {
                Some(c) if c.is_ascii_digit() => Some((c as u8 - b'0') as i64),
                Some(c) if ('a'..='f').contains(&c) => Some(10 + (c as u8 - b'a') as i64),
                Some(c) if ('A'..='F').contains(&c) => Some(10 + (c as u8 - b'A') as i64),
                Some('_') => {
                    if !has_digit {
                        break;
                    }
                    match self.peek_next() {
                        Some(next) if next.is_ascii_hexdigit() => {
                            self.advance();
                            continue;
                        }
                        _ => {
                            self.errors.push(coded_lexer_error(
                                self.span_from(start),
                                "Numeric separators '_' must appear between hex digits (not at the end or doubled)",
                            ));
                            break;
                        }
                    }
                }
                _ => None,
            };
            match digit {
                Some(d) => {
                    has_digit = true;
                    if !overflow {
                        match value.checked_mul(16).and_then(|v| v.checked_add(d)) {
                            Some(v) => value = v,
                            None => {
                                overflow = true;
                                self.errors.push(coded_lexer_error(
                                    self.span_from(start),
                                    "Integer literal overflow",
                                ));
                            }
                        }
                    }
                    self.advance();
                }
                None => break,
            }
        }
        if !has_digit {
            self.errors.push(coded_lexer_error(
                self.span_from(start),
                "Expected hex digits after 0x",
            ));
            return TokenKind::Number(0);
        }
        if let Some(kind) = self.try_read_int_suffix() {
            return TokenKind::NumberSuffix(value, kind);
        }
        TokenKind::Number(value)
    }

    fn try_read_int_suffix(&mut self) -> Option<IntKind> {
        if !matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            return None;
        }
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        IntKind::parse_name(&name)
    }

    fn try_read_float_suffix(&mut self) -> Option<FloatKind> {
        if !matches!(self.peek(), Some(c) if c.is_ascii_alphabetic()) {
            return None;
        }
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        FloatKind::parse_name(&name)
    }

    fn read_string(&mut self) -> TokenKind {
        self.advance(); // opening "
        let mut s = String::new();
        while let Some(c) = self.peek() {
            if c == '"' {
                self.advance();
                return TokenKind::StringLit(s);
            }
            if c == '\\' {
                self.advance();
                if let Some(ch) = self.read_string_escape() {
                    s.push(ch);
                }
            } else if c == '\n' {
                break;
            } else {
                s.push(c);
                self.advance();
            }
        }
        TokenKind::StringLit(s)
    }

    fn read_string_escape(&mut self) -> Option<char> {
        let esc = self.peek()?;
        match esc {
            'n' => {
                self.advance();
                Some('\n')
            }
            'r' => {
                self.advance();
                Some('\r')
            }
            't' => {
                self.advance();
                Some('\t')
            }
            '0'..='7' => {
                let mut val = (esc as u32) - ('0' as u32);
                self.advance();
                for _ in 0..2 {
                    if let Some(c) = self.peek() {
                        if ('0'..='7').contains(&c) {
                            val = val * 8 + (c as u32 - '0' as u32);
                            self.advance();
                        } else {
                            break;
                        }
                    } else {
                        break;
                    }
                }
                char::from_u32(val)
            }
            'x' => {
                self.advance();
                let hi = self.read_hex_digit()?;
                let lo = self.read_hex_digit()?;
                char::from_u32((hi << 4) | lo)
            }
            'u' => {
                self.advance();
                if self.peek() != Some('{') {
                    return None;
                }
                self.advance();
                let mut hex = String::new();
                while let Some(c) = self.peek() {
                    if c == '}' {
                        self.advance();
                        break;
                    }
                    if c.is_ascii_hexdigit() {
                        hex.push(c);
                        self.advance();
                    } else {
                        return None;
                    }
                }
                u32::from_str_radix(&hex, 16).ok().and_then(char::from_u32)
            }
            '\\' => {
                self.advance();
                Some('\\')
            }
            '"' => {
                self.advance();
                Some('"')
            }
            other => {
                self.advance();
                Some(other)
            }
        }
    }

    fn read_hex_digit(&mut self) -> Option<u32> {
        let c = self.peek()?;
        let v = c.to_digit(16)?;
        self.advance();
        Some(v)
    }

    fn read_lifetime_or_char(&mut self) -> TokenKind {
        self.advance(); // opening '
        let start = self.current_position();
        if let Some(c) = self.peek() {
            if c.is_ascii_alphabetic() || c == '_' {
                let mut id = String::new();
                while let Some(ch) = self.peek() {
                    if ch.is_ascii_alphanumeric() || ch == '_' {
                        id.push(ch);
                        self.advance();
                    } else {
                        break;
                    }
                }
                if self.peek() == Some('\'') && id.len() == 1 {
                    self.advance();
                    return TokenKind::CharLit(id.chars().next().unwrap_or('\0') as u32);
                }
                return TokenKind::Lifetime(format!("'{id}"));
            }
        }
        match self.read_char_codepoint() {
            Some(cp) if Self::is_valid_char(cp) => {
                if self.peek() == Some('\'') {
                    self.advance();
                    TokenKind::CharLit(cp)
                } else {
                    self.errors.push(coded_lexer_error(
                        self.span_from(start),
                        "Character literal must end with a single closing quote",
                    ));
                    TokenKind::CharLit(cp)
                }
            }
            _ => {
                self.errors.push(coded_lexer_error(
                    self.span_from(start),
                    "Invalid character literal",
                ));
                while self.peek().is_some_and(|c| c != '\'' && c != '\n') {
                    self.advance();
                }
                if self.peek() == Some('\'') {
                    self.advance();
                }
                TokenKind::CharLit(0)
            }
        }
    }

    fn is_valid_char(cp: u32) -> bool {
        cp <= 0x10FFFF && !(0xD800..=0xDFFF).contains(&cp)
    }

    fn read_char_codepoint(&mut self) -> Option<u32> {
        match self.peek()? {
            '\'' => None,
            '\\' => {
                self.advance();
                let esc = self.peek()?;
                match esc {
                    'n' => {
                        self.advance();
                        Some('\n' as u32)
                    }
                    'r' => {
                        self.advance();
                        Some('\r' as u32)
                    }
                    't' => {
                        self.advance();
                        Some('\t' as u32)
                    }
                    '0' => {
                        self.advance();
                        Some(0)
                    }
                    '\\' | '\'' => {
                        self.advance();
                        Some(esc as u32)
                    }
                    'u' => {
                        self.advance();
                        if self.peek() != Some('{') {
                            return None;
                        }
                        self.advance();
                        let mut hex = String::new();
                        while let Some(c) = self.peek() {
                            if c == '}' {
                                self.advance();
                                break;
                            }
                            if c.is_ascii_hexdigit() {
                                hex.push(c);
                                self.advance();
                            } else {
                                return None;
                            }
                        }
                        u32::from_str_radix(&hex, 16).ok()
                    }
                    _ => None,
                }
            }
            c => {
                self.advance();
                Some(c as u32)
            }
        }
    }

    fn read_template_literal(&mut self) -> TokenKind {
        self.advance(); // opening `
        let mut parts: Vec<TemplateLitPart> = Vec::new();
        let mut text = String::new();
        loop {
            match self.peek() {
                None => break,
                Some('`') => {
                    self.advance();
                    if !text.is_empty() {
                        parts.push(TemplateLitPart::Text(text));
                    }
                    return TokenKind::TemplateLit(TemplateLitToken { parts });
                }
                Some('\\') => {
                    self.advance();
                    if let Some(esc) = self.peek() {
                        let ch = match esc {
                            'n' => '\n',
                            'r' => '\r',
                            't' => '\t',
                            '\\' => '\\',
                            '`' => '`',
                            '{' => '{',
                            '}' => '}',
                            other => other,
                        };
                        text.push(ch);
                        self.advance();
                    }
                }
                Some('{') => {
                    self.advance();
                    if self.peek() == Some('{') {
                        self.advance();
                        text.push('{');
                        continue;
                    }
                    if text.ends_with('$') {
                        text.pop();
                        if !text.is_empty() {
                            parts.push(TemplateLitPart::Text(text));
                            text = String::new();
                        }
                        let mut expr_src = String::new();
                        let mut depth = 1usize;
                        while let Some(c) = self.peek() {
                            if c == '{' {
                                depth += 1;
                            } else if c == '}' {
                                depth -= 1;
                                if depth == 0 {
                                    self.advance();
                                    break;
                                }
                            }
                            expr_src.push(c);
                            self.advance();
                        }
                        parts.push(TemplateLitPart::Interp(expr_src));
                    } else {
                        text.push('{');
                    }
                }
                Some(c) => {
                    text.push(c);
                    self.advance();
                }
            }
        }
        if !text.is_empty() {
            parts.push(TemplateLitPart::Text(text));
        }
        TokenKind::TemplateLit(TemplateLitToken { parts })
    }

    fn read_ident_text(&mut self) -> String {
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        name
    }

    fn read_identifier_or_keyword(&mut self) -> TokenKind {
        let name = self.read_ident_text();
        word_to_token_kind(&name)
    }

    /// `@name` — use a reserved word as an identifier (like Rust `r#name`).
    fn read_raw_identifier(&mut self, start: Position) -> TokenKind {
        self.advance(); // @
        if !matches!(self.peek(), Some(c) if c.is_ascii_alphabetic() || c == '_') {
            self.errors.push(coded_lexer_error(
                self.span_from(start),
                "Expected identifier after '@'",
            ));
            return TokenKind::Identifier(String::new());
        }
        TokenKind::Identifier(self.read_ident_text())
    }

    fn read_attribute_or_error(&mut self, start: errors::Position) -> TokenKind {
        self.advance(); // #
        if self.peek() != Some('[') {
            let span = self.span_from(start);
            self.errors.push(coded_lexer_error(
                span,
                "Expected '[' after '#'",
            ));
            return TokenKind::AttrDerive(vec![]);
        }
        self.advance(); // [
        self.skip_whitespace_and_comments();
        let mut name = String::new();
        while let Some(c) = self.peek() {
            if c.is_ascii_alphanumeric() || c == '_' {
                name.push(c);
                self.advance();
            } else {
                break;
            }
        }
        if name == "derive" {
            self.skip_whitespace_and_comments();
            if self.peek() != Some('(') {
                let span = self.span_from(start);
                self.errors.push(coded_lexer_error(
                    span,
                    "Expected '(' after derive",
                ));
                self.skip_to_attr_end();
                return TokenKind::AttrDerive(vec![]);
            }
            self.advance(); // (
            let mut derives = Vec::new();
            loop {
                self.skip_whitespace_and_comments();
                if self.peek() == Some(')') {
                    self.advance();
                    break;
                }
                let mut item = String::new();
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == '_' {
                        item.push(c);
                        self.advance();
                    } else {
                        break;
                    }
                }
                if item.is_empty() {
                    break;
                }
                derives.push(item);
                self.skip_whitespace_and_comments();
                if self.peek() == Some(',') {
                    self.advance();
                    continue;
                }
                if self.peek() == Some(')') {
                    self.advance();
                    break;
                }
            }
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrDerive(derives);
        }
        if name == "no_escape" {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrNoEscape;
        }
        if name == "inline" {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrInline;
        }
        if name == "hot" {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrHot;
        }
        if name == "cold" {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrCold;
        }
        if name == "comptime" {
            self.skip_whitespace_and_comments();
            if self.peek() == Some(']') {
                self.advance();
            }
            return TokenKind::AttrComptime;
        }
        {
            let span = self.span_from(start);
            self.errors.push(coded_lexer_error(
                span,
                format!("Unknown attribute '#[{name}]' (supported: derive, no_escape, inline, hot, cold, comptime)"),
            ));
            self.skip_to_attr_end();
            return TokenKind::AttrDerive(vec![]);
        }
    }

    fn skip_to_attr_end(&mut self) {
        while let Some(c) = self.peek() {
            if c == ']' {
                self.advance();
                break;
            }
            self.advance();
        }
    }

    fn skip_whitespace_only(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                _ => break,
            }
        }
    }

    fn peek_nth(&self, n: usize) -> Option<char> {
        self.chars.get(self.pos + n).copied()
    }

    fn at_doc_comment(&self) -> bool {
        self.peek() == Some('/')
            && self.peek_nth(1) == Some('/')
            && self.peek_nth(2) == Some('/')
    }

    fn at_line_comment(&self) -> bool {
        self.peek() == Some('/')
            && self.peek_nth(1) == Some('/')
            && self.peek_nth(2) != Some('/')
    }

    fn at_block_comment(&self) -> bool {
        self.peek() == Some('/') && self.peek_nth(1) == Some('*')
    }

    fn lex_doc_comment(&mut self) -> String {
        self.advance();
        self.advance();
        self.advance();
        let mut text = String::new();
        while let Some(c) = self.peek() {
            if c == '\n' {
                break;
            }
            text.push(c);
            self.advance();
        }
        text.trim().to_string()
    }

    fn skip_whitespace_and_comments(&mut self) {
        loop {
            match self.peek() {
                Some(' ') | Some('\t') | Some('\r') => {
                    self.advance();
                }
                Some('/') if self.peek_nth(1) == Some('/') && self.peek_nth(2) == Some('/') => {
                    break;
                }
                Some('/') if self.peek_next() == Some('/') => {
                    while let Some(c) = self.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.advance();
                    }
                }
                Some('/') if self.peek_next() == Some('*') => {
                    let start = self.current_position();
                    self.advance(); // /
                    self.advance(); // *
                    self.skip_block_comment(start);
                }
                _ => break,
            }
        }
    }

    /// Consume through closing `*/`; `start` is the position of `/` in `/*`.
    fn skip_block_comment(&mut self, start: Position) {
        loop {
            match self.peek() {
                None => {
                    self.errors.push(coded_lexer_error(
                        Span::new(self.file.clone(), start, self.current_position()),
                        "unclosed block comment",
                    ));
                    return;
                }
                Some('*') if self.peek_next() == Some('/') => {
                    self.advance(); // *
                    self.advance(); // /
                    return;
                }
                Some('\n') => {
                    self.advance();
                }
                Some(_) => {
                    self.advance();
                }
            }
        }
    }

    fn peek(&self) -> Option<char> {
        self.chars.get(self.pos).copied()
    }

    fn peek_next(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn advance(&mut self) {
        if let Some(c) = self.chars.get(self.pos) {
            if *c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            self.pos += 1;
        }
    }

    fn current_position(&self) -> Position {
        Position {
            line: self.line,
            column: self.column,
        }
    }

    fn span_from(&self, start: Position) -> Span {
        Span::new(self.file.clone(), start, self.current_position())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokenizes_js_style_template_interpolation() {
        let (tokens, errs) = Lexer::new("`${name}hello`", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let lit = tokens
            .iter()
            .find_map(|t| match &t.kind {
                TokenKind::TemplateLit(tl) => Some(tl),
                _ => None,
            })
            .expect("template literal");
        assert_eq!(lit.parts.len(), 2);
        match &lit.parts[0] {
            TemplateLitPart::Interp(src) => assert_eq!(src, "name"),
            other => panic!("expected interpolation, got {other:?}"),
        }
        match &lit.parts[1] {
            TemplateLitPart::Text(s) => assert_eq!(s, "hello"),
            other => panic!("expected text, got {other:?}"),
        }
    }

    #[test]
    fn bare_braces_in_template_are_literal_text() {
        let (tokens, errs) = Lexer::new("`Hello {name}!`", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let lit = tokens
            .iter()
            .find_map(|t| match &t.kind {
                TokenKind::TemplateLit(tl) => Some(tl),
                _ => None,
            })
            .expect("template literal");
        assert_eq!(lit.parts.len(), 1);
        match &lit.parts[0] {
            TemplateLitPart::Text(s) => assert_eq!(s, "Hello {name}!"),
            other => panic!("expected literal text, got {other:?}"),
        }
    }

    #[test]
    fn tokenizes_numeric_separators() {
        let (tokens, errs) = Lexer::new("let n = 1_000_000", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Number(1_000_000))));
    }

    #[test]
    fn tokenizes_numeric_separator_10_0000() {
        let (tokens, errs) = Lexer::new("10_0000", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Number(100_000))));
    }

    #[test]
    fn rejects_trailing_numeric_separator() {
        let (_, errs) = Lexer::new("100_", "test.ny").tokenize();
        assert!(!errs.is_empty());
    }

    #[test]
    fn rejects_doubled_numeric_separator() {
        let (_, errs) = Lexer::new("10__000", "test.ny").tokenize();
        assert!(!errs.is_empty());
    }

    #[test]
    fn tokenizes_block_comment_inline() {
        let (tokens, errs) = Lexer::new("let x = 1 /* add */ + 2", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let kinds: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .map(|t| t.kind.clone())
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Identifier("x".into()),
                TokenKind::Equal,
                TokenKind::Number(1),
                TokenKind::Plus,
                TokenKind::Number(2),
            ]
        );
    }

    #[test]
    fn tokenizes_multiline_block_comment() {
        let src = "let n = 42 /* line one\n line two */ + 0";
        let (tokens, errs) = Lexer::new(src, "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Number(42))));
    }

    #[test]
    fn tokenizes_empty_block_comment() {
        let (tokens, errs) = Lexer::new("/**/ let x = 1", "test.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::Identifier(s) if s == "x")));
    }

    #[test]
    fn rejects_unclosed_block_comment() {
        let (_, errs) = Lexer::new("let x = 1 /* never ends", "test.ny").tokenize();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.message.contains("unclosed block comment")));
    }

    #[test]
    fn tokenizes_let_assignment() {
        let (tokens, errs) = Lexer::new("let x = 10", "test.ny").tokenize();
        assert!(errs.is_empty());
        let kinds: Vec<_> = tokens
            .iter()
            .filter(|t| t.kind != TokenKind::Newline && t.kind != TokenKind::Eof)
            .map(|t| t.kind.clone())
            .collect();
        assert_eq!(
            kinds,
            vec![
                TokenKind::Let,
                TokenKind::Identifier("x".into()),
                TokenKind::Equal,
                TokenKind::Number(10),
            ]
        );
    }

    #[test]
    fn tokenizes_keywords() {
        let src = "fn async await spawn unsafe defer trait impl macro";
        let (tokens, errs) = Lexer::new(src, "k.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Fn)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Async)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Spawn)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Trait)));
    }

    #[test]
    fn tokenizes_string_literal() {
        let (tokens, errs) = Lexer::new(r#""hello""#, "s.ny").tokenize();
        assert!(errs.is_empty());
        assert!(tokens
            .iter()
            .any(|t| matches!(&t.kind, TokenKind::StringLit(s) if s == "hello")));
    }

    #[test]
    fn tokenizes_operators() {
        let (tokens, errs) = Lexer::new("== != && || .. ... =>", "o.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::EqualEqual)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::DotDot)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::DotDotDot)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::FatArrow)));
    }

    #[test]
    fn tokenizes_negative_number() {
        let (tokens, errs) = Lexer::new("-42", "n.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Minus)));
        assert!(tokens.iter().any(|t| matches!(t.kind, TokenKind::Number(42))));
    }

    #[test]
    fn rejects_integer_literal_overflow() {
        let (_, errs) = Lexer::new("888888888888888888888888", "overflow.ny").tokenize();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.message.contains("overflow")));
    }

    #[test]
    fn integer_literal_overflow_does_not_panic() {
        let src = "let let n() { if ((888888888888888888888888((et";
        let result = std::panic::catch_unwind(|| Lexer::new(src, "fuzz.ny").tokenize());
        assert!(result.is_ok(), "lexer must not panic on huge integer literal");
    }

    #[test]
    fn rejects_invalid_character_in_number() {
        let (_, errs) = Lexer::new("12@34", "bad.ny").tokenize();
        assert!(!errs.is_empty());
    }

    #[test]
    fn hex_integer_literal() {
        let (tokens, errs) = Lexer::new("0xff 0xFFu8 0x1_0", "hex.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let nums: Vec<_> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Number(n) => Some(*n),
                TokenKind::NumberSuffix(n, _) => Some(*n),
                _ => None,
            })
            .collect();
        assert_eq!(nums, vec![255, 255, 16]);
    }

    #[test]
    fn string_escape_octal_and_hex() {
        let (tokens, errs) = Lexer::new(r#""\033\n\x1b""#, "s.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let lit = tokens
            .iter()
            .find_map(|t| match &t.kind {
                TokenKind::StringLit(s) => Some(s.clone()),
                _ => None,
            })
            .expect("string literal");
        assert_eq!(lit, "\x1b\n\x1b");
    }

    #[test]
    fn tokenizes_raw_identifiers_for_reserved_words() {
        let (tokens, errs) = Lexer::new("let @module = 1\nlet @clone = @module", "raw.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        let idents: Vec<_> = tokens
            .iter()
            .filter_map(|t| match &t.kind {
                TokenKind::Identifier(s) => Some(s.as_str()),
                _ => None,
            })
            .collect();
        assert_eq!(idents, vec!["module", "clone", "module"]);
    }

    #[test]
    fn raw_identifier_allows_type_names() {
        let (tokens, errs) = Lexer::new("let @i32 = 0", "raw.ny").tokenize();
        assert!(errs.is_empty(), "{errs:?}");
        assert!(tokens.iter().any(|t| {
            matches!(&t.kind, TokenKind::Identifier(s) if s == "i32")
        }));
        assert!(!tokens.iter().any(|t| matches!(t.kind, TokenKind::TypeInt(_))));
    }

    #[test]
    fn rejects_bare_at_sign() {
        let (_, errs) = Lexer::new("let x = @", "raw.ny").tokenize();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.message.contains("Expected identifier after '@'")));
    }

    #[test]
    fn rejects_at_followed_by_digit() {
        let (_, errs) = Lexer::new("let x = @1", "raw.ny").tokenize();
        assert!(!errs.is_empty());
        assert!(errs.iter().any(|e| e.message.contains("Expected identifier after '@'")));
    }

    #[test]
    fn needs_raw_prefix_detects_keywords_and_types() {
        assert!(needs_raw_prefix("clone"));
        assert!(needs_raw_prefix("module"));
        assert!(needs_raw_prefix("i32"));
        assert!(!needs_raw_prefix("foo"));
        assert!(!needs_raw_prefix("no_std"));
    }
}
