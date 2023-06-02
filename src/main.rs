use std::{net::{TcpListener, SocketAddr}, net::TcpStream, io::{Write, Read}, thread, os::{fd::{AsFd, RawFd, AsRawFd, FromRawFd, IntoRawFd}, unix::raw}, time::Duration};
use nix::{sys::time::TimeVal, poll::{PollFd, PollFlags}};
use nix::sys::select::FdSet;

fn handle_client(mut stream: &TcpStream) {
    match stream.write_all(b"+PONG\r\n") {
        Ok(_stream_writed) => {
            println!("{:?} - Response PONG", thread::current().name());
        }
        Err(e) => {
            println!("Error on write: {}", e);
        }
    }
}

pub struct Client{
    stream: TcpStream,
    fd: PollFd
}

impl Client {

    pub const fn new(stream: TcpStream, fd: PollFd) -> Client {
        Client {
            stream,
            fd
        }
    }
}

fn main() {
    println!("Logs from your program will appear here!");
    let listener = TcpListener::bind("127.0.0.1:6379").unwrap();

    listener.set_nonblocking(true).expect("Cannot set non-blocking");

    let listener_fd = listener.as_raw_fd();

    let mut clients: Vec<Client> = Vec::new();
    let mut fds_poll: Vec<PollFd> = Vec::new();
    loop {
        fds_poll.push(PollFd::new(listener_fd, PollFlags::POLLIN));

        fds_poll.append(&mut clients.iter().map(|f| f.fd).clone().collect());

        nix::poll::poll(&mut fds_poll, -1).unwrap();

        if fds_poll[0].revents().unwrap().contains(PollFlags::POLLIN) {
            loop {
                match listener.accept() {
                    Ok((stream, addr)) => {
                        println!("Accepted={:?}", addr);

                        stream.set_nonblocking(true).unwrap();
                        let a = stream.as_raw_fd();
                        let events = PollFlags::POLLOUT | PollFlags::POLLIN | PollFlags::POLLERR;                  
                        clients.push(Client::new(stream, PollFd::new(a, events)));
                    }
                    Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                        break;
                    }
                    Err(e) => {
                        panic!("Error acceppting new connection {}", e);
                    }
                }
            }
        }

        for i in 1..fds_poll.len() {
            let fd = fds_poll[i];
            let revents = fd.revents().unwrap();
            
            if revents.contains(PollFlags::POLLNVAL) {
                continue;
            }
            
            if revents.contains(PollFlags::POLLIN) {
                let mut tcp_stream = &clients.iter().find(|x| x.fd.as_raw_fd() == fd.as_raw_fd()).unwrap().stream;
                loop {
                    match tcp_stream.read(&mut [0; 1000]) {
                        Ok(bytes_received) => {
                            println!("bytes_received: {:?}", bytes_received);
                            if bytes_received == 0 {
                                tcp_stream.shutdown(std::net::Shutdown::Both).unwrap();
                                clients.retain(|s| fd.as_raw_fd() != s.fd.as_raw_fd());
                                break;
                            }

                            handle_client(tcp_stream);
                            println!("received data");
                        }
                         Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            break;
                         }
                        Err(e) => {
                            panic!("Read error stopping server {}", e);
                        }
                    }
                }
            }
        }

        fds_poll.clear();
    }
}

