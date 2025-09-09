use core::fmt;

#[derive(Debug)]
pub enum ParseError {
    UnexpectedToken,
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::UnexpectedToken => write!(f, "Unexpected token"),
        }
    }
}

impl core::error::Error for ParseError {}
