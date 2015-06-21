extern crate chrono;
extern crate sss;

use std::env;
use chrono::UTC;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bucket_name = args.get(1).expect("need bucket");
    let file = args.get(2).expect("need file");
    let credentials = sss::Credentials::from_env();
    let connection = sss::Connection::new(credentials);

    let bucket = connection.bucket("us-west-2", &bucket_name);
    let now = UTC::now();
    match bucket.get(&file).require_modified_since(&now).contents() {
        Ok(contents) => { print!("{}", contents); }
        Err(e) => { println!("{:?}", e); }
    }
}

