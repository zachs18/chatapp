use std::net::*;
use std::io::{self, prelude::*};

mod get_user_input;
use crate::get_user_input::*;

fn main() -> io::Result<()> {
    let ip_and_maybe_port: (IpAddr, Option<u16>) = get_user_input(
        io::stdout().lock(),
        io::stdin().lock(),
        "Server IP: ",
        "Invalid IP.\n",
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
            "Invalid port.\n",
            |s| s.trim().parse().ok()
        )?,
    };

    let server_addr: SocketAddr = (ip, port).into();
    let listener = TcpListener::bind(server_addr)?;

    { // TODO
        let (mut stream, addr) = listener.accept()?;

        let mut x = [0u8];
        stream.read_exact(&mut x)?;
        let mut s = String::new();
        Read::take(&stream, x[0] as u64).read_to_string(&mut s)?;

        println!("Received \"{}\" from {:?}.", s, addr);

        let s = format!("Hello, {:?}.", addr);
        stream.write(&[s.len() as u8])?;
        stream.write(s.as_bytes())?;
    }

    Ok(())
}
