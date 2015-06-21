use hyper;
use connection::Connection;
use std::io::Read;

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

pub enum Error {
    SomeError
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

    pub fn send(&self) -> Result<hyper::client::Response, hyper::error::Error> {
        let request = self.request().ok().expect("could not create request to send");
        self.bucket.connection.send(request, None)
    }

    pub fn contents(&self) -> Result<String, Error> {
        let mut response = self.send().ok().expect("didn't get proper response");
        if response.status == hyper::status::StatusCode::Ok {
            let mut buf = String::new();
            response.read_to_string(&mut buf);
            Ok(buf)
        }
        else {
            println!("{}", response.status);
            unreachable!();
        }
    }

    pub fn to_file(&self, path: &str) {
    }

    fn request(&self) -> Result<hyper::client::Request<hyper::net::Fresh>, Error> {
        let url = hyper::Url::parse(&self.bucket.object_url(&self.path))
                        .ok().expect("could not parse object url");
        let mut request = hyper::client::Request::new(hyper::method::Method::Get, url).ok().expect("could not create request");
        self.fill_headers(&mut request.headers_mut());
        self.bucket.connection.sign(&self.bucket.region, &mut request, None);
        Ok(request)
    }

    fn fill_headers(&self, headers: &mut hyper::header::Headers) {
    }
}
