use std::io::{BufRead, BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use std::thread;
use std::thread::sleep;
use std::time::Duration;

fn main() {
    let mut server_socket = TcpStream::connect("127.0.0.1:8000").unwrap();
    let mut bufwriter = BufWriter::new(server_socket.try_clone().unwrap());
    let mut bufreader = BufReader::new(server_socket);

    println!("{}", "Hello, I'm client");

    bufwriter.write(b"Hello, I'm your new client!\n").unwrap();
    bufwriter.flush();

    thread::spawn(move || {
        loop {
            bufwriter.write(b"Ping!\n").unwrap();
            bufwriter.flush();

            sleep(Duration::new(2, 0));
        }
    });

    loop {
        let mut line = String::new();

        if bufreader.read_line(&mut line).unwrap() > 0 {
            let l = line.chars().filter(|c| *c != '\n').collect::<String>();
            println!("{l:?}");
            line.clear();
        }
    }
}
