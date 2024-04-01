use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // 1. We need to connect to a server socket to be able to send messages there.
    let mut server_socket = TcpStream::connect("127.0.0.1:8000").unwrap();
    println!("{}", "Hello, I'm client");

    println!("{:?}", server_socket);

    server_socket.write(b"Hello, I'm your new client!\n").unwrap();

    loop {
        server_socket.write(b"Ping!\n").unwrap();

        sleep(Duration::new(1, 0));
    }
}