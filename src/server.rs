use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::Write;
use std::fs::remove_file;
use std::vec::Vec;
use common::*;

macro_rules! SERVER_HELLO {() => ("Hello {}, your id is: ")}

mod common;

struct ClientCount {
    master: u64,
    slave: u64,
}

fn register(request: String, con: Arc<Mutex<ClientCount>>) -> Result<String, String> {
    let mut ul_cc = con.lock().unwrap();
    if request == format!("{}{}", CLIENT_HELLO, MASTER) {
        ul_cc.master += 1;
        let id = format!("{}{}", MASTER_ID_PREFIX, ul_cc.master);
        return Ok(id);
    } else if request == format!("{}{}", CLIENT_HELLO, SLAVE) {
        ul_cc.slave += 1;
        let id = format!("{}{}", SLAVE_ID_PREFIX, ul_cc.slave);
        return Ok(id);
    }
    Err("Invalid request".to_string())
}

fn handle_client(mut stream: UnixStream, tx_stream: Sender<(String, UnixStream)>, tx_msg: Sender<(String, String)>, con: Arc<Mutex<ClientCount>>) {
    println!("Connection established.");
    let request = read_line(&mut stream);
    let id = register(request.clone(), con.clone()).unwrap();
    tx_stream.send((id.clone(), stream.try_clone().unwrap())).unwrap();
    tx_msg.send((id.clone(), request.clone())).unwrap();

    loop {
        let request = read_line(&mut stream);
        tx_msg.send((id.clone(), request.clone())).unwrap();

        if request == format!{"{}{}", CMD_PREFIX, EXIT_CMD} {
            break;
        }
    }
    println!("Connection closed.");
}

fn handle_messages(rx_stream: Receiver<(String, UnixStream)>, rx_msg: Receiver<(String, String)>) {
    let mut streams = HashMap::new();
    let mut hist = Vec::new();
    loop {
        let (id, msg) = rx_msg.recv().unwrap();
        loop {
            let r_stream = rx_stream.try_recv();
            match r_stream {
                Ok(stream) => {
                    streams.insert(stream.0, stream.1);
                }
                Err(_err) => {
                    break;
                }
            }
        }
        if msg == format!("{}{}", CMD_PREFIX, EXIT_CMD) {
            streams.get(&id).unwrap().write_all(format!("{}{}{}\n", SERVER_ID, CHAT_SEPARATOR, SERVER_FAREWELL).as_bytes()).unwrap();
            streams.remove(&id);
        } else if msg == format!("{}{}", CLIENT_HELLO, MASTER) {
            streams.get(&id).unwrap().write_all(format!("{}{}{}{}\n", SERVER_ID, CHAT_SEPARATOR, format!(SERVER_HELLO!(), MASTER), &id).as_bytes()).unwrap();
            for record in &hist {
                let (h_id, h_msg) = record;
                streams.get(&id).unwrap().write_all(format!("{}{}{}\n", &h_id, CHAT_SEPARATOR, &h_msg).as_bytes()).unwrap();
            }
        } else if msg == format!("{}{}", CLIENT_HELLO, SLAVE) {
            streams.get(&id).unwrap().write_all(format!("{}{}{}{}\n", SERVER_ID, CHAT_SEPARATOR, format!(SERVER_HELLO!(), SLAVE),  &id).as_bytes()).unwrap();
            for record in &hist {
                let (h_id, h_msg) = record;
                streams.get(&id).unwrap().write_all(format!("{}{}{}\n", &h_id, CHAT_SEPARATOR, &h_msg).as_bytes()).unwrap();
            }
        } else if msg == format!("{}{}", CMD_PREFIX, LIST_CMD) {
            streams.get(&id).unwrap().write_all(format!("{}{}{:?}\n", SERVER_ID, CHAT_SEPARATOR, streams.keys()).as_bytes()).unwrap();
        } else if msg.starts_with(&format!("{}{} ", CMD_PREFIX, DM_CMD)) {
            let rest: String = msg.chars().skip(format!("{}{} ", CMD_PREFIX, DM_CMD).len()).collect(); // Pray that there is no unicode characters in preifx and dm string consts.
            let idx = rest.find(" ").unwrap();
            let recipient: String = rest.chars().take(idx).collect();
            let real_msg: String = rest.chars().skip(idx + 1).collect();
            streams.get(&recipient).unwrap().write_all(format!("{}{}{}{}{}\n", &id, DM_SEPARATOR, &recipient, CHAT_SEPARATOR, &real_msg).as_bytes()).unwrap();
            streams.get(&id).unwrap().write_all(format!("{}{}{}{}{}\n", &id, DM_SEPARATOR, &recipient, CHAT_SEPARATOR, &real_msg).as_bytes()).unwrap();
        } else {
            &mut hist.push((id.clone(), msg.clone()));
            for stream in streams.values_mut() {
                stream.write_all(format!("{}{}{}\n", &id, CHAT_SEPARATOR, &msg).as_bytes()).unwrap();
            }
        }
        println!("id: {}, request: {}", id, msg);
    }
}

fn main() {
    remove_file(SOCKET_PATH).ok();
    let con = Arc::new(Mutex::new(ClientCount {
        master: 0,
        slave: 0
    }));
    let (tx_stream, rx_stream): (Sender<(String, UnixStream)>, Receiver<(String, UnixStream)>)
                                 = mpsc::channel();
    let (tx_msg, rx_msg): (Sender<(String, String)>, Receiver<(String, String)>)
                           = mpsc::channel();
    thread::spawn(|| handle_messages(rx_stream, rx_msg));

    let listener = UnixListener::bind(SOCKET_PATH).unwrap();

    // accept connections and process them, spawning a new thread for each one
    for stream in listener.incoming() {
        let thread_tx_stream = tx_stream.clone();
        let thread_tx_msg = tx_msg.clone();
        let thread_con = con.clone();
        match stream {
            Ok(stream) => {
                /* connection succeeded */
                thread::spawn(|| handle_client(stream, thread_tx_stream, thread_tx_msg, thread_con));
            }
            Err(_err) => {
                /* connection failed */
                break;
            }
        }
    }
}
