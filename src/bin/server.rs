use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::thread;

fn main() {
    println!("Hello, I'm Server!");

    // 1. We need to listed for incoming connections from users.
    //std::new::TcpListener is exactly that.

    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();

    loop {
        match listener.accept() {
            Ok((socket, addr)) => handle_client(socket, addr),
            Err(e) => println!("couldn't get client: {e:?}")
        }
    }


}

fn handle_client(mut socket: TcpStream, addr: SocketAddr) {

    // 2. We need to have each new connection in a new thread.
    let mut res = String::new();

    thread::spawn(move || {
        // println!("new client: {addr:?}");
        // let _ = socket.write(b"Thanks for coming!");
        socket.read_to_string(&mut res).unwrap();
        println!("{res:?}");
    });
}
