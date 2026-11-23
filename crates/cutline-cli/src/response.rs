use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct SuccessEnvelope<T: Serialize> {
    pub status: &'static str,
    pub schema_version: &'static str,
    pub command: String,
    pub data: T,
}

#[derive(Debug, Serialize)]
pub struct ErrorDetails {
    pub code: String,
    pub message: String,
    pub hint: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorEnvelope {
    pub status: &'static str,
    pub schema_version: &'static str,
    pub command: String,
    pub error: ErrorDetails,
}

pub const CURRENT_SCHEMA_VERSION: &str = "1.0.0";

/// Prints pure JSON success envelope strictly to stdout (REQ-AGENT-001)
pub fn print_success<T: Serialize>(command: &str, data: T) {
    let envelope = SuccessEnvelope {
        status: "success",
        schema_version: CURRENT_SCHEMA_VERSION,
        command: command.to_string(),
        data,
    };
    let json = serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string());
    println!("{json}");
}

/// Prints structured JSON error envelope strictly to stdout (REQ-AGENT-001, REQ-AGENT-003)
pub fn print_error(command: &str, code: &str, message: &str, hint: &str) {
    let envelope = ErrorEnvelope {
        status: "error",
        schema_version: CURRENT_SCHEMA_VERSION,
        command: command.to_string(),
        error: ErrorDetails {
            code: code.to_string(),
            message: message.to_string(),
            hint: hint.to_string(),
        },
    };
    let json = serde_json::to_string_pretty(&envelope).unwrap_or_else(|_| "{}".to_string());
    println!("{json}");
}
