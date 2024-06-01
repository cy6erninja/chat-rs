use std::collections::HashMap;
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsFd, AsRawFd};
use std::ptr::null_mut;
use std::sync::mpsc;
use std::sync::mpsc::{Receiver, Sender};
use std::thread;
use libc::{kevent, kqueue};

#[macro_export]
macro_rules! kevents {
    ( $kq:expr, $eq:expr ) => {
        {
           let e = unsafe {
                libc::kevent(
                    $kq as libc::c_int,
                    null_mut(),
                    0,
                    $eq.as_mut_ptr(),
                    256,
                    null_mut()
                )
            };

            if e == -1 {
                panic!("Can't fetch events from kqueue: {}", std::io::Error::last_os_error());
            }

            unsafe {
                 $eq.set_len(e as usize);
            }

            e
        }

    }
}

type SocketRawFileDescriptor = usize;

struct Message {
    pub from: SocketRawFileDescriptor,
    pub to: Option<SocketRawFileDescriptor>,
    pub message: String,
}

enum BroadcastRequest {
    AddSocket(TcpStream),
    RemoveSocket(SocketRawFileDescriptor),
    BroadcastMessage(SocketRawFileDescriptor)
}

enum BroadcastResponse {
    MessageAcknowledged(SocketRawFileDescriptor)
}

fn main() {
    let (
        sender,
        receiver
    ) = mpsc::channel::<BroadcastRequest>();

    let (
        ack_sender,
        ack_receiver
    ) = mpsc::channel::<BroadcastResponse>();

    let kq_fd = unsafe { kqueue() } as usize;

    spawn_kqueue_thread(kq_fd, sender.clone(), ack_receiver);
    spawn_broadcast_thread(kq_fd, receiver, ack_sender);
    accept_clients(sender);
}

fn spawn_kqueue_thread(
    kq_fd: usize,
    sender: Sender<BroadcastRequest>,
    ack_receiver: Receiver<BroadcastResponse>
) {
    thread::spawn(move || {
        let mut event_queue = Vec::<libc::kevent>::with_capacity(256);

        loop {
            let n = kevents!(kq_fd as libc::c_int, event_queue);
            // let n = unsafe {
            //     libc::kevent(
            //         kq_fd as libc::c_int,
            //         null_mut(),
            //         0,
            //         event_queue.as_mut_ptr(),
            //         256,
            //         null_mut()
            //     )
            // };

            // if n == -1 {
            //     panic!("Can't fetch events from kqueue: {}", std::io::Error::last_os_error());
            // }

            // Vector length is not updated automatically in unsafe mode, so we do it manually.
            // unsafe {
            //     event_queue.set_len(n as usize);
            // }

            while let event = event_queue.pop() {
                match event {
                    Some(k) => {
                        if k.flags as i16 & libc::EVFILT_READ != 0 {
                            sender.send(BroadcastRequest::BroadcastMessage(k.ident)).unwrap();
                        }

                        if k.flags & libc::EV_EOF != 0 {
                            sender.send(BroadcastRequest::RemoveSocket(k.ident)).unwrap()
                        }

                        match ack_receiver.recv() {
                            Ok(res) => {
                                match res {
                                    BroadcastResponse::MessageAcknowledged(fd) => {
                                        if fd != k.ident {
                                            println!("Wrong file descriptor acknowledgement received!");

                                            continue;
                                        }
                                    }
                                }
                            }
                            Err(e) => {
                                println!("Could not acknowledge message because of error: {}", e);

                                continue;
                            }
                        }
                    },
                    None => {}
                }
            }
        }
    });
}

fn spawn_broadcast_thread(kq_fd: usize, receiver: Receiver<BroadcastRequest>, ack_sender: Sender<BroadcastResponse>) {
    // Map raw socket file descriptor to socket tcp stream.
    let mut clients = HashMap::<usize, TcpStream>::new();

    thread::spawn(move || {
        loop {
            match receiver.recv().unwrap() {
                BroadcastRequest::AddSocket(socket) => {
                    add_socket_listener(kq_fd, &socket);
                    clients.insert(socket.as_raw_fd() as usize, socket);
                    println!("Client connected...");
                }
                BroadcastRequest::RemoveSocket(fd) => {
                    // Removing dead socket.
                    clients.remove(&(fd as usize));
                    println!("Client disconnected...");
                }
                BroadcastRequest::BroadcastMessage(fd) => {
                    let mut message_sender = match clients.get(&fd) {
                        Some(socket) => { BufReader::new(socket) }
                        None => {
                            println!("Socket with file descriptor {} is not found...", &fd);

                            continue;
                        }
                    };


                    let mut len_buf = [0u8; 4];
                    message_sender.read_exact(&mut len_buf).unwrap();

                    let mut message = vec![0u8; u32::from_be_bytes(len_buf) as usize];
                    message_sender.read_exact(&mut message).unwrap();
                    println!("{:?}", &message);

                        // Ok(_) => {}
                        // Err(e) => {
                        //     println!("Could not read a message from socket {}", e);
                        //
                        //     continue;
                        // }
                    // };
                    for (_, mut socket) in &clients {
                        match socket.write(&message) {
                            Ok(_) => {}
                            Err(e) => {
                                println!("Could not broadcast a message {}", e);

                                continue;
                            }
                        }
                    }

                    match ack_sender.send(BroadcastResponse::MessageAcknowledged(fd)) {
                        Ok(_) => {}
                        Err(e) => {
                            println!("Could not send message acknowledgement {}", e);
                        }
                    };
                }
            }
        };
    });
}

fn accept_clients(sender: Sender<BroadcastRequest>) {
    let server_addr = std::env::args().nth(1);

    let server_addr = match server_addr {
        Some(addr) => addr,
        None => {
            panic!("Please, provide server address and port in the first arg e.g. 127.0.0.1:8000");
        }
    };

    let listener = match TcpListener::bind(server_addr) {
        Ok(l) => l,
        Err(e) => {
            println!("Could not start a server: {}", e);

            return;
        }
    };

    // Accept new clients.
    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                sender.send(BroadcastRequest::AddSocket(socket)).unwrap();
            },
            Err(e) => println!("Some client could not connect: {}", e.to_string()),
        };
    }
}

fn add_socket_listener(kq_fd: usize, socket: &TcpStream) {
    let mut ke = libc::kevent {
        // Raw file descriptor of the socket we want to listen.
        ident: socket.as_fd().as_raw_fd() as libc::uintptr_t,
        // Filter events we are interested in.
        filter: libc::EVFILT_READ,
        // Operation that we want to do with our kevent.
        // In this case we add it to kqueue.
        flags: libc::EV_ADD | libc::EV_EOF,
        fflags: 0,
        data: 0,
        udata: null_mut(),
    };

    unsafe {
        // Here we want only to register our socket for listening in kqueue.
        // The actual listening will happen in a dedicated thread.
        let n = libc::kevent(
            // kqueue file descriptor where we want to add an event listener.
            kq_fd as libc::c_int,
            // kevent struct that specifies details of the event we are registering.
            &mut ke as *mut libc::kevent,
            // Number of changes. We only want to listen to one socket here.
            1,
            // We don't want to listen for events here so we don't define the place
            // where events should be put. This would block the thread.
            null_mut(),
            // Consequently, we don't need to define how many events max we expect.
            0,
            // Timeout. We don't need to specify it anywhere since we want to wait
            // for new socket events indefinitely.
            null_mut()
        );

        if n == -1 {
            println!(
                "Error occurred when registering socket in kqueue: {}",
                std::io::Error::last_os_error()
            );

            return;
        }

        println!("New socket listener has been added to kqueue. Socket fd: {}", ke.ident as usize);
    }
}
