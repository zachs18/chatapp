use std::net::*;
use std::io;
use std::collections::HashMap;
use std::sync::{Mutex, Arc};
use std::time::Duration;

mod util;
use crate::util::*;

mod messages;
use crate::messages::*;

fn new_name_validity(clients: &HashMap<SocketAddr, (String, TcpStream)>, addr: SocketAddr, new_name: &str) -> Result<(), u8> {
    if new_name.len() == 0 {
        return Err(0);
    }
    for (other_addr, (other_name, _)) in clients.iter() {
        if &addr != other_addr && other_name == new_name {
            return Err(1);
        }
    }
    return Ok(());
}

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

    let clients_: Arc<Mutex<HashMap<SocketAddr, (String, TcpStream)>>> = Arc::new(Mutex::new(HashMap::new()));

    let clients__ = Arc::clone(&clients_);
    let _connection_handler_thread = std::thread::spawn(move || -> io::Result<()> {
        loop {
            let (mut stream, addr) = listener.accept()?;
            let mut clients = clients__.lock().unwrap();
            let name = format!("{}", addr);
            let msg = Message::NameAssignment((&name).into());
            send_msg(&mut stream, &msg.to_bytes())?;
            let joined_msg = Message::ChatMessage(format!("{} joined", name).into());
            let joined_msg_bytes = joined_msg.to_bytes();
            // send "{name} joined" message to all other clients
            for (_, stream) in clients.values_mut() {
                send_msg(stream, &joined_msg_bytes)?;
            }
            clients.insert(addr, (name, stream));
        }
    });

    loop {
        let mut clients = clients_.lock().unwrap();
        match poll_in(clients.iter_mut().map(|(addr, (name, stream))| ((addr, name), stream)), 0)? {
            Some(((addr, name), stream)) => {
                let msg = recv_msg(stream)?;
                let src_addr = *addr; // clients's .iter_mut() borrow should end here if name is not used?
                use Message::*;
                match Message::from_bytes(&msg[..]) {
                    Some(Disconnect) => {
                        let addr = *addr;
                        let (name, _stream) = clients.remove(&addr).unwrap();
                        let msg = ChatMessage(format!("{} disconnected", name).into());
                        let msg_bytes = msg.to_bytes();
                        for (_, stream) in clients.values_mut() {
                            send_msg(stream, &msg_bytes)?;
                        }
                    },
                    Some(ChatMessage(s)) => {
                        let msg = ChatMessage(format!("{}: {}", name, s).into());
                        let msg_bytes = msg.to_bytes();
                        for (dst_addr, (_, stream)) in clients.iter_mut() {
                            if &src_addr != dst_addr {
                                send_msg(stream, &msg_bytes)?;
                            }
                        }
                    },
                    Some(NameChangeRequest(new_name)) => {
                        let src_addr = *addr;
                        match new_name_validity(&*clients, src_addr, &new_name) {
                            Ok(()) => {
                                let mut new_name: String = new_name.into();
                                let (name, stream) = clients.get_mut(&src_addr).unwrap();
                                std::mem::swap(name, &mut new_name);
                                let old_name = new_name;
                                send_msg(stream, &NameChangeApproval.to_bytes())?;
                                let msg = ChatMessage(format!("{} is now known as {}", old_name, name).into());
                                let msg_bytes = msg.to_bytes();
                                for (dst_addr, (_, stream)) in clients.iter_mut() {
                                    if dst_addr != &src_addr {
                                        send_msg(stream, &msg_bytes)?;
                                    }
                                }
                            },
                            Err(reason) => {
                                let (_, stream) = clients.get_mut(&src_addr).unwrap();
                                send_msg(stream, &NameChangeDenial(reason).to_bytes())?;
                            },
                        }
                    },
                    _ => todo!(),
                };
            },
            None => {
                drop(clients); // explicitly unlock and sleep so other thread can lock the mutex
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
}
