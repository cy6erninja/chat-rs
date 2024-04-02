use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    // 1. We need to connect to a server socket to be able to send messages there.
    let mut server_socket = TcpStream::connect("127.0.0.1:8000").unwrap();
    // 2. To potentially improve performance by reducing IO operations, we need a write buffer.
    let mut bufwriter = BufWriter::new(&server_socket);
    let mut bufreader = BufReader::new(&server_socket);

    println!("{}", "Hello, I'm client");

    bufwriter.write(b"Hello, I'm your new client!\n").unwrap();


    loop {
        let mut line = String::new();
        bufwriter.write(b"Ping!\n").unwrap();
        bufwriter.flush();

        if bufreader.read_line(&mut line).unwrap() > 0 {
            let l = line.chars().filter(|c| *c != '\n').collect::<String>();
            println!("{l:?}");
            line.clear();
        }

        sleep(Duration::new(1, 0));
    }
}
