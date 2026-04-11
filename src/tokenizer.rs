//! Module for tokenizing input strings into a list of tokens according to the Lox grammar.

use std::fmt::Debug;

use crate::format::format_num;

/// Possible states for the tokenizer's finite-state machine.
enum TokenizerState {
    Default,

    // ! = < > (may be followed by =)
    // Parameters are the token type and lexeme to emit if the next character is not '='.
    WaitingEquals(String, String),

    WaitingSlash, // / may be followed by / (start of comment)
    Comment,
    Number(String),     // Accumulate digits for number literals (no dot yet)
    NumberDot(String),  // Accumulate digits for number literals that have a dot
    String(String),     // " received, waiting for closing "
    Identifier(String), // Accumulate characters for identifiers and keywords
    Finished,
}

/// Identifiers cannot be one of these, which have special meaning to the parser.
const RESERVED_KEYWORDS: &[&str] = &[
    "and", "class", "else", "false", "for", "fun", "if", "nil", "or", "print", "return", "super",
    "this", "true", "var", "while",
];

/// A token with a type, lexeme, literal value, and line/position information for error reporting.
/// The literal value is:
///  - the unquoted string for string literals
///  - the number as a string for number literals, with at least one decimal place, e.g. "1.0"
///  - "null" for all other tokens (including keywords and identifiers)
#[derive(Clone)]
pub struct Token {
    pub token_type: String,
    pub lexeme: String,
    pub literal: String,
    pub line: usize,
    pub col: usize,
}

impl Token {
    fn new(
        token_type: String,
        lexeme: String,
        literal: String,
        line_no: usize,
        position: usize,
    ) -> Self {
        Token {
            token_type,
            lexeme,
            literal,
            line: line_no,
            col: position,
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "({}, {}, {}, line {}, pos {})",
            self.token_type, self.lexeme, self.literal, self.line, self.col
        )
    }
}

/// Finite-state machine for tokenizing the input string.
struct Tokenizer {
    state: TokenizerState,
    has_errors: bool,
    tokens: Vec<Token>,
    curr_line: usize,
    curr_pos: usize,
    /// The column at which the current (multi-char/deferred) token started.
    /// Set whenever we enter an accumulating state (WaitingEquals, Number, etc.).
    curr_token_start: usize,
}

impl Tokenizer {
    fn new() -> Self {
        Tokenizer {
            state: TokenizerState::Default,
            has_errors: false,
            tokens: Vec::new(),
            curr_line: 1,
            curr_pos: 1,
            curr_token_start: 1,
        }
    }

    fn push(&mut self, token_type: String, lexeme: String, literal: String) {
        self.tokens.push(Token::new(
            token_type,
            lexeme,
            literal,
            self.curr_line,
            self.curr_token_start,
        ));
    }

    /// Handle a single character of input, pushing tokens and updating the state as needed.
    fn handle_char(&mut self, c: char) {
        // Take ownership of the current state to avoid borrowing issues
        // This sets the state in the struct to Default temporarily, but we will set it
        // to the correct state before returning.
        let state = std::mem::replace(&mut self.state, TokenizerState::Default);

        match state {
            TokenizerState::Finished => {
                self.state = TokenizerState::Finished; // Restore the finished state
                panic!("Tokenizer is in finished state, but received more input.");
            }
            TokenizerState::Default => {
                self.curr_token_start = self.curr_pos;
                match c {
                    '=' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::WaitingEquals("EQUAL".into(), "=".into());
                    }
                    '!' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::WaitingEquals("BANG".into(), "!".into());
                    }
                    '<' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::WaitingEquals("LESS".into(), "<".into());
                    }
                    '>' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::WaitingEquals("GREATER".into(), ">".into());
                    }
                    '/' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::WaitingSlash;
                    }
                    ',' => {
                        self.push("COMMA".into(), ",".into(), "null".into());
                    }
                    '.' => {
                        self.push("DOT".into(), ".".into(), "null".into());
                    }
                    '-' => {
                        self.push("MINUS".into(), "-".into(), "null".into());
                    }
                    '+' => {
                        self.push("PLUS".into(), "+".into(), "null".into());
                    }
                    ';' => {
                        self.push("SEMICOLON".into(), ";".into(), "null".into());
                    }
                    '*' => {
                        self.push("STAR".into(), "*".into(), "null".into());
                    }
                    '(' => {
                        self.push("LEFT_PAREN".into(), "(".into(), "null".into());
                    }
                    ')' => {
                        self.push("RIGHT_PAREN".into(), ")".into(), "null".into());
                    }
                    '{' => {
                        self.push("LEFT_BRACE".into(), "{".into(), "null".into());
                    }
                    '}' => {
                        self.push("RIGHT_BRACE".into(), "}".into(), "null".into());
                    }
                    '"' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::String(String::new());
                    }
                    '0'..='9' => {
                        self.curr_token_start = self.curr_pos;
                        self.state = TokenizerState::Number(c.to_string());
                    }
                    '_' | 'a'..='z' | 'A'..='Z' => {
                        self.curr_token_start = self.curr_pos;
                        let new_str = c.to_string();
                        self.state = TokenizerState::Identifier(new_str);
                    }
                    ' ' | '\r' | '\t' => {
                        // Ignore whitespace characters.
                    }
                    '\n' => {
                        self.curr_line += 1;
                        self.curr_pos = 0; // Becomes 1 after incrementing at end of `match`.
                    }
                    _ => {
                        // Mark as error for unrecognized characters.
                        eprintln!("[line {}] Error: Unexpected character: {c}", self.curr_line);
                        self.has_errors = true;
                    }
                }
            }
            TokenizerState::WaitingEquals(curr_token, curr_lexeme) => {
                // Check if the next character is also '=', indicating '=='.
                // Note that this always enters the Default state afterwards.
                match c {
                    '=' => match curr_token.as_str() {
                        "EQUAL" => {
                            self.push("EQUAL_EQUAL".into(), "==".into(), "null".into());
                        }
                        "BANG" => {
                            self.push("BANG_EQUAL".into(), "!=".into(), "null".into());
                        }
                        "LESS" => {
                            self.push("LESS_EQUAL".into(), "<=".into(), "null".into());
                        }
                        "GREATER" => {
                            self.push("GREATER_EQUAL".into(), ">=".into(), "null".into());
                        }
                        _ => unreachable!(),
                    },
                    _ => {
                        self.push(curr_token, curr_lexeme, "null".into());
                        // Re-process the current character in the Default state.
                        // Decrement after to fix double-firing of `curr_pos += 1`.
                        self.handle_char(c);
                        self.curr_pos -= 1;
                    }
                }
            }
            TokenizerState::WaitingSlash => {
                match c {
                    '/' => {
                        self.state = TokenizerState::Comment;
                    }
                    _ => {
                        self.push("SLASH".into(), "/".into(), "null".into());
                        // Re-process the current character in the Default state.
                        // Decrement after to fix double-firing of `curr_pos += 1`.
                        self.handle_char(c);
                        self.curr_pos -= 1;
                    }
                }
            }
            TokenizerState::Comment => {
                if let '\n' = c {
                    self.curr_line += 1;
                    self.state = TokenizerState::Default;
                } else {
                    // Restore the Comment state
                    self.state = TokenizerState::Comment;
                }
            }
            TokenizerState::Number(current_str) => {
                match c {
                    '0'..='9' => {
                        // Extend the integer part of the number.
                        let mut new_str = current_str.clone();
                        new_str.push(c);
                        self.state = TokenizerState::Number(new_str);
                    }
                    '.' => {
                        // Transition into the fractional part of the number.
                        let mut new_str = current_str.clone();
                        new_str.push(c);
                        self.state = TokenizerState::NumberDot(new_str);
                    }
                    _ => {
                        // Can safely unwrap here assuming our tokenizer is correct.
                        // Ensures integer literals like "123" are emitted as "123.0" to match
                        // the expected output format.
                        let value = current_str.parse::<f64>().unwrap();

                        self.push("NUMBER".into(), current_str.clone(), format_num(value));
                        // Re-process the current character in the Default state.
                        // Decrement after to fix double-firing of `curr_pos += 1`.
                        self.handle_char(c);
                        self.curr_pos -= 1;
                    }
                }
            }
            TokenizerState::NumberDot(current_str) => {
                match c {
                    '0'..='9' => {
                        // Extend the fractional part of the number.
                        let mut new_str = current_str.clone();
                        new_str.push(c);
                        self.state = TokenizerState::NumberDot(new_str);
                    }
                    _ => {
                        // End of number literal
                        let value = current_str.parse::<f64>().unwrap();
                        self.push("NUMBER".into(), current_str.clone(), format_num(value));
                        // Re-process the current character in the Default state.
                        // Decrement after to fix double-firing of `curr_pos += 1`.
                        self.handle_char(c);
                        self.curr_pos -= 1;
                    }
                }
            }
            TokenizerState::Identifier(current_str) => {
                match c {
                    '_' | 'a'..='z' | 'A'..='Z' | '0'..='9' => {
                        let mut new_str = current_str.clone();
                        new_str.push(c);
                        self.state = TokenizerState::Identifier(new_str);
                    }
                    _ => {
                        // Check if the identifier is a reserved keyword
                        // push either (<KEYWORD>, <keyword>, null) or
                        // (IDENTIFIER, lexeme, null)
                        let token_type = if RESERVED_KEYWORDS.contains(&current_str.as_str()) {
                            current_str.to_uppercase()
                        } else {
                            "IDENTIFIER".into()
                        };
                        self.push(token_type, current_str.clone(), "null".into());
                        // Re-process the current character in the Default state.
                        // Decrement after to fix double-firing of `curr_pos += 1`.
                        self.handle_char(c);
                        self.curr_pos -= 1;
                    }
                }
            }
            TokenizerState::String(current_str) => {
                match c {
                    '"' => {
                        // End of string
                        let lexeme = format!("\"{}\"", current_str);
                        self.push("STRING".into(), lexeme, current_str.clone());
                    }
                    '\n' => {
                        // Lox strings can span multiple lines; just include the newline.
                        let mut new_str = current_str.clone();
                        new_str.push('\n');
                        self.state = TokenizerState::String(new_str);
                        self.curr_line += 1;
                        self.curr_pos = 0; // Becomes 1 after incrementing at end of `match`.
                    }
                    _ => {
                        let mut new_str = current_str.clone();
                        new_str.push(c);
                        // Restore the String state with the updated string.
                        self.state = TokenizerState::String(new_str);
                    }
                }
            }
        }
        self.curr_pos += 1;
    }

    /// Handle the final state after processing all characters.
    fn handle_final_state(&mut self) {
        let state = std::mem::replace(&mut self.state, TokenizerState::Finished);
        match state {
            TokenizerState::WaitingEquals(curr_token, curr_lexeme) => {
                self.push(curr_token, curr_lexeme, "null".into());
            }
            TokenizerState::WaitingSlash => {
                self.push("SLASH".into(), "/".into(), "null".into());
            }
            TokenizerState::Number(s) | TokenizerState::NumberDot(s) => {
                // Last token is a number
                let value = s.parse::<f64>().unwrap();
                self.push("NUMBER".into(), s.clone(), format_num(value));
            }
            TokenizerState::Identifier(s) => {
                // Check if the identifier is a reserved keyword
                // push either (<KEYWORD>, <keyword>, null) or (IDENTIFIER, lexeme, null)
                let token_type = if RESERVED_KEYWORDS.contains(&s.as_str()) {
                    s.to_uppercase()
                } else {
                    "IDENTIFIER".into()
                };
                self.push(token_type, s.clone(), "null".into());
            }
            TokenizerState::String(_) => {
                eprintln!("[line {}] Error: Unterminated string.", self.curr_line);
                self.has_errors = true;
            }
            _ => {}
        }
        // Set state to Finished
        self.state = TokenizerState::Finished;
    }
}

/// Apply the tokenizer to the input string and return the list of tokens and whether any
/// errors were encountered.
pub fn tokenize(input: &str) -> (Vec<Token>, bool) {
    let mut tokenizer = Tokenizer::new();
    for c in input.chars() {
        tokenizer.handle_char(c);
    }

    // handle the final state, e.g. if we are in the '=' state, and there is
    // no more input, we should emit the 'EQUAL' token.
    tokenizer.handle_final_state();

    // Add EOF token at the end of the input.
    tokenizer.push("EOF".into(), "".into(), "null".into());

    (tokenizer.tokens, tokenizer.has_errors)
}
