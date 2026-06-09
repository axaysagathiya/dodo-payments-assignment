use serde::{Deserialize, Serialize};
use sqlx::Type;
use std::fmt;

#[derive(Debug, Serialize, Deserialize, Type, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum InvoiceState {
    Draft,
    Open,
    Paid,
    Cancelled,
}

impl fmt::Display for InvoiceState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Draft => "draft",
            Self::Open => "open",
            Self::Paid => "paid",
            Self::Cancelled => "cancelled",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, Type, PartialEq, Eq, Clone, Copy)]
#[sqlx(type_name = "text", rename_all = "lowercase")]
#[serde(rename_all = "lowercase")]
pub enum PaymentStatus {
    Unknown,
    Processing,
    Succeeded,
    Failed,
}

impl fmt::Display for PaymentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Unknown => "unknown",
            Self::Processing => "processing",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
        };
        write!(f, "{}", s)
    }
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PspResponseStatus {
    Succeeded,
    Failed,
    Unknown,
    Error,
}

#[derive(Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum PspToken {
    #[serde(rename = "tok_success")]
    Success,
    #[serde(rename = "tok_insufficient_funds")]
    InsufficientFunds,
    #[serde(rename = "tok_card_declined")]
    CardDeclined,
    #[serde(rename = "tok_timeout")]
    Timeout,
    #[serde(rename = "tok_network_error")]
    NetworkError,
}

impl PspToken {
    /// Returns the string representation of the token as bytes.
    pub fn as_bytes(&self) -> &[u8] {
        match self {
            Self::Success => b"tok_success",
            Self::InsufficientFunds => b"tok_insufficient_funds",
            Self::CardDeclined => b"tok_card_declined",
            Self::Timeout => b"tok_timeout",
            Self::NetworkError => b"tok_network_error",
        }
    }
}
