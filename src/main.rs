extern crate openssl;
extern crate rustc_serialize;
#[macro_use]
extern crate hyper;
extern crate crypto;
extern crate chrono;

use hyper::client::request::Request;

use std::io::Read;

pub mod s3;

header! { (RangeBytes, "Ranges:bytes") => [String] }
header! { (Date, "Date") => [String] }

fn main() {
    let credentials = s3::connection::Credentials::from_env();
    let connection = s3::connection::Connection::new(credentials);

    let bucket = s3::bucket::Bucket::new(connection, "us-west-2", "test_bucket");
    let contents = bucket.get("testfile").contents().ok().expect("could not get contents");
    println!("{}",  contents);
}

