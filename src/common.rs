use std::os::unix::net::UnixStream;
use std::io::Read;

pub static SOCKET_PATH: &'static str = "socket";
pub static EXIT_CMD: &'static str = "exit";
pub static LIST_CMD: &'static str = "list";
pub static DM_CMD: &'static str = "msg";
pub static CMD_PREFIX: &'static str = "/";
pub static SERVER_ID: &'static str = "server";
pub static CHAT_SEPARATOR: &'static str = ": ";
pub static DM_SEPARATOR: &'static str = " -> ";
pub static MASTER_ID_PREFIX: &'static str = "m-";
pub static SLAVE_ID_PREFIX: &'static str = "s-";
pub static CLIENT_HELLO: &'static str = "Hello! I am ";
pub static SERVER_FAREWELL: &'static str = "Bye";
pub static MASTER: &'static str = "Master";
pub static SLAVE: &'static str = "Slave";

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
