use std::io::{BufRead, BufReader, Read};
use std::net::{TcpListener, TcpStream};
use std::os::fd::{AsFd, AsRawFd, BorrowedFd, FromRawFd, RawFd};
use std::ptr::{null, null_mut};
use libc::{timespec, uintptr_t};

fn main() {
    let listener = TcpListener::bind("127.0.0.1:8000").unwrap();

    loop {
        println!("Server is waiting for clients...");

        let socket = match listener.accept() {
            Ok((socket, _addr)) => socket,
            Err(e) => panic!("{}", e),
        };
        let borrowed_fd = socket.as_fd();

        unsafe {
            // Create kernel queue
            let kq_fd = libc::kqueue();
            println!("Kernel queue has been created: {:?}", kq_fd);

            let mut ke = libc::kevent {
                ident: borrowed_fd.as_raw_fd() as uintptr_t,
                filter: libc::EVFILT_READ,
                flags: libc::EV_ADD,
                fflags: 0,
                data: 0,
                udata: null_mut(),
            };

            // let timeout = timespec { tv_sec: 5, tv_nsec: 0 };

            let mut events = Vec::with_capacity(1);

            let n = libc::kevent(
                kq_fd,
                &mut ke as *mut libc::kevent,
                1,
                events.as_mut_ptr(),
                1,
                null()
            );

            if n == -1 {
                panic!("Error occurred: {}", std::io::Error::last_os_error());
            }

            events.set_len(1);

            let event = events.pop().unwrap();
            let id = event.ident as RawFd;
            let socket = TcpStream::from_raw_fd(id);

            let bufreader = BufReader::new(socket);

            for line in bufreader.lines() {
                println!("{}", line.unwrap());
            }

            println!("Come client connected and wrote a message! {:?}", n);
        }
    }
}
