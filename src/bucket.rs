use std::io;
use std::io::Read;

use hyper;
use chrono::DateTime;
use chrono::UTC;
use url;

use connection::Connection;

header! { (RequireTag, "If-Match") => [String] }
header! { (RequireNotTag, "If-None-Match") => [String] }
header! { (RequireModifiedSince, "If-Modified-Since") => [String] }
header! { (RequireNotModifiedSince, "If-Not-Modified-Since") => [String] }

pub struct Bucket<'a> {
    hostname: String,
    connection: &'a Connection,
    region: String,
}

impl <'a> Bucket<'a> {
    pub fn new(connection: &'a Connection, region: &str, name: &str) -> Bucket<'a> {
        Bucket {
            hostname: connection.host(region, name),
            region: region.to_string(),
            connection: connection,
        }
    }

    pub fn object_url(&self, path: &str) -> String {
        format!("{}://{}/{}", self.connection.protocol(), self.hostname, path)
    }

    pub fn get(&'a self, path: &str) -> GetObject<'a> {
        GetObject::new(&self, &path)
    }
}

pub struct GetObject<'b> {
    path: String,
    bucket: &'b Bucket<'b>,
    require_tag: Option<String>,
    require_not_tag: Option<String>,
    require_modified_since: Option<String>,
    require_not_modified_since: Option<String>,
    byte_range: Option<(i32, i32)>,
}

// XXX move this out of this module
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

impl <'b> GetObject<'b> {
    pub fn new(bucket: &'b Bucket<'b>, path: &str) -> GetObject<'b> {
        GetObject {
            path: path.to_string(),
            bucket: &bucket,
            require_tag: None,
            require_not_tag: None,
            require_modified_since: None,
            require_not_modified_since: None,
            byte_range: None,
        }
    }

    pub fn require_tag(mut self, tag: &str) -> Self {
        self.require_tag = Some(tag.to_string());
        self
    }

    pub fn require_not_tag(mut self, tag: &str) -> Self {
        self.require_not_tag = Some(tag.to_string());
        self
    }

    pub fn require_modified_since(mut self, dt: &DateTime<UTC>) -> Self {
        self.require_modified_since = Some(dt.format("%Y%m%dT%H%M%SZ").to_string());
        self
    }

    pub fn require_not_modified_since(mut self, dt: &DateTime<UTC>) -> Self {
        self.require_not_modified_since = Some(dt.format("%Y%m%dT%H%M%SZ").to_string());
        self
    }

    pub fn byte_range(mut self, start: i32, end: i32) -> Self {
        self.byte_range = Some((start, end));
        self
    }

    pub fn send(&self) -> Result<hyper::client::Response, Error> {
        let request = try!(self.request());
        Ok(try!(self.bucket.connection.send(request, None)))
    }

    pub fn contents(&self) -> Result<String, Error> {
        let mut response = try!(self.send());
        if response.status == hyper::status::StatusCode::Ok {
            let mut buf = String::new();
            try!(response.read_to_string(&mut buf));
            Ok(buf)
        }
        else {
            Err(Error::StatusError(response.status))
        }
    }

    pub fn to_file(&self, path: &str) {
    }

    fn request(&self) -> Result<hyper::client::Request<hyper::net::Fresh>, Error> {
        let url = try!(hyper::Url::parse(&self.bucket.object_url(&self.path)));
        let mut request = try!(hyper::client::Request::new(hyper::method::Method::Get, url));
        self.fill_headers(&mut request.headers_mut());
        self.bucket.connection.sign(&self.bucket.region, &mut request, None);
        Ok(request)
    }

    fn fill_headers(&self, headers: &mut hyper::header::Headers) {
    }
}
