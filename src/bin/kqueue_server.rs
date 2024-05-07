use std::collections::HashMap;
use std::io::{BufRead, BufReader, Write};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsFd, AsRawFd};
use std::ptr::null_mut;
use std::sync::{Arc, Condvar, mpsc, Mutex};
use std::sync::mpsc::{Receiver, RecvError, Sender};
use std::thread;
use libc::{kqueue, socket};

type SocketRawFileDescriptor = usize;

struct Message {
    pub from: SocketRawFileDescriptor,
    pub message: String,
}

enum BroadcastRequest {
    AddSocket(TcpStream),
    RemoveSocket(SocketRawFileDescriptor),
    BroadcastMessage(SocketRawFileDescriptor)
}

enum BroadcastResponse {
    Acknowledged(SocketRawFileDescriptor)
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
            let n = unsafe {
                libc::kevent(
                    kq_fd as libc::c_int,
                    null_mut(),
                    0,
                    event_queue.as_mut_ptr(),
                    256,
                    null_mut()
                )
            };

            if n == -1 {
                panic!("Error occurred: {}", std::io::Error::last_os_error());
            }

            // TODO: handle errors.

            println!("Listening: {:?}", n);

            // Vector length is not updated automatically in unsafe mode, so we do it manually.
            unsafe {
                event_queue.set_len(n as usize);
            }

            while let event = event_queue.pop() {
                match event {
                    Some(k) => {
                        if k.flags as i16 & libc::EVFILT_READ != 0 {
                            println!("READ");
                            sender.send(BroadcastRequest::BroadcastMessage(k.ident)).unwrap();
                        }

                        if k.flags & libc::EV_EOF != 0 {
                            println!("EOF");
                            sender.send(BroadcastRequest::RemoveSocket(k.ident)).unwrap()
                        }

                        match ack_receiver.recv() {
                            Ok(res) => {
                                match res {
                                    BroadcastResponse::Acknowledged(fd) => {
                                        if fd != k.ident {
                                            panic!("Wrong file descriptor acknowledgement received!");
                                        }
                                    }
                                }
                            }
                            Err(e) => {panic!("{}", e)}
                        }
                    },
                    None => {
                        println!("None!");
                        break;
                    }
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
                }
                BroadcastRequest::RemoveSocket(fd) => {
                    // Removing dead socket.
                    clients.remove(&(fd as usize));
                }
                BroadcastRequest::BroadcastMessage(fd) => {
                    let message_sender = clients.get(&fd).unwrap();
                    let mut buf = BufReader::new(message_sender);

                    // let Some(line) = buf.lines().next();
                    let mut tmp = String::new();

                    buf.read_line(&mut tmp);
                    for (client_fd, mut socket) in &clients {
                        socket.write(tmp.as_bytes()).unwrap();
                    }

                    ack_sender.send(BroadcastResponse::Acknowledged(fd)).unwrap()
                }
            }
        };
    });
}

fn accept_clients(sender: Sender<BroadcastRequest>) {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();

    // Accept new clients.
    loop {
        match listener.accept() {
            Ok((socket, addr)) => {
                sender.send(BroadcastRequest::AddSocket(socket)).unwrap();
            },
            Err(e) => panic!("{}", e),
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
            panic!("Error occurred: {}", std::io::Error::last_os_error());
        }

        // TODO: handle errors.

        println!("New socket has been registered...");
    }
}