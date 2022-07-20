use core::fmt;

#[derive(PartialEq,Debug)]

/// Enum of predictable errors. Each error should provide a 
/// specific error message, indicating the line raising the bug.
pub enum Errors {
    InvalidOperation(String),
    InvalidClient(String),
    InvalidTx(String),
    InvalidAmount(String),
    ConflictTransaction(String),
    SecurityErrDivergentClientId(String)

}

impl fmt::Display for Errors {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            Errors::InvalidOperation(line) => write!(f, "Invalid Operation at line: {}",line),
            Errors::InvalidClient(line) => write!(f, "Invalid Client at line: {}",line),
            Errors::InvalidTx(line) => write!(f, "Invalid Tx at line: {}",line),
            Errors::InvalidAmount(line) => write!(f, "Invalid Amount at line: {}",line),
            Errors::ConflictTransaction(line) => write!(f, "Conflicting Transaction at line: {}",line),
            Errors::SecurityErrDivergentClientId(line) => write!(f, "Divergent Transaction and Client ID at line: {}",line),
       }
    }
}
