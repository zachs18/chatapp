use std::net::*;
use std::io::{self, prelude::*};
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::time::Duration;

mod util;
use crate::util::*;

mod messages;
use crate::messages::*;

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

    let clients: Arc<Mutex<HashMap<SocketAddr, (String, TcpStream)>>> = Arc::new(Mutex::new(HashMap::new()));

    let clients_ = Arc::clone(&clients);
    let connection_handler_thread = std::thread::spawn(move || -> io::Result<()> {
        loop {
            let (mut stream, addr) = listener.accept()?;
            let mut clients = clients_.lock().unwrap();
            let name = format!("{}", addr);
            let msg = Message::NameAssignment((&name).into());
            send_msg(&mut stream, &msg.to_bytes());
            eprintln!("{} joined", name);
            clients.insert(addr, (name, stream));
            // TODO: send "{name} joined" message to all(?) clients
        }
    });

    loop {
        let mut clients_ = clients.lock().unwrap();
        match poll_in(clients_.iter_mut().map(|(addr, (name, stream))| ((addr, name), stream)), 0)? {
            Some(((addr, name), stream)) => {
                let msg = recv_msg(stream)?;
                let addr = *addr; // clients_'s .iter_mut() borrow should end here if name is not used?
                use Message::*;
                match Message::from_bytes(&msg[..]) {
                    Some(Disconnect) => clients_.remove(&addr),
                    Some(ChatMessage(s)) => {
                        let msg = format!("{}: {}", name, s);
                        todo!("send message to other clients");
                    },
                    Some(NameChangeRequest(new_name)) => {
                        clients_.get_mut(&addr).unwrap().0 = new_name.into();
                        // TODO: uniqueness checking
                        todo!();
                    },
                    _ => todo!(),
                };
            },
            None => {
                drop(clients_); // explicitly unlock and sleep so other thread can lock the mutex
                std::thread::sleep(Duration::from_millis(50));
            },
        };

//        let mut x = [0u8];
//        stream.read_exact(&mut x)?;
//        let mut s = String::new();
//        Read::take(&stream, x[0] as u64).read_to_string(&mut s)?;
//
//        println!("Received \"{}\" from {:?}.", s, addr);
//
//        let s = format!("Hello, {:?}.", addr);
//        stream.write(&[s.len() as u8])?;
//        stream.write(s.as_bytes())?;
    }

    Ok(())
}
