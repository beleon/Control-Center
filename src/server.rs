use std::thread;
use std::collections::HashMap;
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::os::unix::net::{UnixStream, UnixListener};
use std::io::Write;
use std::fs::remove_file;
use common::{SOCKET_PATH, read_line};

mod common;

struct ClientCount {
    master: u64,
    slave: u64,
}

fn register(request: String, con: Arc<Mutex<ClientCount>>) -> Result<String, String> {
    let mut ul_cc = con.lock().unwrap();
    if request == "Hello! I am Master" {
        ul_cc.master += 1;
        let id = format!("m-{}", ul_cc.master);
        return Ok(id);
    } else if request == "Hello! I am Slave" {
        ul_cc.slave += 1;
        let id = format!("s-{}", ul_cc.slave);
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

        if request == "/exit" {
            break;
        }
    }
    println!("Connection closed.");
}

fn handle_messages(rx_stream: Receiver<(String, UnixStream)>, rx_msg: Receiver<(String, String)>) {
    let mut streams = HashMap::new();
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
        match msg.as_ref() {
            "/exit" => {
                streams.get(&id).unwrap().write_all(b"Server: Bye\n").unwrap();
                streams.remove(&id);
            }
            "Hello! I am Master" => {
                streams.get(&id).unwrap().write_all(format!("Server: Hello Master, your id is: {}\n", &id).as_bytes()).unwrap();
            }
            "Hello! I am Slave" => {
                streams.get(&id).unwrap().write_all(format!("Server: Hello Slave, your id is: {}\n", &id).as_bytes()).unwrap();
            }
            _ => {
                for stream in streams.values_mut() {
                    stream.write_all(format!("{}: {}\n", &id, &msg).as_bytes()).unwrap();
                }
            }

        }
        println!("id: {}, request: {}", id, msg);
    }
}

fn main() {
    remove_file("socket").ok();
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
