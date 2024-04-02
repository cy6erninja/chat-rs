use std::io::{BufRead, BufReader, BufWriter, Write};
use std::net::{ SocketAddr, TcpListener, TcpStream };
use std::thread;

fn main() {
    println!("Hello, I'm Server!");

    // 1. We need to listed for incoming connections from users.
    //std::new::TcpListener is exactly that.

    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();
    let mut last_client_id: usize = 0;

    loop {
        match listener.accept() {
            Ok((socket, addr)) => handle_client(
                socket,
                addr,
                {last_client_id +=1; last_client_id}
            ),
            Err(e) => println!("couldn't get client: {e:?}")
        }
    }
}

fn handle_client(mut socket: TcpStream, addr: SocketAddr, client_id: usize) {

    // 2. We need to have each new connection in a new thread.
    let mut res = String::new();

    // let mut res: [u8;10]= [0; 10];
    //3. We need buffer from std::io to collect information from stream. This is because
    // when data is sent over TCP, it may become fragmented and should be gathered in a buffer
    // before using. Also, IO operations are more expensive than CPU operations, so reading
    // as much as possible from a stream helps.

    thread::spawn(move || {

        let mut bufwriter = BufWriter::new(&socket);
        let mut bufreader = BufReader::new(&socket);

        for line in bufreader.lines() {
            match line {
                Ok(l) =>
                    {
                        println!("Client {client_id:?}-> {l:?}");
                        if l.eq("Ping!") {
                            bufwriter.write(b"Pong!\n");
                            bufwriter.flush();
                        }
                    },
                Err(_) => {}
            };
        }
    });
}
