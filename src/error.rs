use std::error::Error;
use std::fmt::Display;

#[derive(Debug)]
pub enum TmsError {
    ConfigError,
    IoError,
    TuiError(String),
}
impl Error for TmsError {}
impl Display for TmsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ConfigError => write!(f, "Config Error"),
            Self::IoError => write!(f, "IO Error"),
            Self::TuiError(inner) => write!(f, "TUI error: {inner}"),
        }
    }
}

pub type Result<T> = error_stack::Result<T, TmsError>;

#[derive(Debug)]
pub struct Suggestion(pub &'static str);
impl Display for Suggestion {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!("Suggestion: {}", self.0))
    }
}
