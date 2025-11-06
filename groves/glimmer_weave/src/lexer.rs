//! # Lexer Module
//!
//! Tokenizes Glimmer-Weave source code into a stream of tokens.
//!
//! The lexer handles:
//! - Keywords and identifiers
//! - Numeric and string literals
//! - Operators and delimiters
//! - Comments (lines starting with `#`)
//! - Significant newlines
//!
//! ## Example
//!
//! ```rust,ignore
//! use glimmer_weave::lexer::Lexer;
//!
//! let source = "bind x to 42";
//! let mut lexer = Lexer::new(source);
//!
//! while let Some(token) = lexer.next_token() {
//!     println!("{:?}", token);
//! }
//! ```

use alloc::string::String;
use alloc::vec::Vec;
use crate::token::{Span, Token};

/// Lexer state for tokenizing Glimmer-Weave source code
pub struct Lexer {
    /// Source code as character array
    input: Vec<char>,
    /// Current position in input
    position: usize,
    /// Current character
    current_char: Option<char>,
    /// Current line number (for error reporting)
    line: usize,
    /// Current column number (for error reporting)
    column: usize,
}

impl Lexer {
    /// Create a new lexer for the given source code
    pub fn new(input: &str) -> Self {
        let chars: Vec<char> = input.chars().collect();
        let current_char = chars.get(0).copied();
        Lexer {
            input: chars,
            position: 0,
            current_char,
            line: 1,
            column: 1,
        }
    }

    /// Get current position as a Span
    pub fn span(&self) -> Span {
        Span::new(self.line, self.column)
    }

    /// Advance to the next character
    fn advance(&mut self) {
        if let Some('\n') = self.current_char {
            self.line += 1;
            self.column = 1;
        } else {
            self.column += 1;
        }

        self.position += 1;
        self.current_char = self.input.get(self.position).copied();
    }

    /// Peek at the next character without consuming
    fn peek(&self) -> Option<char> {
        self.input.get(self.position + 1).copied()
    }

    /// Skip whitespace (but NOT newlines - they're significant)
    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char {
            if c == ' ' || c == '\t' || c == '\r' {
                self.advance();
            } else {
                break;
            }
        }
    }

    /// Skip comment (from `#` to end of line)
    fn skip_comment(&mut self) {
        // Skip until newline or EOF
        while let Some(c) = self.current_char {
            if c == '\n' {
                break;
            }
            self.advance();
        }
    }

    /// Read a string literal (enclosed in double quotes)
    fn read_string(&mut self) -> Token {
        // Skip opening quote
        self.advance();

        let mut result = String::new();

        while let Some(c) = self.current_char {
            if c == '"' {
                // Closing quote
                self.advance();
                return Token::Text(result);
            } else if c == '\\' {
                // Escape sequence
                self.advance();
                match self.current_char {
                    Some('n') => result.push('\n'),
                    Some('t') => result.push('\t'),
                    Some('r') => result.push('\r'),
                    Some('\\') => result.push('\\'),
                    Some('"') => result.push('"'),
                    Some(c) => {
                        // Unknown escape, keep the backslash and character
                        result.push('\\');
                        result.push(c);
                    }
                    None => {
                        result.push('\\');
                        break;
                    }
                }
                self.advance();
            } else {
                result.push(c);
                self.advance();
            }
        }

        // Unterminated string - return what we have
        Token::Text(result)
    }

    /// Read a numeric literal (integer or float)
    fn read_number(&mut self) -> Token {
        let mut num_str = String::new();

        // Read digits before decimal point
        while let Some(c) = self.current_char {
            if c.is_ascii_digit() {
                num_str.push(c);
                self.advance();
            } else {
                break;
            }
        }

        // Check for decimal point
        if self.current_char == Some('.') && self.peek().map_or(false, |c| c.is_ascii_digit()) {
            num_str.push('.');
            self.advance();

            // Read digits after decimal point
            while let Some(c) = self.current_char {
                if c.is_ascii_digit() {
                    num_str.push(c);
                    self.advance();
                } else {
                    break;
                }
            }
        }

        // Parse as f64
        let value = num_str.parse::<f64>().unwrap_or(0.0);
        Token::Number(value)
    }

    /// Read an identifier or keyword
    fn read_identifier_or_keyword(&mut self) -> Token {
        let start = self.position;

        // Read alphanumeric characters and underscores
        while let Some(c) = self.current_char {
            if c.is_alphanumeric() || c == '_' {
                self.advance();
            } else {
                break;
            }
        }

        let text: String = self.input[start..self.position].iter().collect();

        // Check for multi-word keywords (e.g., "is not")
        if text == "is" && self.current_char == Some(' ') {
            let saved_pos = self.position;
            let saved_char = self.current_char;
            let saved_line = self.line;
            let saved_col = self.column;

            // Try to read "not"
            self.skip_whitespace();
            if let Some(c) = self.current_char {
                if c.is_alphabetic() {
                    let start2 = self.position;
                    while let Some(c2) = self.current_char {
                        if c2.is_alphanumeric() || c2 == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let text2: String = self.input[start2..self.position].iter().collect();
                    if text2 == "not" {
                        return Token::IsNot;
                    }
                }
            }

            // Restore position if not "is not"
            self.position = saved_pos;
            self.current_char = saved_char;
            self.line = saved_line;
            self.column = saved_col;
        }

        // Check for "greater than"
        if text == "greater" && self.current_char == Some(' ') {
            let saved_pos = self.position;
            let saved_char = self.current_char;
            let saved_line = self.line;
            let saved_col = self.column;

            self.skip_whitespace();
            if let Some(c) = self.current_char {
                if c.is_alphabetic() {
                    let start2 = self.position;
                    while let Some(c2) = self.current_char {
                        if c2.is_alphanumeric() || c2 == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let text2: String = self.input[start2..self.position].iter().collect();
                    if text2 == "than" {
                        return Token::GreaterThan;
                    }
                }
            }

            self.position = saved_pos;
            self.current_char = saved_char;
            self.line = saved_line;
            self.column = saved_col;
        }

        // Check for "less than"
        if text == "less" && self.current_char == Some(' ') {
            let saved_pos = self.position;
            let saved_char = self.current_char;
            let saved_line = self.line;
            let saved_col = self.column;

            self.skip_whitespace();
            if let Some(c) = self.current_char {
                if c.is_alphabetic() {
                    let start2 = self.position;
                    while let Some(c2) = self.current_char {
                        if c2.is_alphanumeric() || c2 == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let text2: String = self.input[start2..self.position].iter().collect();
                    if text2 == "than" {
                        return Token::LessThan;
                    }
                }
            }

            self.position = saved_pos;
            self.current_char = saved_char;
            self.line = saved_line;
            self.column = saved_col;
        }

        // Check for "at least"
        if text == "at" && self.current_char == Some(' ') {
            let saved_pos = self.position;
            let saved_char = self.current_char;
            let saved_line = self.line;
            let saved_col = self.column;

            self.skip_whitespace();
            if let Some(c) = self.current_char {
                if c.is_alphabetic() {
                    let start2 = self.position;
                    while let Some(c2) = self.current_char {
                        if c2.is_alphanumeric() || c2 == '_' {
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let text2: String = self.input[start2..self.position].iter().collect();
                    if text2 == "least" {
                        return Token::AtLeast;
                    } else if text2 == "most" {
                        return Token::AtMost;
                    }
                }
            }

            self.position = saved_pos;
            self.current_char = saved_char;
            self.line = saved_line;
            self.column = saved_col;
        }

        // Match keyword
        match text.as_str() {
            "bind" => Token::Bind,
            "weave" => Token::Weave,
            "set" => Token::Set,
            "to" => Token::To,
            "as" => Token::As,
            "should" => Token::Should,
            "then" => Token::Then,
            "end" => Token::End,
            "otherwise" => Token::Otherwise,
            "for" => Token::For,
            "each" => Token::Each,
            "in" => Token::In,
            "range" => Token::Range,
            "whilst" => Token::Whilst,
            "chant" => Token::Chant,
            "yield" => Token::Yield,
            "form" => Token::Form,
            "seek" => Token::Seek,
            "where" => Token::Where,
            "by" => Token::By,
            "filter" => Token::Filter,
            "sort" => Token::Sort,
            "take" => Token::Take,
            "first" => Token::First,
            "last" => Token::Last,
            "attempt" => Token::Attempt,
            "harmonize" => Token::Harmonize,
            "on" => Token::On,
            "match" => Token::Match,
            "when" => Token::When,
            "with" => Token::With,
            "request" => Token::Request,
            "justification" => Token::Justification,
            "Triumph" => Token::Triumph,
            "Mishap" => Token::Mishap,
            "Present" => Token::Present,
            "Absent" => Token::Absent,
            "after" => Token::After,
            "before" => Token::Before,
            "descending" => Token::Descending,
            "ascending" => Token::Ascending,
            "true" => Token::Truth(true),
            "false" => Token::Truth(false),
            "nothing" => Token::Nothing,
            "is" => Token::Is,
            "and" => Token::And,
            "or" => Token::Or,
            "not" => Token::Not,
            _ => Token::Ident(text),
        }
    }

    /// Get the next token from the input
    pub fn next_token(&mut self) -> Token {
        // Skip whitespace (but not newlines)
        self.skip_whitespace();

        // Match current character
        match self.current_char {
            None => Token::Eof,

            Some('\n') => {
                self.advance();
                Token::Newline
            }

            Some('#') => {
                // Comment - skip to end of line
                self.skip_comment();
                self.next_token() // Get next real token
            }

            Some('"') => self.read_string(),

            Some(c) if c.is_ascii_digit() => self.read_number(),

            Some(c) if c.is_alphabetic() || c == '_' => self.read_identifier_or_keyword(),

            Some('+') => {
                self.advance();
                Token::Plus
            }

            Some('-') => {
                self.advance();
                if self.current_char == Some('>') {
                    self.advance();
                    Token::Arrow
                } else {
                    Token::Minus
                }
            }

            Some('*') => {
                self.advance();
                Token::Star
            }

            Some('/') => {
                self.advance();
                Token::Slash
            }

            Some('%') => {
                self.advance();
                Token::Percent
            }

            // < and > are ONLY for generic type syntax (e.g., List<Number>)
            // For comparisons, use natural language: "greater than", "less than", etc.
            Some('<') => {
                self.advance();
                Token::LeftAngle
            }

            Some('>') => {
                self.advance();
                Token::RightAngle
            }

            Some('|') => {
                self.advance();
                Token::Pipe
            }

            Some('(') => {
                self.advance();
                Token::LeftParen
            }

            Some(')') => {
                self.advance();
                Token::RightParen
            }

            Some('[') => {
                self.advance();
                Token::LeftBracket
            }

            Some(']') => {
                self.advance();
                Token::RightBracket
            }

            Some('{') => {
                self.advance();
                Token::LeftBrace
            }

            Some('}') => {
                self.advance();
                Token::RightBrace
            }

            Some(',') => {
                self.advance();
                Token::Comma
            }

            Some(':') => {
                self.advance();
                Token::Colon
            }

            Some('.') => {
                self.advance();
                Token::Dot
            }

            Some(c) => {
                // Unknown character - skip it and try next
                self.advance();
                self.next_token()
            }
        }
    }

    /// Tokenize entire input into a vector
    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        loop {
            let token = self.next_token();
            let is_eof = matches!(token, Token::Eof);
            tokens.push(token);

            if is_eof {
                break;
            }
        }

        tokens
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keywords() {
        let source = "bind weave set to should then end";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Bind);
        assert_eq!(tokens[1], Token::Weave);
        assert_eq!(tokens[2], Token::Set);
        assert_eq!(tokens[3], Token::To);
        assert_eq!(tokens[4], Token::Should);
        assert_eq!(tokens[5], Token::Then);
        assert_eq!(tokens[6], Token::End);
    }

    #[test]
    fn test_numbers() {
        let source = "42 3.14 0 100.5";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Number(42.0));
        assert_eq!(tokens[1], Token::Number(3.14));
        assert_eq!(tokens[2], Token::Number(0.0));
        assert_eq!(tokens[3], Token::Number(100.5));
    }

    #[test]
    fn test_strings() {
        let source = r#""hello" "world" "test\nstring""#;
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Text("hello".to_string()));
        assert_eq!(tokens[1], Token::Text("world".to_string()));
        assert_eq!(tokens[2], Token::Text("test\nstring".to_string()));
    }

    #[test]
    fn test_identifiers() {
        let source = "foo bar_baz _test name123";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Ident("foo".to_string()));
        assert_eq!(tokens[1], Token::Ident("bar_baz".to_string()));
        assert_eq!(tokens[2], Token::Ident("_test".to_string()));
        assert_eq!(tokens[3], Token::Ident("name123".to_string()));
    }

    #[test]
    fn test_operators() {
        let source = "+ - * / % > < |";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Plus);
        assert_eq!(tokens[1], Token::Minus);
        assert_eq!(tokens[2], Token::Star);
        assert_eq!(tokens[3], Token::Slash);
        assert_eq!(tokens[4], Token::Percent);
        // < and > are now only for type generics (List<Number>)
        assert_eq!(tokens[5], Token::RightAngle);
        assert_eq!(tokens[6], Token::LeftAngle);
        assert_eq!(tokens[7], Token::Pipe);
    }

    #[test]
    fn test_delimiters() {
        let source = "( ) [ ] { } , : .";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::LeftParen);
        assert_eq!(tokens[1], Token::RightParen);
        assert_eq!(tokens[2], Token::LeftBracket);
        assert_eq!(tokens[3], Token::RightBracket);
        assert_eq!(tokens[4], Token::LeftBrace);
        assert_eq!(tokens[5], Token::RightBrace);
        assert_eq!(tokens[6], Token::Comma);
        assert_eq!(tokens[7], Token::Colon);
        assert_eq!(tokens[8], Token::Dot);
    }

    #[test]
    fn test_is_not_keyword() {
        let source = "x is not y";
        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        assert_eq!(tokens[0], Token::Ident("x".to_string()));
        assert_eq!(tokens[1], Token::IsNot);
        assert_eq!(tokens[2], Token::Ident("y".to_string()));
    }

    #[test]
    fn test_simple_program() {
        let source = r#"bind x to 42
bind name to "Elara"
should x greater than 40 then
    VGA.write("Large number")
end"#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        // bind x to 42
        assert_eq!(tokens[0], Token::Bind);
        assert_eq!(tokens[1], Token::Ident("x".to_string()));
        assert_eq!(tokens[2], Token::To);
        assert_eq!(tokens[3], Token::Number(42.0));
        assert_eq!(tokens[4], Token::Newline);

        // bind name to "Elara"
        assert_eq!(tokens[5], Token::Bind);
        assert_eq!(tokens[6], Token::Ident("name".to_string()));
        assert_eq!(tokens[7], Token::To);
        assert_eq!(tokens[8], Token::Text("Elara".to_string()));
        assert_eq!(tokens[9], Token::Newline);

        // should x greater than 40 then
        assert_eq!(tokens[10], Token::Should);
        assert_eq!(tokens[11], Token::Ident("x".to_string()));
        assert_eq!(tokens[12], Token::GreaterThan);
        assert_eq!(tokens[13], Token::Number(40.0));
        assert_eq!(tokens[14], Token::Then);
    }

    #[test]
    fn test_comments() {
        let source = r#"# This is a comment
bind x to 42  # inline comment
# Another comment"#;

        let mut lexer = Lexer::new(source);
        let tokens = lexer.tokenize();

        // Comments should be skipped
        assert_eq!(tokens[0], Token::Newline); // After first comment
        assert_eq!(tokens[1], Token::Bind);
        assert_eq!(tokens[2], Token::Ident("x".to_string()));
        assert_eq!(tokens[3], Token::To);
        assert_eq!(tokens[4], Token::Number(42.0));
    }
}
