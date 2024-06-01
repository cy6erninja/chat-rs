use std::collections::HashMap;
use std::hash::Hash;
use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{ SocketAddr, TcpListener, TcpStream };
use std::sync::{Arc, mpsc, Mutex};
use std::thread;
use chat_rs::user::User;

struct Group {
    name: String,
    users: Vec<User>
}

struct Message {
    pub from: SocketAddr,
    pub message: String,
}

fn main() {
    println!("Hello, I'm Server!");

    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
    let stream_writers: HashMap<SocketAddr, BufWriter<TcpStream>> = HashMap::new();
    // Start using the same interprocess communication technique(names channels)
    let arc_writers = Arc::new(Mutex::new(stream_writers));
    let broadcast_writers = Arc::clone(&arc_writers);

    let (
        sender,
        receiver
    ) = mpsc::channel::<Message>();

    thread::spawn(move || {
        loop {
            let message = receiver.recv().unwrap();
            let mut writers = broadcast_writers.lock().unwrap();

            for (_, mut writer) in &mut *writers {
                writer.write(message.message.as_bytes()).unwrap();
                writer.write(b"\n").unwrap();
                writer.flush().unwrap();
            }
        }
    });

    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                arc_writers.lock().unwrap().insert(
                    socket.peer_addr().unwrap(),
                    BufWriter::new(socket.try_clone().unwrap())
                );

                let thread_sender = sender.clone();

                thread::spawn(move || {
                    let stream_reader = BufReader::new(&socket);
                    let peer_addr = socket.peer_addr().unwrap();

                    for line in stream_reader.lines() {
                        match line {
                            Ok(l) =>
                                {
                                    println!("Received message: {l:?}");
                                    thread_sender.send(Message {
                                        from: peer_addr,
                                        message: l
                                    }).unwrap();
                                },
                            Err(_) => {}
                        };
                    }
                });

            },
            Err(e) => println!("couldn't get client: {e:?}")
        }
    }
}
