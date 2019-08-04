use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub enum CerberusError {
    ConnectionError(String, u16),
}

impl Error for CerberusError {}

impl fmt::Display for CerberusError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CerberusError::ConnectionError(host, port) =>
                write!(f,
                    "Failed to connect to node {}:{} through its public TCP port.",
                    host, port)
        }
    }
}
