extern crate cursive;

use cursive::view::Scrollable;
use cursive::view::ScrollStrategy::StickToBottom;
use cursive::views::{IdView, ScrollView, BoxView, LinearLayout, DummyView, EditView, Dialog, Panel, TextView};
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

    let (tx_msg, rx_msg): (Sender<String>, Receiver<String>) = mpsc::channel();

    siv.add_fullscreen_layer(BoxView::with_full_screen(
        Dialog::around(LinearLayout::vertical()
            .child(Panel::new(TextView::new("").with_id("text").scrollable().with_id("scroll"))
                .title("Control Center"))
            .child(DummyView)
            .child(EditView::new()
                .on_submit(move |s, _t| {
                    let input = format!("{}\n", s.call_on_id("input", |view: &mut EditView| {
                        view.get_content()
                    }).unwrap());
                    if input != "\n" {
                        s.call_on_id("input", |view: &mut EditView| {
                            view.set_content("")
                        }).unwrap();
                        tx_msg.send(input).unwrap();
                    }
                })
                .with_id("input")))
    ));
    siv.call_on_id("scroll", |scroll: &mut ScrollView<IdView<TextView>>| {
        scroll.set_scroll_strategy(StickToBottom)
    }).unwrap();

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
                s.call_on_id("scroll", |scroll: &mut ScrollView<IdView<TextView>>| {
                    scroll.set_scroll_strategy(StickToBottom)
                }).unwrap();
            }));
            if response == "Server: Bye" {
                tx_siv.send(Box::new(|s: &mut Cursive| s.quit()));
                break;
            }
        }
    });

    let t2 = thread::spawn(move || {
        loop {
            let msg = rx_msg.recv().unwrap();
            stream.write_all(msg.as_bytes()).unwrap();
            if msg == "/exit\n" {
                break;
            }
        }
    });


    siv.set_fps(10);

    siv.run();

    t.join().unwrap();
    t2.join().unwrap();
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
                if input == "/exit\n" {
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
