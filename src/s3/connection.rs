extern crate openssl;
extern crate rustc_serialize;

use openssl::crypto::{hmac, hash};

pub struct Connection {
        protocol : &str,
}

pub struct Authorizer {
        access_key: &str,
        secret_key: &str,
}

impl Connection {
    pub fn new(access_key: &str, secret_key: &str, protocol: &str) -> Connection {
        Connection {
            Authorizer {
                access_key: access_key,
                secret_key: secret_key,
            },
            protocol: protocol,
        }
    }
}

// todo make this part of a trait so that it can be mocked for fakes3
impl Authorizer {
    pub fn sign(region: &str, service: &str, headers: hyper::headers::Headers)
        let signature = {
            let mut hmac = hmac::HMAC::new(hash::Type::SHA1, self.secret_key.as_bytes());
            let _ = hmac.write_all(string.as_bytes);
            hmac.finish.to_base64(rustc_serialize::base64::STANDARD)
        };

        headers.set("Authentication", signature);
        headers
    }
}
