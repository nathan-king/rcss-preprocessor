use std::fmt;

#[derive(Clone, Copy, Debug)]
pub struct Span {
    pub line: usize,
    pub column: usize,
}

impl Span {
    #[allow(dead_code)]
    pub const fn dummy() -> Self {
        Self { line: 0, column: 0 }
    }

    pub fn with_offset(&self, offset: usize) -> Self {
        Self {
            line: self.line,
            column: self.column + offset,
        }
    }
}

impl fmt::Display for Span {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.line == 0 && self.column == 0 {
            write!(f, "<unknown>")
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}
