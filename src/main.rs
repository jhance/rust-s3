extern crate chrono;
extern crate sss;

use std::io::Read;
use std::env;
use chrono::UTC;

fn main() {
    let args: Vec<String> = env::args().collect();
    let bucket_name = args.get(1).expect("need bucket");
    let file = args.get(2).expect("need file");
    let contents = args.get(3).expect("need contents");
    let credentials = sss::Credentials::from_env();
    let connection = sss::Connection::new(credentials);

    let bucket = connection.bucket("us-west-2", &bucket_name);
    let now = UTC::now();
    match bucket.put(&file, &contents).send() {
        Ok(response) => { println!("PUT OK\n{:?}", response); }
        Err(e) => { println!("PUT ERR\n{:?}", e); }
    };
    match bucket.get(&file).send() {
        Ok(mut response) => {
            println!("GET OK");
            let mut outfile = std::fs::File::create("output").ok().expect("couldn't open outfile");
            let _ = std::io::copy(&mut response, &mut outfile);
        }
        Err(e) => { println!("GET ERR\n{:?}", e); }
    };
}

