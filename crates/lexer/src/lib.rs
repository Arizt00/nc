#[derive(Debug, PartialEq, Clone)]
pub enum Token {
    // Keywords
    Stc,
    Fn,
    Public,
    Return,

    // Types
    Str,

    // Identifiers
    Identifier(String),

    // Annotations
    Annotation(String), // @system

    // Literals
    StringLiteral(String),

    // Symbols
    Colon,
    Semicolon,
    Arrow, // ->
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,

    // Special
    EOF,
}

pub struct Lexer {
    input: Vec<char>,
    position: usize,
}

impl Lexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
        }
    }

    fn current(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn read_identifier(&mut self) -> String {
        let mut result = String::new();

        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    fn read_string(&mut self) -> String {
        let mut result = String::new();
        self.advance(); // skip opening quote

        while let Some(c) = self.current() {
            if c == '"' {
                self.advance();
                break;
            }
            result.push(c);
            self.advance();
        }

        result
    }

    fn read_annotation(&mut self) -> String {
        self.advance(); // skip '@'
        let mut result = String::new();

        while let Some(c) = self.current() {
            if c.is_alphanumeric() || c == '_' {
                result.push(c);
                self.advance();
            } else {
                break;
            }
        }

        result
    }

    pub fn next_token(&mut self) -> Token {
        self.skip_whitespace();

        match self.current() {
            Some(':') => {
                self.advance();
                Token::Colon
            }
            Some(';') => {
                self.advance();
                Token::Semicolon
            }
            Some('(') => {
                self.advance();
                Token::LParen
            }
            Some(')') => {
                self.advance();
                Token::RParen
            }
            Some('{') => {
                self.advance();
                Token::LBrace
            }
            Some('}') => {
                self.advance();
                Token::RBrace
            }
            Some('[') => {
                self.advance();
                Token::LBracket
            }
            Some(']') => {
                self.advance();
                Token::RBracket
            }
            Some('-') => {
                self.advance();
                if self.current() == Some('>') {
                    self.advance();
                    Token::Arrow
                } else {
                    Token::EOF
                }
            }
            Some('"') => {
                let s = self.read_string();
                Token::StringLiteral(s)
            }
            Some('@') => {
                let ann = self.read_annotation();
                Token::Annotation(ann)
            }
            Some(c) if c.is_alphabetic() => {
                let ident = self.read_identifier();

                match ident.as_str() {
                    "stc" => Token::Stc,
                    "fn" => Token::Fn,
                    "public" => Token::Public,
                    "return" => Token::Return,
                    "str" => Token::Str,
                    _ => Token::Identifier(ident),
                }
            }
            None => Token::EOF,
            _ => {
                self.advance();
                self.next_token()
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let input = r#"
            helloWorld[@system] {
                stc:
                    mensaje: str;

                fn:
                    iniciar() -> str {
                        public:
                            return "Hello World";
                    }
            }
        "#;

        let mut lexer = Lexer::new(input);
        let mut tokens = Vec::new();

        loop {
            let tok = lexer.next_token();
            tokens.push(tok.clone());

            if tok == Token::EOF {
                break;
            }
        }

        assert_eq!(
            tokens,
            vec![
                Token::Identifier("helloWorld".into()),
                Token::LBracket,
                Token::Annotation("system".into()),
                Token::RBracket,
                Token::LBrace,
                Token::Stc,
                Token::Colon,
                Token::Identifier("mensaje".into()),
                Token::Colon,
                Token::Str,
                Token::Semicolon,
                Token::Fn,
                Token::Colon,
                Token::Identifier("iniciar".into()),
                Token::LParen,
                Token::RParen,
                Token::Arrow,
                Token::Str,
                Token::LBrace,
                Token::Public,
                Token::Colon,
                Token::Return,
                Token::StringLiteral("Hello World".into()),
                Token::Semicolon,
                Token::RBrace,
                Token::RBrace,
                Token::EOF,
            ]
        );
    }
}
