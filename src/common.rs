use std::os::unix::net::UnixStream;
use std::io::Read;

pub static SOCKET_PATH: &'static str = "socket";

pub fn read_line(stream: &mut UnixStream) -> String {
    let mut buffer = Vec::new();
    for byte in stream.bytes() {
        let b = byte.unwrap();
        if b == b'\n' {
            break;
        }
        &buffer.push(b);
    }
    String::from_utf8(buffer).unwrap()
}
