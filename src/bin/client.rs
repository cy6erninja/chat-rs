use std::io::Write;
use std::net::TcpStream;

fn main() {
    // 1. We need to connect to a server socket to be able to send messages there.
    let mut server_socket = TcpStream::connect("127.0.0.1:8000").unwrap();
    // println!("{}", "Hello, I'm client");
    //

    println!("{:?}", server_socket);

    server_socket.write(b"Hello, I'm your new client!").unwrap();
}