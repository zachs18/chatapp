use std::net::*;
use std::io::{self, prelude::*};
use std::os::unix::io::AsRawFd;

mod util;
use crate::util::*;

mod messages;
use crate::messages::*;

enum StdinOrServer {
    Stdin,
    Server,
}

fn main() -> io::Result<()> {
    let ip_and_maybe_port: (IpAddr, Option<u16>) = get_user_input(
        io::stdout().lock(),
        io::stdin().lock(),
        "Server IP: ",
        "Invalid IP",
        |s| -> Option<(IpAddr, Option<u16>)> {
            let s = s.trim();
            match s.parse::<SocketAddr>() {
                Ok(socket) => Some((socket.ip(), Some(socket.port()))),
                Err(_) => match s.parse() {
                    Ok(ip) => Some((ip, None)),
                    Err(_) => None,
                },
            }
        }
    )?;
    let ip = ip_and_maybe_port.0;
    let port: u16 = match ip_and_maybe_port.1 {
        Some(port) => port,
        None => get_user_input(
            io::stdout().lock(),
            io::stdin().lock(),
            "Server port: ",
            "Invalid port",
            |s| s.trim().parse().ok()
        )?,
    };

    let addr: SocketAddr = (ip, port).into();
    let mut stream = TcpStream::connect(addr)?;
    let mut name: String =
        // get first message, which should be a NameAssignment
        match Message::from_bytes(&recv_msg(&mut stream)?) {
            Some(Message::NameAssignment(name)) => name.into(),
            _ => {
                eprintln!("Server did not respond as expected.");
                Err(io::Error::new(io::ErrorKind::InvalidData, "Server did not send a NameAssignment message"))?;
                unreachable!()
            }
        };
    println!("Name: {}", name);

    let mut input_line: String = String::new();
    let mut stdin = io::stdin();
    loop {
        match poll_in(
            vec![(StdinOrServer::Stdin, &mut stdin as &mut AsRawFd), (StdinOrServer::Server, &mut stream)].into_iter(),
            50
        )? {
            Some((StdinOrServer::Stdin, _)) => {
                todo!("read from stdin and send to server");
            },
            Some((StdinOrServer::Server, _)) => {
                let msg = recv_msg(&mut stream)?;
                use Message::*;
                match Message::from_bytes(&msg[..]) {
                    Some(Disconnect) => {
                        println!("Disconnected.");
                        todo!("formatted output");
                    },
                    Some(ChatMessage(s)) => {
                        println!("{}", s);
                        todo!("formatted output");
                    },
                    _ => todo!(),
                };
            },
            None => {},
        };
//        let s = format!("Hello, {:?}.", addr);
//        socket.write(&[s.len() as u8])?;
//        socket.write(s.as_bytes())?;
//
//        let mut x = [0u8];
//        socket.read_exact(&mut x)?;
//        let mut s = String::new();
//        Read::take(&socket, x[0] as u64).read_to_string(&mut s)?;
//        println!("Received \"{}\" from {:?}.", s, addr);
    }

    Ok(())
}
