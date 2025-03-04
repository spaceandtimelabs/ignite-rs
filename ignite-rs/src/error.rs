use snafu::Snafu;
use std::convert;
use std::io::Error as IoError;
#[cfg(feature = "ssl")]
use webpki::InvalidDNSNameError;

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Snafu, Debug)]
pub struct Error {
    pub(crate) desc: String,
}

impl convert::From<IoError> for Error {
    fn from(e: IoError) -> Self {
        Error {
            desc: e.to_string(),
        }
    }
}

impl convert::From<&str> for Error {
    fn from(desc: &str) -> Self {
        Error {
            desc: String::from(desc),
        }
    }
}

impl convert::From<Option<String>> for Error {
    fn from(desc: Option<String>) -> Self {
        match desc {
            Some(desc) => Error { desc },
            None => Error {
                desc: "Ignite client error! No description provided".to_owned(),
            },
        }
    }
}

#[cfg(feature = "ssl")]
impl convert::From<InvalidDNSNameError> for Error {
    fn from(err: InvalidDNSNameError) -> Self {
        Error {
            desc: err.to_string(),
        }
    }
}
