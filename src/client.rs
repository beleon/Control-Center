extern crate cursive;

use cursive::align::HAlign;
use cursive::view::Scrollable;
use cursive::views::{EditView, Dialog, Panel, TextView};
use cursive::traits::*;
use cursive::Cursive;
use std::os::unix::net::UnixStream;
use std::io::prelude::*;
use std::io;
use std::thread;
use std::env;
use common::{SOCKET_PATH, read_line};
use std::sync::mpsc::{Sender, Receiver};
use std::sync::mpsc;

mod common;

fn run_as_master() {
    // Read some long text from a file.

    let mut siv = Cursive::default();

    // We can quit by pressing q
    siv.add_global_callback('q', |s| s.quit());

    // The text is too long to fit on a line, so the view will wrap lines,
    // and will adapt to the terminal size.
    siv.add_fullscreen_layer(
        Dialog::around(Panel::new(TextView::new("").with_id("text").scrollable()))
            .title("Control Center")
            // This is the alignment for the button
            .h_align(HAlign::Center)
    );
    // Show a popup on top of the view.



    let mut stream = UnixStream::connect(SOCKET_PATH).unwrap();
    stream.write_all(b"Hello! I am Master\n").unwrap();

    let tx_siv = siv.cb_sink().clone();
    let mut s2 = stream.try_clone().unwrap();
    let t = thread::spawn(move || {
        loop {
            let response = read_line(&mut s2);
            let msg = response.clone();
            tx_siv.send(Box::new(move |s: &mut Cursive| {
                s.call_on_id("text", |text: &mut TextView| {
                    text.append(format!("{}\n", msg))
                });
            }));
            if response == "Server: Bye" {
                tx_siv.send(Box::new(|s: &mut Cursive| s.quit()));
                break;
            }
        }
    });

    let (tx_msg, rx_msg): (Sender<String>, Receiver<String>)
                           = mpsc::channel();

    let t2 = thread::spawn(move || {
        loop {
            let msg = rx_msg.recv().unwrap();
            stream.write_all(msg.as_bytes()).unwrap();
            if msg == ":exit\n" {
                break;
            }
        }
    });

    add_name(&mut siv, tx_msg);

    siv.set_fps(10);

    siv.run();

    t.join().unwrap();
    t2.join().unwrap();
}

fn add_name(s: &mut Cursive, tx_msg: Sender<String>) {
    s.add_layer(Dialog::around(EditView::new()
            .with_id("input")
            .fixed_width(10))
        .title("Enter a command")
        .button("Ok", move |s| {
            let input = format!("{}\n", s.call_on_id("input", |view: &mut EditView| {
                view.get_content()
            }).unwrap());
            tx_msg.send(input).unwrap();
        }));
}

fn run_as_slave() {
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

fn main() {
    let args: Vec<String> = env::args().collect();

    if args.len() == 2 && args[1] == "--slave" {
        run_as_slave();
    } else {
        run_as_master();
    }

}
