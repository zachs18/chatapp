use std::net::*;
use std::io::{self, prelude::*};

mod get_user_input;
use crate::get_user_input::*;

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
    let mut socket = TcpStream::connect(addr)?;

    { // TODO
        let s = format!("Hello, {:?}.", addr);
        socket.write(&[s.len() as u8])?;
        socket.write(s.as_bytes())?;

        let mut x = [0u8];
        socket.read_exact(&mut x)?;
        let mut s = String::new();
        Read::take(&socket, x[0] as u64).read_to_string(&mut s)?;
        println!("Received \"{}\" from {:?}.", s, addr);
    }

    Ok(())
}
