extern crate sss;

use std::env;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bucket_name = args.get(1).expect("need bucket");
    let file = args.get(2).expect("need file");
    let credentials = sss::Credentials::from_env();
    let connection = sss::Connection::new(credentials);

    let bucket = connection.bucket("us-west-2", &bucket_name);
    match bucket.get(&file).contents() {
        Ok(contents) => { print!("{}", contents); }
        Err(e) => { println!("{:?}", e); }
    }
}

