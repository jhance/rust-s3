use hyper;
use std::io;
use url;

#[derive(Debug)]
pub enum Error {
    StatusError(hyper::status::StatusCode),
    IoError(io::Error),
    HttpError(hyper::error::Error),
    ParseError(url::ParseError),
}

impl From<hyper::error::Error> for Error {
    fn from(e: hyper::error::Error) -> Self {
        Error::HttpError(e)
    }
}

impl From<io::Error> for Error {
    fn from(e: io::Error) -> Self {
        Error::IoError(e)
    }
}

impl From<url::ParseError> for Error {
    fn from(e: url::ParseError) -> Self {
        Error::ParseError(e)
    }
}
