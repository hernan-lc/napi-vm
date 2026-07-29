#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    Number(f64),
    String(String),
    Identifier(String),
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    PlusPlus,
    MinusMinus,
    Equal,
    PlusEqual,
    MinusEqual,
    StarEqual,
    SlashEqual,
    EqualEqual,
    NotEqual,
    EqualEqualEqual,
    NotEqualEqual,
    Less,
    Greater,
    LessEqual,
    GreaterEqual,
    And,
    Or,
    Not,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Semicolon,
    Comma,
    Dot,
    Colon,
    Question,
    Arrow,
    DotDotDot,
    KwVar,
    KwLet,
    KwConst,
    KwFunction,
    KwReturn,
    KwIf,
    KwElse,
    KwFor,
    KwWhile,
    KwDo,
    KwSwitch,
    KwCase,
    KwDefault,
    KwBreak,
    KwContinue,
    KwClass,
    KwExtends,
    KwNew,
    KwThis,
    KwSuper,
    KwImport,
    KwExport,
    KwFrom,
    KwAs,
    KwAsync,
    KwAwait,
    KwTry,
    KwCatch,
    KwFinally,
    KwThrow,
    KwTypeof,
    KwInstanceof,
    KwIn,
    KwOf,
    KwTrue,
    KwFalse,
    KwNull,
    KwUndefined,
    KwDelete,
    KwVoid,
    KwStatic,
    KwGet,
    KwSet,
    KwConstructor,
    EOF,
}

pub struct Lexer {
    src: Vec<char>,
    pos: usize,
}

impl Lexer {
    pub fn new(s: &str) -> Self {
        Self { src: s.chars().collect(), pos: 0 }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut toks = Vec::new();
        while self.pos < self.src.len() {
            self.skip_ws();
            if self.pos >= self.src.len() {
                break;
            }
            if let Some(t) = self.next() {
                toks.push(t);
            }
        }
        toks.push(Token::EOF);
        toks
    }

    fn skip_ws(&mut self) {
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c.is_whitespace() {
                self.pos += 1;
            } else if c == '/' && self.pos + 1 < self.src.len() {
                let n = self.src[self.pos + 1];
                if n == '/' {
                    self.pos += 2;
                    while self.pos < self.src.len() && self.src[self.pos] != '\n' {
                        self.pos += 1;
                    }
                } else if n == '*' {
                    self.pos += 2;
                    while self.pos + 1 < self.src.len() {
                        if self.src[self.pos] == '*' && self.src[self.pos + 1] == '/' {
                            self.pos += 2;
                            break;
                        }
                        self.pos += 1;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    fn next(&mut self) -> Option<Token> {
        let c = *self.src.get(self.pos)?;
        Some(match c {
            '(' => { self.pos += 1; Token::LParen }
            ')' => { self.pos += 1; Token::RParen }
            '{' => { self.pos += 1; Token::LBrace }
            '}' => { self.pos += 1; Token::RBrace }
            '[' => { self.pos += 1; Token::LBracket }
            ']' => { self.pos += 1; Token::RBracket }
            ';' => { self.pos += 1; Token::Semicolon }
            ',' => { self.pos += 1; Token::Comma }
            ':' => { self.pos += 1; Token::Colon }
            '?' => { self.pos += 1; Token::Question }
            '.' => {
                if self.pos + 2 < self.src.len() && self.src[self.pos + 1] == '.' && self.src[self.pos + 2] == '.' {
                    self.pos += 3;
                    Token::DotDotDot
                } else {
                    self.pos += 1;
                    Token::Dot
                }
            }
            '+' => match self.src.get(self.pos + 1) {
                Some('+') => { self.pos += 2; Token::PlusPlus }
                Some('=') => { self.pos += 2; Token::PlusEqual }
                _ => { self.pos += 1; Token::Plus }
            },
            '-' => match self.src.get(self.pos + 1) {
                Some('-') => { self.pos += 2; Token::MinusMinus }
                Some('=') => { self.pos += 2; Token::MinusEqual }
                _ => { self.pos += 1; Token::Minus }
            },
            '*' => match self.src.get(self.pos + 1) {
                Some('=') => { self.pos += 2; Token::StarEqual }
                _ => { self.pos += 1; Token::Star }
            },
            '/' => match self.src.get(self.pos + 1) {
                Some('=') => { self.pos += 2; Token::SlashEqual }
                _ => { self.pos += 1; Token::Slash }
            },
            '%' => { self.pos += 1; Token::Percent }
            '=' => match (self.src.get(self.pos + 1), self.src.get(self.pos + 2)) {
                (Some('='), Some('=')) => { self.pos += 3; Token::EqualEqualEqual }
                (Some('='), _) => { self.pos += 2; Token::EqualEqual }
                (Some('>'), _) => { self.pos += 2; Token::Arrow }
                _ => { self.pos += 1; Token::Equal }
            },
            '!' => match (self.src.get(self.pos + 1), self.src.get(self.pos + 2)) {
                (Some('='), Some('=')) => { self.pos += 3; Token::NotEqualEqual }
                (Some('='), _) => { self.pos += 2; Token::NotEqual }
                _ => { self.pos += 1; Token::Not }
            },
            '<' => match self.src.get(self.pos + 1) {
                Some('<') => { self.pos += 2; Token::Less }
                Some('=') => { self.pos += 2; Token::LessEqual }
                _ => { self.pos += 1; Token::Less }
            },
            '>' => match self.src.get(self.pos + 1) {
                Some('>') => { self.pos += 2; Token::Greater }
                Some('=') => { self.pos += 2; Token::GreaterEqual }
                _ => { self.pos += 1; Token::Greater }
            },
            '&' => match self.src.get(self.pos + 1) {
                Some('&') => { self.pos += 2; Token::And }
                _ => { self.pos += 1; Token::And }
            },
            '|' => match self.src.get(self.pos + 1) {
                Some('|') => { self.pos += 2; Token::Or }
                _ => { self.pos += 1; Token::Or }
            },
            '"' | '\'' => self.read_str(c),
            c if c.is_ascii_digit() => self.read_num(),
            c if c.is_ascii_alphabetic() || c == '_' || c == '$' => self.read_ident(),
            _ => { self.pos += 1; return None; }
        })
    }

    fn read_str(&mut self, q: char) -> Token {
        self.pos += 1;
        let mut s = String::new();
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if c == q {
                self.pos += 1;
                break;
            }
            if c == '\\' && self.pos + 1 < self.src.len() {
                self.pos += 1;
                match self.src[self.pos] {
                    'n' => s.push('\n'),
                    't' => s.push('\t'),
                    'r' => s.push('\r'),
                    '\\' => s.push('\\'),
                    '"' => s.push('"'),
                    '\'' => s.push('\''),
                    '0' => s.push('\0'),
                    o => s.push(o),
                }
            } else {
                s.push(c);
            }
            self.pos += 1;
        }
        Token::String(s)
    }

    fn read_num(&mut self) -> Token {
        let s = self.pos;
        while self.pos < self.src.len() && (self.src[self.pos].is_ascii_digit() || self.src[self.pos] == '_') {
            self.pos += 1;
        }
        if self.pos < self.src.len() && self.src[self.pos] == '.' {
            self.pos += 1;
            while self.pos < self.src.len() && (self.src[self.pos].is_ascii_digit() || self.src[self.pos] == '_') {
                self.pos += 1;
            }
        }
        let n: String = self.src[s..self.pos].iter().filter(|c| **c != '_').collect();
        Token::Number(n.parse().unwrap_or(0.0))
    }

    fn read_ident(&mut self) -> Token {
        let s = self.pos;
        while self.pos < self.src.len() {
            let c = self.src[self.pos];
            if !c.is_ascii_alphanumeric() && c != '_' && c != '$' {
                break;
            }
            self.pos += 1;
        }
        let i: String = self.src[s..self.pos].iter().collect();
        match i.as_str() {
            "var" => Token::KwVar,
            "let" => Token::KwLet,
            "const" => Token::KwConst,
            "function" => Token::KwFunction,
            "return" => Token::KwReturn,
            "if" => Token::KwIf,
            "else" => Token::KwElse,
            "for" => Token::KwFor,
            "while" => Token::KwWhile,
            "do" => Token::KwDo,
            "switch" => Token::KwSwitch,
            "case" => Token::KwCase,
            "default" => Token::KwDefault,
            "break" => Token::KwBreak,
            "continue" => Token::KwContinue,
            "class" => Token::KwClass,
            "extends" => Token::KwExtends,
            "new" => Token::KwNew,
            "this" => Token::KwThis,
            "super" => Token::KwSuper,
            "import" => Token::KwImport,
            "export" => Token::KwExport,
            "from" => Token::KwFrom,
            "as" => Token::KwAs,
            "async" => Token::KwAsync,
            "await" => Token::KwAwait,
            "try" => Token::KwTry,
            "catch" => Token::KwCatch,
            "finally" => Token::KwFinally,
            "throw" => Token::KwThrow,
            "typeof" => Token::KwTypeof,
            "instanceof" => Token::KwInstanceof,
            "in" => Token::KwIn,
            "of" => Token::KwOf,
            "true" => Token::KwTrue,
            "false" => Token::KwFalse,
            "null" => Token::KwNull,
            "undefined" => Token::KwUndefined,
            "delete" => Token::KwDelete,
            "void" => Token::KwVoid,
            "static" => Token::KwStatic,
            "get" => Token::KwGet,
            "set" => Token::KwSet,
            "constructor" => Token::KwConstructor,
            _ => Token::Identifier(i),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_numbers() {
        let mut lex = Lexer::new("42 3.14 1_000");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::Number(42.0));
        assert_eq!(toks[1], Token::Number(3.14));
        assert_eq!(toks[2], Token::Number(1000.0));
    }

    #[test]
    fn test_strings() {
        let mut lex = Lexer::new(r#""hello" 'world'"#);
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::String("hello".to_string()));
        assert_eq!(toks[1], Token::String("world".to_string()));
    }

    #[test]
    fn test_string_escapes() {
        let mut lex = Lexer::new(r#""hello\nworld""#);
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::String("hello\nworld".to_string()));
    }

    #[test]
    fn test_operators() {
        let mut lex = Lexer::new("+ - * / % ++ -- == != === !== < > <= >=");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::Plus);
        assert_eq!(toks[1], Token::Minus);
        assert_eq!(toks[2], Token::Star);
        assert_eq!(toks[3], Token::Slash);
        assert_eq!(toks[4], Token::Percent);
        assert_eq!(toks[5], Token::PlusPlus);
        assert_eq!(toks[6], Token::MinusMinus);
        assert_eq!(toks[7], Token::EqualEqual);
        assert_eq!(toks[8], Token::NotEqual);
        assert_eq!(toks[9], Token::EqualEqualEqual);
        assert_eq!(toks[10], Token::NotEqualEqual);
        assert_eq!(toks[11], Token::Less);
        assert_eq!(toks[12], Token::Greater);
        assert_eq!(toks[13], Token::LessEqual);
        assert_eq!(toks[14], Token::GreaterEqual);
    }

    #[test]
    fn test_keywords() {
        let mut lex = Lexer::new("var let const function return if else for while");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::KwVar);
        assert_eq!(toks[1], Token::KwLet);
        assert_eq!(toks[2], Token::KwConst);
        assert_eq!(toks[3], Token::KwFunction);
        assert_eq!(toks[4], Token::KwReturn);
        assert_eq!(toks[5], Token::KwIf);
        assert_eq!(toks[6], Token::KwElse);
        assert_eq!(toks[7], Token::KwFor);
        assert_eq!(toks[8], Token::KwWhile);
    }

    #[test]
    fn test_identifiers() {
        let mut lex = Lexer::new("foo _bar $baz myVar");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::Identifier("foo".to_string()));
        assert_eq!(toks[1], Token::Identifier("_bar".to_string()));
        assert_eq!(toks[2], Token::Identifier("$baz".to_string()));
        assert_eq!(toks[3], Token::Identifier("myVar".to_string()));
    }

    #[test]
    fn test_comments() {
        let mut lex = Lexer::new("1 // comment\n2 /* block */ 3");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::Number(1.0));
        assert_eq!(toks[1], Token::Number(2.0));
        assert_eq!(toks[2], Token::Number(3.0));
    }

    #[test]
    fn test_arrow() {
        let mut lex = Lexer::new("=>");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::Arrow);
    }

    #[test]
    fn test_spread() {
        let mut lex = Lexer::new("...");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::DotDotDot);
    }

    #[test]
    fn test_compound_assignment() {
        let mut lex = Lexer::new("+= -= *= /=");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::PlusEqual);
        assert_eq!(toks[1], Token::MinusEqual);
        assert_eq!(toks[2], Token::StarEqual);
        assert_eq!(toks[3], Token::SlashEqual);
    }

    #[test]
    fn test_punctuation() {
        let mut lex = Lexer::new("( ) { } [ ] ; , . : ?");
        let toks = lex.tokenize();
        assert_eq!(toks[0], Token::LParen);
        assert_eq!(toks[1], Token::RParen);
        assert_eq!(toks[2], Token::LBrace);
        assert_eq!(toks[3], Token::RBrace);
        assert_eq!(toks[4], Token::LBracket);
        assert_eq!(toks[5], Token::RBracket);
        assert_eq!(toks[6], Token::Semicolon);
        assert_eq!(toks[7], Token::Comma);
        assert_eq!(toks[8], Token::Dot);
        assert_eq!(toks[9], Token::Colon);
        assert_eq!(toks[10], Token::Question);
    }
}
