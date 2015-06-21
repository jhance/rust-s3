use hyper::client::request::Request;
use hyper::client::Response;
use hyper::header::Headers;
use hyper::header::HeaderView;
use hyper::net::Fresh;
use hyper::Url;
use std::env;
use std::ascii::AsciiExt;
use crypto::digest::Digest;
use crypto::sha2::Sha256;
use chrono::DateTime;
use chrono::UTC;
use rustc_serialize::hex::ToHex;
use hyper::error;
use std::io::Read;
use openssl::crypto::hash;
use openssl::crypto::hmac;
use bucket;

header! { (XAMZHash, "x-amz-content-sha256") => [String] }
header! { (XAMZDate, "x-amz-date") => [String] }
header! { (Authorization, "Authorization") => [String] }

pub struct Credentials {
    access_key: String,
    secret_key: String,
}

pub struct Connection {
    credentials: Credentials,
    fake: bool,
}

impl Credentials {
    pub fn new(access_key: &str, secret_key: &str) -> Self {
        Credentials {
            access_key: access_key.to_string(),
            secret_key: secret_key.to_string(),
        }
    }

    pub fn from_env() -> Credentials {
        let access_key = env::var("AWS_ACCESS_KEY").ok().expect("need access key");
        let secret_key = env::var("AWS_SECRET_KEY").ok().expect("need secret key");
        Credentials::new(&access_key, &secret_key)
    }
}

impl Connection {
    pub fn new(credentials: Credentials) -> Self {
        Connection {
            credentials: credentials,
            fake: false,
        }
    }

    pub fn new_fake() -> Self {
        Connection {
            credentials: Credentials {
                access_key: "AKIAIOSFODNN7EXAMPLE".to_string(),
                secret_key: "wJalrXUtnFEMI/K7MDENG/bPxRfiCYEXAMPLEKEY".to_string(),
            },
            fake: true,
        }
    }

    pub fn bucket<'a>(&'a self, region: &str, name: &str) -> bucket::Bucket<'a> {
        bucket::Bucket::new(&self, &region, &name)
    }

    pub fn protocol(&self) -> &'static str {
        if self.fake { "http" } else { "https" }
    }

    pub fn host(&self, region: &str, bucket_name: &str) -> String {
        // XXX we should override to localhost:some port here for fake connections
        // or somehow split fake connections into a trait.
        format!("{}.s3-{}.amazonaws.com", bucket_name, region)
    }

    /// Signs an outgoing request a specific region using the
    /// credentials that the connection was created with.
    pub fn sign<R: Read>(&self, region: &str, request: &mut Request<Fresh>, payload: Option<&mut R>) -> Result<(), ::std::io::Error> {
        let dt = UTC::now();
        let payload_hash = try!(self.payload_hash(payload));
        {
            let mut headers = request.headers_mut();
            headers.set(XAMZHash(payload_hash.to_owned()));
            headers.set(XAMZDate(self.timestamp(&dt)));
        }
        let creq = self.canonical_request(request, &payload_hash);
        let signing_str = self.signing_string(&creq, &region, &dt);
        let signing_key = self.signing_key(region, &dt);
        let signature = self.hmac(&signing_key, &signing_str).to_hex();
        let credential = self.credential(region, &dt);
        let auth = self.authorization(&credential, &signature, &request.headers());
        {
            let mut headers = request.headers_mut();
            headers.set(Authorization(auth.to_owned()));
        }
        Ok(())
    }

    fn authorization(&self, credential: &str, signature: &str, headers: &Headers) -> String{
        format!("AWS4-HMAC-SHA256 Credential={},SignedHeaders={},Signature={}",
                credential,
                self.signed_headers(&headers),
                signature)
    }

    fn credential(&self, region: &str, dt: &DateTime<UTC>) -> String {
        format!("{}/{}/{}/s3/aws4_request", self.credentials.access_key, self.datestamp(&dt), region)
    }

    fn signing_key(&self, region: &str, dt: &DateTime<UTC>) -> Vec<u8> {
        let key = format!("AWS4{}", self.credentials.secret_key);
        let sk = key.as_bytes();
        let dk = self.hmac(&sk, &self.datestamp(&dt));
        let drk = self.hmac(&dk, region);
        let drsk = self.hmac(&drk, "s3");
        self.hmac(&drsk, "aws4_request")
    }

    fn scope(&self, region: &str, dt: &DateTime<UTC>) -> String {
        format!("{datestamp}/{region}/s3/aws4_request",
                datestamp = self.datestamp(dt),
                region = region)
    }

    fn signing_string(&self, creq: &str, region: &str, dt: &DateTime<UTC>) -> String {
        let mut sha256 = Sha256::new();
        sha256.input_str(creq);
        let creq_hash = sha256.result_str();

        format!("AWS4-HMAC-SHA256\n{timestamp}\n{scope}\n{creq_hash}",
                timestamp = self.timestamp(&dt),
                scope = self.scope(region, &dt),
                creq_hash = creq_hash)
    }

    fn timestamp(&self, dt: &DateTime<UTC>) -> String {
        dt.format("%Y%m%dT%H%M%SZ").to_string()
    }

    fn datestamp(&self, dt: &DateTime<UTC>) -> String {
        dt.format("%Y%m%d").to_string()
    }

    fn canonical_request(&self, request: &Request<Fresh>, payload_hash: &str) -> String {
        let canonical_method = request.method().to_string();
        let canonical_path = request.url.serialize_path().expect("require serializable path");
        let canonical_query = self.canonical_query(&request.url);
        let canonical_headers = self.canonical_headers(request.headers());
        let signed_headers = self.signed_headers(request.headers());
        format!("{method}\n{path}\n{query}\n{headers}\n{signed_headers}\n{payload_hash}",
                method = canonical_method,
                path = canonical_path,
                query = canonical_query,
                headers = canonical_headers,
                signed_headers = signed_headers,
                payload_hash = payload_hash)
    }

    fn payload_hash<R: Read>(&self, payload: Option<&mut R>) -> Result<String, ::std::io::Error> {
        let mut hash = hash::Hasher::new(hash::Type::SHA256);
        match payload {
            None => {}
            Some(reader) => { try!(::std::io::copy(reader, &mut hash)); }
        };
        Ok(hash.finish().to_hex())
    }

    fn signed_headers(&self, headers: &Headers) -> String {
        let mut vec = headers.iter()
                         .map(|hv| hv.name().to_ascii_lowercase())
                         .collect::<Vec<String>>();
        vec.sort();
        vec.connect(";").to_string()
    }

    fn canonical_headers(&self, headers: &Headers) -> String {
        let mut vec = headers.iter()
                         .map(|hv| self.header_tuple(&hv))
                         .collect::<Vec<(String, String)>>();
        vec.sort();
        format!("{}\n", vec.iter()
           .map(|t| format!("{}:{}", t.0, t.1))
           .collect::<Vec<String>>()
           .connect("\n")
           .to_string())
    }

    fn header_tuple(&self, header_view: &HeaderView) -> (String, String) {
        (header_view.name().to_ascii_lowercase(), header_view.value_string())
    }

    fn canonical_query(&self, url: &Url) -> String {
        match url.query_pairs() {
            None => "".to_string(),
            Some(vec) => vec.iter()
                            .map(|t| format!("{}={}", t.0, t.1))
                            .collect::<Vec<String>>()
                            .connect("&")
                            .to_string(),
        }
    }

    fn hmac(&self, key: &[u8], val: &str) -> Vec<u8> {
        hmac::hmac(hash::Type::SHA256, key, val.as_bytes())
    }

    pub fn send<R: Read>(&self, request: Request<Fresh>, payload: Option<&mut R>) -> Result<Response, error::Error> {
        let mut srequest = request.start().ok().expect("couldn't stream request");
        match payload {
            None => {},
            Some(s) => { try!(::std::io::copy(s, &mut srequest)); }
        }
        srequest.send()
    }
}
