//! # Token Module
//!
//! Defines all tokens recognized by the Glimmer-Weave lexer.
//!
//! Glimmer-Weave uses natural language-inspired keywords like `bind`, `weave`,
//! `should`, `chant`, and `seek` to create a readable scripting experience.

use alloc::string::String;

/// Position information for error reporting
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    pub fn new(line: usize, column: usize) -> Self {
        Span { line, column }
    }
}

/// All tokens recognized by Glimmer-Weave
#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    // === Keywords ===
    /// `bind` - Immutable variable binding
    Bind,
    /// `weave` - Mutable variable declaration
    Weave,
    /// `set` - Mutation operator
    Set,
    /// `to` - Assignment operator
    To,
    /// `as` - Type annotation or alias
    As,

    /// `should` - Conditional keyword (if)
    Should,
    /// `then` - Begins conditional/loop body
    Then,
    /// `end` - Closes block
    End,
    /// `otherwise` - Else clause
    Otherwise,

    /// `for` - Bounded loop keyword
    For,
    /// `each` - Iteration keyword
    Each,
    /// `in` - Range/collection membership
    In,
    /// `range` - Range constructor
    Range,
    /// `whilst` - Unbounded loop keyword (while)
    Whilst,

    /// `chant` - Function declaration
    Chant,
    /// `yield` - Return statement
    Yield,

    /// `form` - Struct/type declaration
    Form,

    /// `seek` - Query/search keyword
    Seek,
    /// `where` - Query filter
    Where,
    /// `by` - Sort/filter criterion
    By,
    /// `filter` - Filter operation
    Filter,
    /// `sort` - Sort operation
    Sort,
    /// `take` - Limit results
    Take,
    /// `first` - Get first element
    First,
    /// `last` - Get last element
    Last,

    /// `attempt` - Try block
    Attempt,
    /// `harmonize` - Catch/handle errors
    Harmonize,
    /// `on` - Error type matcher
    On,

    /// `match` - Pattern matching
    Match,
    /// `when` - Match arm
    When,
    /// `with` - Match subject

    With,

    /// `request` - Capability request
    Request,
    /// `justification` - Capability justification
    Justification,

    /// `Triumph` - Successful Outcome constructor
    Triumph,
    /// `Mishap` - Failed Outcome constructor
    Mishap,
    /// `Present` - Present Maybe constructor
    Present,
    /// `Absent` - Absent Maybe constructor
    Absent,

    /// `after` - Temporal comparison
    After,
    /// `before` - Temporal comparison
    Before,
    /// `descending` - Sort order
    Descending,
    /// `ascending` - Sort order
    Ascending,

    // === Literals ===
    /// Numeric literal (integer or float)
    Number(f64),
    /// String literal
    Text(String),
    /// Boolean literal (`true` or `false`)
    Truth(bool),
    /// Null/void value
    Nothing,

    // === Identifiers ===
    /// Variable/function name
    Ident(String),

    // === Operators ===
    /// `+` addition
    Plus,
    /// `-` subtraction
    Minus,
    /// `*` multiplication
    Star,
    /// `/` division
    Slash,
    /// `%` modulo
    Percent,

    /// `is` equality comparison
    Is,
    /// `is not` inequality comparison
    IsNot,
    /// `greater than` comparison
    GreaterThan,
    /// `less than` comparison
    LessThan,
    /// `at least` (>=) comparison
    AtLeast,
    /// `at most` (<=) comparison
    AtMost,

    /// `<` left angle bracket (for generic type syntax only)
    LeftAngle,
    /// `>` right angle bracket (for generic type syntax only)
    RightAngle,

    /// `and` logical AND
    And,
    /// `or` logical OR
    Or,
    /// `not` logical NOT
    Not,

    /// `|` pipeline operator
    Pipe,
    /// `->` arrow (for return type annotations)
    Arrow,

    // === Delimiters ===
    /// `(` left parenthesis
    LeftParen,
    /// `)` right parenthesis
    RightParen,
    /// `[` left bracket
    LeftBracket,
    /// `]` right bracket
    RightBracket,
    /// `{` left brace
    LeftBrace,
    /// `}` right brace
    RightBrace,
    /// `,` comma
    Comma,
    /// `:` colon
    Colon,
    /// `.` dot (member access)
    Dot,

    // === Special ===
    /// Newline (significant in Glimmer-Weave)
    Newline,
    /// End of file
    Eof,
}

impl Token {
    /// Check if this token is a keyword
    pub fn is_keyword(&self) -> bool {
        matches!(
            self,
            Token::Bind
                | Token::Weave
                | Token::Set
                | Token::To
                | Token::As
                | Token::Should
                | Token::Then
                | Token::End
                | Token::Otherwise
                | Token::For
                | Token::Each
                | Token::In
                | Token::Range
                | Token::Whilst
                | Token::Chant
                | Token::Yield
                | Token::Form
                | Token::Seek
                | Token::Where
                | Token::By
                | Token::Filter
                | Token::Sort
                | Token::Take
                | Token::First
                | Token::Last
                | Token::Attempt
                | Token::Harmonize
                | Token::On
                | Token::Match
                | Token::When
                | Token::With
                | Token::Request
                | Token::Justification
                | Token::Triumph
                | Token::Mishap
                | Token::Present
                | Token::Absent
                | Token::After
                | Token::Before
                | Token::Descending
                | Token::Ascending
        )
    }

    /// Check if this token can start a statement
    pub fn is_statement_start(&self) -> bool {
        matches!(
            self,
            Token::Bind
                | Token::Weave
                | Token::Set
                | Token::Should
                | Token::For
                | Token::Whilst
                | Token::Chant
                | Token::Form
                | Token::Seek
                | Token::Attempt
                | Token::Match
                | Token::Request
                | Token::Ident(_)
        )
    }

    /// Get a human-readable description of this token
    pub fn description(&self) -> &str {
        match self {
            Token::Bind => "bind",
            Token::Weave => "weave",
            Token::Set => "set",
            Token::To => "to",
            Token::As => "as",
            Token::Should => "should",
            Token::Then => "then",
            Token::End => "end",
            Token::Otherwise => "otherwise",
            Token::For => "for",
            Token::Each => "each",
            Token::In => "in",
            Token::Range => "range",
            Token::Whilst => "whilst",
            Token::Chant => "chant",
            Token::Yield => "yield",
            Token::Form => "form",
            Token::Seek => "seek",
            Token::Where => "where",
            Token::By => "by",
            Token::Filter => "filter",
            Token::Sort => "sort",
            Token::Take => "take",
            Token::First => "first",
            Token::Last => "last",
            Token::Attempt => "attempt",
            Token::Harmonize => "harmonize",
            Token::On => "on",
            Token::Match => "match",
            Token::When => "when",
            Token::With => "with",
            Token::Request => "request",
            Token::Justification => "justification",
            Token::Triumph => "Triumph",
            Token::Mishap => "Mishap",
            Token::Present => "Present",
            Token::Absent => "Absent",
            Token::After => "after",
            Token::Before => "before",
            Token::Descending => "descending",
            Token::Ascending => "ascending",
            Token::Number(_) => "number",
            Token::Text(_) => "text",
            Token::Truth(_) => "truth",
            Token::Nothing => "nothing",
            Token::Ident(_) => "identifier",
            Token::Plus => "+",
            Token::Minus => "-",
            Token::Star => "*",
            Token::Slash => "/",
            Token::Percent => "%",
            Token::Is => "is",
            Token::IsNot => "is not",
            Token::GreaterThan => "greater than",
            Token::LessThan => "less than",
            Token::AtLeast => "at least",
            Token::AtMost => "at most",
            Token::LeftAngle => "<",
            Token::RightAngle => ">",
            Token::And => "and",
            Token::Or => "or",
            Token::Not => "not",
            Token::Pipe => "|",
            Token::Arrow => "->",
            Token::LeftParen => "(",
            Token::RightParen => ")",
            Token::LeftBracket => "[",
            Token::RightBracket => "]",
            Token::LeftBrace => "{",
            Token::RightBrace => "}",
            Token::Comma => ",",
            Token::Colon => ":",
            Token::Dot => ".",
            Token::Newline => "newline",
            Token::Eof => "end of file",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_detection() {
        assert!(Token::Bind.is_keyword());
        assert!(Token::Chant.is_keyword());
        assert!(!Token::Ident("foo".to_string()).is_keyword());
        assert!(!Token::Number(42.0).is_keyword());
    }

    #[test]
    fn test_statement_start() {
        assert!(Token::Bind.is_statement_start());
        assert!(Token::For.is_statement_start());
        assert!(Token::Ident("foo".to_string()).is_statement_start());
        assert!(!Token::Then.is_statement_start());
        assert!(!Token::Plus.is_statement_start());
    }
}
