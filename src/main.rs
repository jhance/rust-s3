
pub mod s3;

fn main() {
    println!("Hello, world!");

    let access_key = std::env::var("AWS_ACCESS_KEY").ok().expect("need access key");
    let secret_key = std::env::var("AWS_SECRET_KEY").ok().expect("need secret key");

    let connection = s3::connection::Connection::new(access_key, secret_key, "http".to_string());
}

