//! Defines a `Span` struct to represent a range of source code locations.

/// A location in the source code.
#[derive(Clone)]
pub struct Location {
    pub line: usize,
    pub col: usize,
}

/// A span in the source code corresponding to an expression.
#[derive(Clone)]
pub struct Span {
    pub start: Location,
    pub end: Location,
}

impl Location {
    pub fn new(line: usize, col: usize) -> Self {
        Self { line, col }
    }
}

impl Span {
    pub fn new(start: Location, end: Location) -> Self {
        Self { start, end }
    }
}
