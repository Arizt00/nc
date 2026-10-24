use ncc_diagnostics::Diagnostic;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenKind {
    Identifier(String),
    Number(String),
    String(String),
    Char(char),

    LBrace,
    RBrace,
    LParen,
    RParen,
    LBracket,
    RBracket,
    Lt,
    Gt,
    Colon,
    Semicolon,
    Comma,
    Dot,
    At,
    Star,

    Arrow,
    Arrow2,
    Shift2,
    Shift3,
    BangShift,
    LessDash,
    Pipe,

    Greater,
    Less,
    GreaterEq,
    LessEq,
    Eq,
    EqEq,
    NotEq,
    Bang,
    Tilde,
    Plus,
    PlusPlus,
    PlusEq,
    Minus,
    MinusMinus,
    MinusEq,
    Slash,
    Percent,
    Amp,
    Caret,
    Question,
    ShiftLeft,
    StarEq,
    AndAnd,
    OrOr,

    KeywordUse,
    KeywordImport,
    KeywordStc,
    KeywordLocal,
    KeywordFn,
    KeywordMove,
    KeywordProbe,
    KeywordBind,
    KeywordBinary,
    KeywordRaw,
    KeywordStructure,
    KeywordStr,
    KeywordFinal,
    KeywordFa,
    KeywordPublic,
    KeywordPrivate,
    KeywordInternal,
    KeywordStrictly,
    KeywordReturn,
    KeywordDev,
    KeywordIf,
    KeywordElse,
    KeywordWhile,
    KeywordRef,
    KeywordMain,
    KeywordSkkMain,
    KeywordTrue,
    KeywordFalse,

    KeywordLy,
    KeywordCnt,
    KeywordCut,

    KeywordAnchor,
    KeywordSlot,
    KeywordPattern,
    KeywordMask,
    KeywordWindow,
    KeywordSignal,
    KeywordBus,
    KeywordState,
    KeywordTiming,
    KeywordEmit,
    KeywordSample,
    KeywordLatch,
    KeywordAssert,
    KeywordRupture,
    KeywordBridge,
    KeywordSeal,

    KeywordReg,
    KeywordFlag,
    KeywordLabel,
    KeywordInst,

    Eof,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub kind: TokenKind,
    pub line: usize,
    pub column: usize,
}

pub fn lex(input: &str) -> Result<Vec<Token>, Diagnostic> {
    let mut lexer = Lexer::new(input);
    lexer.lex_all()
}

struct Lexer<'a> {
    chars: Vec<char>,
    pos: usize,
    line: usize,
    col: usize,
    _src: &'a str,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Self {
            chars: src.chars().collect(),
            pos: 0,
            line: 1,
            col: 1,
            _src: src,
        }
    }

    fn lex_all(&mut self) -> Result<Vec<Token>, Diagnostic> {
        let mut tokens = Vec::new();

        while !self.is_eof() {
            self.skip_ws_and_comments()?;
            if self.is_eof() {
                break;
            }

            let line = self.line;
            let column = self.col;
            let ch = self.peek();

            let kind = match ch {
                '{' => {
                    self.bump();
                    TokenKind::LBrace
                }
                '}' => {
                    self.bump();
                    TokenKind::RBrace
                }
                '(' => {
                    self.bump();
                    TokenKind::LParen
                }
                ')' => {
                    self.bump();
                    TokenKind::RParen
                }
                '[' => {
                    self.bump();
                    TokenKind::LBracket
                }
                ']' => {
                    self.bump();
                    TokenKind::RBracket
                }
                ':' => {
                    self.bump();
                    TokenKind::Colon
                }
                ';' => {
                    self.bump();
                    TokenKind::Semicolon
                }
                ',' => {
                    self.bump();
                    TokenKind::Comma
                }
                '.' => {
                    self.bump();
                    TokenKind::Dot
                }
                '@' => {
                    self.bump();
                    TokenKind::At
                }
                '*' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::StarEq
                    } else {
                        TokenKind::Star
                    }
                }
                '|' => {
                    self.bump();
                    if self.match_char('|') {
                        TokenKind::OrOr
                    } else {
                        TokenKind::Pipe
                    }
                }
                '&' => {
                    self.bump();
                    if self.match_char('&') {
                        TokenKind::AndAnd
                    } else {
                        TokenKind::Amp
                    }
                }
                '^' => {
                    self.bump();
                    TokenKind::Caret
                }
                '?' => {
                    self.bump();
                    TokenKind::Question
                }
                '~' => {
                    self.bump();
                    TokenKind::Tilde
                }
                '+' => {
                    self.bump();
                    if self.match_char('+') {
                        TokenKind::PlusPlus
                    } else if self.match_char('=') {
                        TokenKind::PlusEq
                    } else {
                        TokenKind::Plus
                    }
                }
                '/' => {
                    self.bump();
                    TokenKind::Slash
                }
                '%' => {
                    self.bump();
                    TokenKind::Percent
                }
                '-' => {
                    self.bump();
                    if self.match_char('>') {
                        if self.match_char('>') {
                            TokenKind::Arrow2
                        } else {
                            TokenKind::Arrow
                        }
                    } else if self.match_char('-') {
                        TokenKind::MinusMinus
                    } else if self.match_char('=') {
                        TokenKind::MinusEq
                    } else {
                        TokenKind::Minus
                    }
                }
                '<' => {
                    self.bump();
                    if self.match_char('-') {
                        TokenKind::LessDash
                    } else if self.match_char('<') {
                        TokenKind::ShiftLeft
                    } else if self.match_char('=') {
                        TokenKind::LessEq
                    } else {
                        TokenKind::Less
                    }
                }
                '>' => {
                    self.bump();
                    if self.match_char('>') {
                        if self.match_char('>') {
                            TokenKind::Shift3
                        } else {
                            TokenKind::Shift2
                        }
                    } else if self.match_char('=') {
                        TokenKind::GreaterEq
                    } else {
                        TokenKind::Greater
                    }
                }
                '!' => {
                    self.bump();
                    if self.match_char('>') {
                        TokenKind::BangShift
                    } else if self.match_char('=') {
                        TokenKind::NotEq
                    } else {
                        TokenKind::Bang
                    }
                }
                '=' => {
                    self.bump();
                    if self.match_char('=') {
                        TokenKind::EqEq
                    } else {
                        TokenKind::Eq
                    }
                }
                '"' => self.lex_string(line, column)?,
                '\'' => self.lex_char(line, column)?,
                c if c.is_ascii_digit() => self.lex_number(),
                c if is_ident_start(c) => self.lex_identifier_or_keyword(),
                _ => {
                    return Err(Diagnostic::new(
                        format!("Unexpected character '{}'", ch),
                        line,
                        column,
                    ))
                }
            };

            tokens.push(Token { kind, line, column });
        }

        tokens.push(Token {
            kind: TokenKind::Eof,
            line: self.line,
            column: self.col,
        });

        Ok(tokens)
    }

    fn skip_ws_and_comments(&mut self) -> Result<(), Diagnostic> {
        loop {
            while !self.is_eof() && self.peek().is_whitespace() {
                self.bump();
            }

            if self.is_eof() {
                return Ok(());
            }

            if self.peek() == '/' && self.peek2() == Some('/') {
                while !self.is_eof() && self.peek() != '\n' {
                    self.bump();
                }
                continue;
            }

            if self.peek() == '/' && self.peek2() == Some('*') {
                let start_line = self.line;
                let start_col = self.col;
                self.bump();
                self.bump();
                let mut terminated = false;
                while !self.is_eof() {
                    if self.peek() == '*' && self.peek2() == Some('/') {
                        self.bump();
                        self.bump();
                        terminated = true;
                        break;
                    }
                    self.bump();
                }
                if !terminated {
                    return Err(Diagnostic::new(
                        "Unterminated block comment",
                        start_line,
                        start_col,
                    ));
                }
                continue;
            }

            break;
        }
        Ok(())
    }

    fn lex_string(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        self.bump();
        let mut s = String::new();

        while !self.is_eof() && self.peek() != '"' {
            if self.peek() == '\\' {
                s.push(self.lex_string_escape()?);
            } else {
                s.push(self.bump());
            }
        }

        if self.is_eof() {
            return Err(Diagnostic::new("Unterminated string literal", line, column));
        }

        self.bump();
        Ok(TokenKind::String(s))
    }

    fn lex_char(&mut self, line: usize, column: usize) -> Result<TokenKind, Diagnostic> {
        self.bump();
        if self.is_eof() {
            return Err(Diagnostic::new("Unterminated char literal", line, column));
        }

        let ch = if self.peek() == '\\' {
            self.lex_string_escape()?
        } else {
            self.bump()
        };

        if self.is_eof() || self.peek() != '\'' {
            return Err(Diagnostic::new(
                "Char literal must contain exactly one character",
                line,
                column,
            ));
        }
        self.bump();

        if u32::from(ch) > u32::from(u8::MAX) {
            return Err(Diagnostic::new(
                "Char literal must fit in one byte; use str for Unicode text",
                line,
                column,
            ));
        }

        Ok(TokenKind::Char(ch))
    }

    fn lex_string_escape(&mut self) -> Result<char, Diagnostic> {
        let escape_line = self.line;
        let escape_col = self.col;
        self.bump();

        if self.is_eof() {
            return Err(Diagnostic::new(
                "Unterminated string escape",
                escape_line,
                escape_col,
            ));
        }

        let escape = self.bump();
        match escape {
            'n' => Ok('\n'),
            'r' => Ok('\r'),
            't' => Ok('\t'),
            '0' => Ok('\0'),
            '"' => Ok('"'),
            '\'' => Ok('\''),
            '\\' => Ok('\\'),
            '&' => Ok('&'),
            '?' => Ok('?'),
            '#' => Ok('#'),
            'x' => self.lex_hex_escape(escape_line, escape_col),
            'u' => self.lex_unicode_escape(escape_line, escape_col),
            other => Err(Diagnostic::new(
                format!("Invalid string escape '\\{other}'"),
                escape_line,
                escape_col,
            )),
        }
    }

    fn lex_hex_escape(&mut self, line: usize, column: usize) -> Result<char, Diagnostic> {
        let high = self.bump_required_hex_digit("Invalid hex string escape", line, column)?;
        let low = self.bump_required_hex_digit("Invalid hex string escape", line, column)?;
        let value = high * 16 + low;
        char::from_u32(value)
            .ok_or_else(|| Diagnostic::new("Invalid hex string escape scalar", line, column))
    }

    fn lex_unicode_escape(&mut self, line: usize, column: usize) -> Result<char, Diagnostic> {
        if self.is_eof() || self.peek() != '{' {
            return Err(Diagnostic::new(
                "Invalid unicode string escape",
                line,
                column,
            ));
        }
        self.bump();

        let mut digits = String::new();
        while !self.is_eof() && self.peek() != '}' {
            let ch = self.peek();
            if !ch.is_ascii_hexdigit() {
                return Err(Diagnostic::new(
                    "Invalid unicode string escape digit",
                    line,
                    column,
                ));
            }
            if digits.len() == 6 {
                return Err(Diagnostic::new(
                    "Invalid unicode string escape length",
                    line,
                    column,
                ));
            }
            digits.push(self.bump());
        }

        if self.is_eof() {
            return Err(Diagnostic::new(
                "Unterminated unicode string escape",
                line,
                column,
            ));
        }
        self.bump();

        if digits.is_empty() {
            return Err(Diagnostic::new(
                "Invalid unicode string escape",
                line,
                column,
            ));
        }

        let value = u32::from_str_radix(&digits, 16)
            .map_err(|_| Diagnostic::new("Invalid unicode string escape value", line, column))?;
        char::from_u32(value)
            .ok_or_else(|| Diagnostic::new("Invalid unicode string escape scalar", line, column))
    }

    fn bump_required_hex_digit(
        &mut self,
        message: &'static str,
        line: usize,
        column: usize,
    ) -> Result<u32, Diagnostic> {
        if self.is_eof() {
            return Err(Diagnostic::new(message, line, column));
        }
        let ch = self.bump();
        ch.to_digit(16)
            .ok_or_else(|| Diagnostic::new(message, line, column))
    }

    fn lex_number(&mut self) -> TokenKind {
        let mut s = String::new();

        if !self.is_eof() && self.peek() == '0' {
            if matches!(self.peek2(), Some('x' | 'X')) {
                s.push(self.bump()); // 0
                s.push(self.bump()); // x|X
                while !self.is_eof() {
                    let ch = self.peek();
                    if ch.is_ascii_hexdigit() || ch == '_' {
                        s.push(self.bump());
                    } else {
                        break;
                    }
                }
                return TokenKind::Number(s);
            }

            if matches!(self.peek2(), Some('b' | 'B')) {
                s.push(self.bump()); // 0
                s.push(self.bump()); // b|B
                while !self.is_eof() {
                    let ch = self.peek();
                    if ch == '0' || ch == '1' || ch == '_' {
                        s.push(self.bump());
                    } else {
                        break;
                    }
                }
                return TokenKind::Number(s);
            }

            if matches!(self.peek2(), Some('o' | 'O')) {
                s.push(self.bump()); // 0
                s.push(self.bump()); // o|O
                while !self.is_eof() {
                    let ch = self.peek();
                    if matches!(ch, '0'..='7' | '_') {
                        s.push(self.bump());
                    } else {
                        break;
                    }
                }
                return TokenKind::Number(s);
            }
        }

        let mut seen_decimal_dot = false;
        while !self.is_eof() {
            let ch = self.peek();
            if ch.is_ascii_digit() || ch == '_' {
                s.push(self.bump());
                continue;
            }

            if ch == '.' {
                let next_is_digit = self.peek2().map(|c| c.is_ascii_digit()).unwrap_or(false);
                if !seen_decimal_dot && next_is_digit {
                    seen_decimal_dot = true;
                    s.push(self.bump());
                    continue;
                }
            }

            break;
        }

        TokenKind::Number(s)
    }

    fn lex_identifier_or_keyword(&mut self) -> TokenKind {
        let mut ident = String::new();
        while !self.is_eof() && is_ident_continue(self.peek()) {
            ident.push(self.bump());
        }

        match ident.as_str() {
            "use" => TokenKind::KeywordUse,
            "import" => TokenKind::KeywordImport,
            "stc" => TokenKind::KeywordStc,
            "local" => TokenKind::KeywordLocal,
            "fn" => TokenKind::KeywordFn,
            "move" => TokenKind::KeywordMove,
            "probe" => TokenKind::KeywordProbe,
            "bind" => TokenKind::KeywordBind,
            "binary" => TokenKind::KeywordBinary,
            "raw" => TokenKind::KeywordRaw,
            "structure" => TokenKind::KeywordStructure,
            "str" => TokenKind::KeywordStr,
            "final" => TokenKind::KeywordFinal,
            "fa" => TokenKind::KeywordFa,
            "public" => TokenKind::KeywordPublic,
            "private" => TokenKind::KeywordPrivate,
            "internal" => TokenKind::KeywordInternal,
            "strictly" => TokenKind::KeywordStrictly,
            "return" => TokenKind::KeywordReturn,
            "dev" => TokenKind::KeywordDev,
            "if" => TokenKind::KeywordIf,
            "else" => TokenKind::KeywordElse,
            "while" => TokenKind::KeywordWhile,
            "ref" => TokenKind::KeywordRef,
            "main" => TokenKind::KeywordMain,
            "skkMain" => TokenKind::KeywordSkkMain,
            "true" => TokenKind::KeywordTrue,
            "false" => TokenKind::KeywordFalse,
            "ly" => TokenKind::KeywordLy,
            "cnt" => TokenKind::KeywordCnt,
            "cut" => TokenKind::KeywordCut,
            "anchor" => TokenKind::KeywordAnchor,
            "slot" => TokenKind::KeywordSlot,
            "pattern" => TokenKind::KeywordPattern,
            "mask" => TokenKind::KeywordMask,
            "window" => TokenKind::KeywordWindow,
            "signal" => TokenKind::KeywordSignal,
            "bus" => TokenKind::KeywordBus,
            "state" => TokenKind::KeywordState,
            "timing" => TokenKind::KeywordTiming,
            "emit" => TokenKind::KeywordEmit,
            "sample" => TokenKind::KeywordSample,
            "latch" => TokenKind::KeywordLatch,
            "assert" => TokenKind::KeywordAssert,
            "rupture" => TokenKind::KeywordRupture,
            "bridge" => TokenKind::KeywordBridge,
            "seal" => TokenKind::KeywordSeal,
            "reg" => TokenKind::KeywordReg,
            "flag" => TokenKind::KeywordFlag,
            "label" => TokenKind::KeywordLabel,
            "inst" => TokenKind::KeywordInst,
            _ => TokenKind::Identifier(ident),
        }
    }

    fn match_char(&mut self, c: char) -> bool {
        if !self.is_eof() && self.peek() == c {
            self.bump();
            true
        } else {
            false
        }
    }

    fn peek(&self) -> char {
        self.chars[self.pos]
    }

    fn peek2(&self) -> Option<char> {
        self.chars.get(self.pos + 1).copied()
    }

    fn bump(&mut self) -> char {
        let ch = self.chars[self.pos];
        self.pos += 1;
        if ch == '\n' {
            self.line += 1;
            self.col = 1;
        } else {
            self.col += 1;
        }
        ch
    }

    fn is_eof(&self) -> bool {
        self.pos >= self.chars.len()
    }
}

fn is_ident_start(c: char) -> bool {
    c.is_ascii_alphabetic() || c == '_'
}

fn is_ident_continue(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

#[cfg(test)]
mod tests {
    use super::{lex, TokenKind};

    #[test]
    fn lex_hex_number_literal_as_single_number_token() {
        let tokens = lex("addr 0x03F8;").expect("expected lexer to parse hex literal");
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "addr"));
        assert!(matches!(tokens[1].kind, TokenKind::Number(ref s) if s == "0x03F8"));
    }

    #[test]
    fn lex_binary_number_with_underscores_as_single_number_token() {
        let tokens = lex("mask 0b1010_0001;").expect("expected lexer to parse binary literal");
        assert!(matches!(tokens[0].kind, TokenKind::KeywordMask));
        assert!(matches!(tokens[1].kind, TokenKind::Number(ref s) if s == "0b1010_0001"));
    }

    #[test]
    fn lex_octal_number_with_underscores_as_single_number_token() {
        let tokens = lex("mode 0o7_55;").expect("expected lexer to parse octal literal");
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "mode"));
        assert!(matches!(tokens[1].kind, TokenKind::Number(ref s) if s == "0o7_55"));
    }

    #[test]
    fn lex_decimal_number_with_underscores_as_single_number_token() {
        let tokens =
            lex("width 10_000;").expect("expected lexer to parse decimal literal with underscores");
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "width"));
        assert!(matches!(tokens[1].kind, TokenKind::Number(ref s) if s == "10_000"));
    }

    #[test]
    fn lex_operator_surface_tokens() {
        let tokens = lex("! ~ % & ^ << >> >>> ++ -- += -= *=")
            .expect("expected lexer to parse operator surface");
        assert!(matches!(tokens[0].kind, TokenKind::Bang));
        assert!(matches!(tokens[1].kind, TokenKind::Tilde));
        assert!(matches!(tokens[2].kind, TokenKind::Percent));
        assert!(matches!(tokens[3].kind, TokenKind::Amp));
        assert!(matches!(tokens[4].kind, TokenKind::Caret));
        assert!(matches!(tokens[5].kind, TokenKind::ShiftLeft));
        assert!(matches!(tokens[6].kind, TokenKind::Shift2));
        assert!(matches!(tokens[7].kind, TokenKind::Shift3));
        assert!(matches!(tokens[8].kind, TokenKind::PlusPlus));
        assert!(matches!(tokens[9].kind, TokenKind::MinusMinus));
        assert!(matches!(tokens[10].kind, TokenKind::PlusEq));
        assert!(matches!(tokens[11].kind, TokenKind::MinusEq));
        assert!(matches!(tokens[12].kind, TokenKind::StarEq));
    }

    #[test]
    fn lex_import_keyword_token() {
        let tokens = lex("import system.net.core;")
            .expect("expected lexer to parse import keyword as dedicated token");
        assert!(matches!(tokens[0].kind, TokenKind::KeywordImport));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "system"));
    }

    #[test]
    fn lex_string_decodes_common_escapes_to_real_scalars() {
        let tokens = lex(r#""line\nquote\"slash\\tab\tzero\0amp\&query\?hash\#""#)
            .expect("expected lexer to decode common string escapes");
        assert!(
            matches!(tokens[0].kind, TokenKind::String(ref s) if s == "line\nquote\"slash\\tab\tzero\0amp&query?hash#")
        );
    }

    #[test]
    fn lex_block_comments_are_discarded_like_line_comments() {
        let tokens = lex("uno /* comentario\nmultilinea */ dos // cola\ntres")
            .expect("expected lexer to discard line and block comments");
        assert!(matches!(tokens[0].kind, TokenKind::Identifier(ref s) if s == "uno"));
        assert!(matches!(tokens[1].kind, TokenKind::Identifier(ref s) if s == "dos"));
        assert!(matches!(tokens[2].kind, TokenKind::Identifier(ref s) if s == "tres"));
    }

    #[test]
    fn lex_string_decodes_hex_and_unicode_escapes() {
        let tokens = lex(r#""A\x41\u{00F1}\u{1F642}""#)
            .expect("expected lexer to decode hex and unicode string escapes");
        assert!(matches!(tokens[0].kind, TokenKind::String(ref s) if s == "AAñ🙂"));
    }

    #[test]
    fn lex_char_decodes_one_byte_literal() {
        let tokens = lex(r#"'A' '\n' '\x41'"#).expect("expected lexer to decode char literals");
        assert!(matches!(tokens[0].kind, TokenKind::Char('A')));
        assert!(matches!(tokens[1].kind, TokenKind::Char('\n')));
        assert!(matches!(tokens[2].kind, TokenKind::Char('A')));
    }

    #[test]
    fn lex_char_rejects_multi_char_or_wide_unicode() {
        assert!(lex("'AB'").is_err());
        assert!(lex("'🙂'").is_err());
    }

    #[test]
    fn lex_string_rejects_unknown_escape() {
        assert!(lex(r#""bad\q""#).is_err());
    }

    #[test]
    fn lex_string_rejects_bad_hex_escape() {
        assert!(lex(r#""bad\x4Z""#).is_err());
    }

    #[test]
    fn lex_string_rejects_bad_unicode_escape() {
        assert!(lex(r#""bad\u{110000}""#).is_err());
    }
}
