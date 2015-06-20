#[macro_use]
extern crate hyper;

use hyper;

header! { (Authorization, "Authorization") => [String] }

struct Bucket {
    hostname: String,
    protocol: String,
    authentication_token: String,
}

impl Bucket {
    pub fn new(connection: &Connection, region: String, name: String) {
        Bucket {
            hostname: connection.hostname(region, name),
            protocol: connection.protocol,
            authorizer: connection.get_authorizer(),
        }
    }

    pub fn new_mock(hostname: String, protocol: String, authentication_token: String) {
        Bucket {
            hostname: hostname,
            protocol: protocol,
            authentication_token,
        }
    }

    pub fn object_url(&self, path: String) {
        format!("{}://{}/{}", self.protocol, self.hostname, self.path)
    }

    pub fn get_object_contents(&self, path: String) {
        let client = hyper::Client::new();
        let headers = Headers::new();
        headers.set(Authorization(self.authentication_token))
        let res = client.get(self.object_url(path))
                        .send().unwrap();
    }
}
