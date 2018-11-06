use std::os::unix::net::UnixStream;
use std::io::prelude::*;
use std::io;
use std::thread;
use common::{SOCKET_PATH, read_line};

mod common;

fn main() {
    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    let t;
    {
        let mut s2 = stream.try_clone().unwrap();
        t = thread::spawn(move || {
            loop {
                let response = read_line(&mut s2);
                println!("{}", &response);
                if response == "Server: Bye" {
                    break;
                }
            }
        });
    }
    stream.write_all(b"Hello! I am Slave\n").unwrap();
    loop {
        let mut input = String::new();
        match io::stdin().read_line(&mut input) {
            Ok(_n) => {
                stream.write_all(input.as_bytes()).unwrap();
                if input == ":exit\n" {
                    break;
                }
            }
            Err(error) => println!("error: {}", error),
        }
    }
    t.join().unwrap();
}
