use bufstream::BufStream;
use snafu::Snafu;
use std::convert;
use std::io::Error as IoError;
use std::net::TcpStream;
use std::sync::{MutexGuard, PoisonError};
#[cfg(feature = "ssl")]
use webpki::InvalidDNSNameError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Snafu, Debug)]
pub enum Error {
    /// Custom error description
    #[snafu(display("{}", desc))]
    Custom { desc: String },

    /// IO Error
    #[snafu(display("IO Error: {}", source))]
    IoError { source: IoError },

    /// Poisoned Mutex
    #[snafu(display("Mutex poisoned: {}", desc))]
    MutexPoisoned { desc: String },

    /// Invalid DNS Name
    #[cfg(feature = "ssl")]
    #[snafu(display("Invalid DNS Name: {}", desc))]
    InvalidDNSName { desc: String },
}

impl convert::From<IoError> for Error {
    fn from(err: IoError) -> Self {
        Error::IoError { source: err }
    }
}

impl convert::From<&str> for Error {
    fn from(desc: &str) -> Self {
        Error::Custom {
            desc: desc.to_string(),
        }
    }
}

impl From<PoisonError<MutexGuard<'_, BufStream<TcpStream>>>> for Error {
    fn from(err: PoisonError<MutexGuard<'_, BufStream<TcpStream>>>) -> Self {
        Error::MutexPoisoned {
            desc: err.to_string(),
        }
    }
}

#[cfg(feature = "ssl")]
impl convert::From<InvalidDNSNameError> for Error {
    fn from(err: InvalidDNSNameError) -> Self {
        Error::InvalidDNSName {
            desc: err.to_string(),
        }
    }
}
