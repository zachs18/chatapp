use std::net::*;
use std::io;
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
    let mut new_name: Option<String> = None;
    println!("Name: {}", name);

    // TODO: use tui crate with a window above for message history and a text entry box for message entry

    let mut input_line: String = String::new();
    let mut stdin = io::stdin();
    loop {
        match poll_in(
            vec![(StdinOrServer::Stdin, &mut stdin as &mut dyn AsRawFd), (StdinOrServer::Server, &mut stream)].into_iter(),
            50
        )? {
            Some((StdinOrServer::Stdin, _)) => {
                // TODO: handle non-full lines?
                input_line.clear();
                stdin.read_line(&mut input_line)?;
                if let Some(name_request) = input_line.strip_prefix("/name ") {
                    let name_request = name_request.trim();
                    new_name = Some(name_request.into());
                    let msg = Message::NameChangeRequest(name_request.into());
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    println!("You requested new name: {}", name_request);
                } else if input_line.starts_with("/disconnect") {
                    let msg = Message::Disconnect;
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    println!("Disconnecting");
                    break;
                } else if input_line.starts_with("/") {
                    println!("Command not implemented: {}", input_line);
                } else if input_line.len() > 0 {
                    let input_line = input_line.trim();
                    let msg = Message::ChatMessage(input_line.into());
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    println!("(you): {}", input_line);
                }
            },
            Some((StdinOrServer::Server, _)) => {
                let msg = recv_msg(&mut stream)?;
                use Message::*;
                match Message::from_bytes(&msg[..]) {
                    Some(Disconnect) => {
                        println!("Disconnected.");
                    },
                    Some(ChatMessage(s)) => {
                        println!("{}", s);
                    },
                    Some(NameChangeApproval) => {
                        name = new_name.take().unwrap();
                        println!("New name: {}", name);
                    },
                    Some(NameChangeDenial(reason)) => {
                        let denied_name = new_name.take().unwrap();
                        println!("Name request ({}) denied: {}.", denied_name, reason);
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
