use std::io::Read;
use std::io::Seek;

use chrono::DateTime;
use chrono::UTC;
use hyper;

use connection::Connection;
use error::Error;

header! { (RequireTag, "If-Match") => [String] }
header! { (RequireNotTag, "If-None-Match") => [String] }
header! { (RequireModifiedSince, "If-Modified-Since") => [String] }
header! { (RequireNotModifiedSince, "If-Unmodified-Since") => [String] }
header! { (ByteRange, "Range:bytes") => [String] }

pub struct Bucket<'a> {
    hostname: String,
    connection: &'a Connection,
    region: String,
}

impl <'a> Bucket<'a> {
    /// Creates a new bucket. This is equivalent to `Connection.bucket`.
    pub fn new(connection: &'a Connection, region: &str, name: &str) -> Self {
        Bucket {
            hostname: connection.host(region, name),
            region: region.to_string(),
            connection: connection,
        }
    }

    fn object_url(&self, path: &str) -> String {
        format!("{}://{}/{}", self.connection.protocol(), self.hostname, path)
    }

    /// Gets the corresponding response for a bucket get request. Since the response
    /// implements Read, the resulting data can be read from the object using
    /// the normal io operations.
    pub fn get(&'a self, path: &str) -> Result<hyper::client::Response, Error> {
        GetObject::new(&self, &path).send()
    }

    /// Gets the contents of a path in the bucket.
    pub fn get_as_string(&'a self, path: &str) -> Result<String, Error> {
        GetObject::new(&self, &path).contents()
    }

    /// Create/update an object in a bucket using the specified source. Because
    /// we have to hash the entire object using SHA256 before sending it, we
    /// require `Seek` to be implemented.
    pub fn put<R: Read+Seek>(&self, path: &str, source: &mut R) -> Result<hyper::client::Response, Error> {
        let mut src = source;
        PutObject::new(&self, &path, &mut src).send()
    }
}

/// Advanced API for getting objects that allows setting optional headers.
/// Call 'send' or 'contents' after setting all options you wish.
pub struct GetObject<'b> {
    path: String,
    bucket: &'b Bucket<'b>,
    require_tag: Option<String>,
    require_not_tag: Option<String>,
    require_modified_since: Option<String>,
    require_not_modified_since: Option<String>,
    byte_range: Option<(i32, i32)>,
}

impl <'b> GetObject<'b> {
    pub fn new(bucket: &'b Bucket<'b>, path: &str) -> Self {
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

    /// Set the required "Etag" of the object. This is usually just the md5sum,
    /// so you can use this for sanity-checking if you know it ahead of time.
    pub fn require_tag(mut self, tag: &str) -> Self {
        self.require_tag = Some(tag.to_string());
        self
    }

    /// Require that the object not have the specified "etag". This is usually just
    /// the md5sum so you can use this for syncing small objects (just calculate the
    /// md5sum and pass that as the not-tag, and the item won't be re-downloaded).
    pub fn require_not_tag(mut self, tag: &str) -> Self {
        self.require_not_tag = Some(tag.to_string());
        self
    }

    /// XXX its not specified how to format this header, and this does not work
    pub fn require_modified_since(mut self, dt: &DateTime<UTC>) -> Self {
        self.require_modified_since = Some(dt.format("%Y%m%dT%H%M%SZ").to_string());
        self
    }

    /// XXX its not specified how to format this header, and this does not work
    pub fn require_not_modified_since(mut self, dt: &DateTime<UTC>) -> Self {
        self.require_not_modified_since = Some(dt.format("%Y%m%dT%H%M%SZ").to_string());
        self
    }

    /// Set the specified byte-range to download.
    pub fn byte_range(mut self, start: i32, end: i32) -> Self {
        self.byte_range = Some((start, end));
        self
    }

    pub fn send(&self) -> Result<hyper::client::Response, Error> {
        let request = try!(self.request());
        let payload: Option<&mut ::std::io::Empty> = None;
        Ok(try!(self.bucket.connection.send(request, payload)))
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

    fn request(&self) -> Result<hyper::client::Request<hyper::net::Fresh>, Error> {
        let url = try!(hyper::Url::parse(&self.bucket.object_url(&self.path)));
        let mut request = try!(hyper::client::Request::new(hyper::method::Method::Get, url));
        self.fill_headers(&mut request.headers_mut());
        let payload: Option<&mut ::std::io::Empty> = None;
        try!(self.bucket.connection.sign(&self.bucket.region, &mut request, payload));
        println!("{:?}", request.headers());
        Ok(request)
    }

    fn fill_headers(&self, headers: &mut hyper::header::Headers) {
        match self.require_tag {
            None => {}
            Some(ref tag) => { headers.set(RequireTag(tag.to_string())); }
        };
        match self.require_not_tag {
            None => {}
            Some(ref tag) => { headers.set(RequireNotTag(tag.to_string())); }
        };
        match self.require_modified_since {
            None => {}
            Some(ref date) => { headers.set(RequireModifiedSince(date.to_string())); }
        };
        match self.require_not_modified_since {
            None => {}
            Some(ref date) => { headers.set(RequireNotModifiedSince(date.to_string())); }
        };
        match self.byte_range {
            None => {}
            Some((start, end)) => { headers.set(ByteRange(format!("{}-{}", start, end))); }
        };
    }
}

/// Advanced api for creating/updating objects. Call 'send' after setting all desired options.
///
/// This part of the api does *not* support multi-part uploads and this will be a separate
/// feature (possibly as an option to PutObject f the amazon api is compatible 100%, otherwise
/// it will probably be PutPart).
pub struct PutObject<'a, R: Read+Seek+'a> {
    path: String,
    source: &'a mut R,
    bucket: &'a Bucket<'a>,
}

impl <'a, R: Read+Seek> PutObject<'a, R> {
    pub fn new(bucket: &'a Bucket<'a>, path: &str, source: &'a mut R) -> Self {
        PutObject {
            path: path.to_string(),
            source: source,
            bucket: bucket,
        }
    }

    /// Send the request and get the response.
    pub fn send(mut self) -> Result<hyper::client::Response, Error> {
        let request = try!(self.request());
        Ok(try!(self.bucket.connection.send(request, Some(&mut self.source))))
    }

    /// Get the raw request associated with the operation. Seeks the input back to 0.
    /// This request is signed, but there isn't anything stopping you from re-signing
    /// the request since this will only overwrite the Authorization header.
    fn request(&mut self) -> Result<hyper::client::Request<hyper::net::Fresh>, Error> {
        let url = try!(hyper::Url::parse(&self.bucket.object_url(&self.path)));
        let mut request = try!(hyper::client::Request::new(hyper::method::Method::Put, url));
        try!(self.fill_headers(&mut request.headers_mut()));
        try!(self.bucket.connection.sign(&self.bucket.region, &mut request, Some(&mut self.source)));
        try!(self.source.seek(::std::io::SeekFrom::Start(0)));
        println!("{:?}", request.headers());
        Ok(request)
    }

    fn fill_headers(&mut self, headers: &mut hyper::header::Headers) -> Result<(), Error> {
        let len = try!(self.source.seek(::std::io::SeekFrom::End(0)));
        headers.set(hyper::header::ContentLength(len as u64));
        try!(self.source.seek(::std::io::SeekFrom::Start(0)));
        Ok(())
    }
}
