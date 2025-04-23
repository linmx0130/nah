use serde::Deserialize;

#[derive(Debug)]
pub struct NahError {
    code: i32,
    message: String,
}

impl std::fmt::Display for NahError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "NahError {}: {}", self.code, self.message)
    }
}

impl std::error::Error for NahError {}

impl NahError {
    pub fn io_error(message: &str) -> NahError {
        NahError {
            code: 1,
            message: format!("IO Error: {}", message),
        }
    }
    pub fn invalid_value(message: &str) -> NahError {
        NahError {
            code: 2,
            message: format!("Invalid value error: {}", message),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct MCPServerCommand {
    pub command: String,
    pub args: Vec<String>,
}
