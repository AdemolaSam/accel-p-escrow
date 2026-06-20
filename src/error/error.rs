use thiserror::Error;
use pinocchio::error::{ProgramError};

#[repr(u32)]
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum EscrowError {
    #[error("The token provided is not the expected token")]
    InvalidToken,
}

impl From<EscrowError> for ProgramError {
    fn from(e: EscrowError) -> Self {
        ProgramError::Custom(e as u32)
    }
}