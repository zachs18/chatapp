// Parts of this file adapted from https://github.com/fdehau/tui-rs/blob/v0.16.0/examples/user_input.rs
// licensed by fdehau on GitHub and other tui-rs contributors under the MIT license

use std::net::*;
use std::io;

mod util;
use crate::util::*;

mod messages;
use crate::messages::*;

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

    // TUI init
    let mut terminal = tui::Terminal::new(
        tui::backend::TermionBackend::new(
            termion::screen::AlternateScreen::from(
                termion::input::MouseTerminal::from(
                    termion::raw::IntoRawMode::into_raw_mode(io::stdout())?
                )
            )
        )
    )?;

    let addr: SocketAddr = (ip, port).into();
    let mut stream = TcpStream::connect(addr)?;
    let mut name: String =
        // get first message, which should be a NameAssignment
        match Message::from_bytes(&recv_msg(&mut stream)?) {
            Some(Message::NameAssignment(name)) => name.into(),
            _ => {
                eprintln!("Server did not respond as expected.");
                Err(io::Error::new(io::ErrorKind::InvalidData, "Server did not send a NameAssignment message"))?;
                unreachable!("above Err should have caused an early return from main")
            }
        };
    let mut message_history: Vec<std::borrow::Cow<'static, str>> = vec![];
    let mut new_name: Option<String> = None;
    message_history.push(format!("Name: {}", name).into());

    // TODO: use tui crate with a window above for message history and a text entry box for message entry

    let (tx, input_rx) = std::sync::mpsc::channel();
    let _input_thread_handle = std::thread::spawn(move || -> io::Result<()> {
        use termion::input::TermRead;
        for event in io::stdin().keys() {
            tx.send(event?).unwrap();
        }
        Ok(())
    });


    let (tx, net_rx) = std::sync::mpsc::channel::<Message<'static>>();
    let stream_ = stream.try_clone()?;
    let _stream_thread_handle = std::thread::spawn(move || -> io::Result<()> {
        let mut stream = stream_;
        loop {
            let msg = recv_msg(&mut stream)?;
            match Message::from_bytes(&msg[..]) {
                Some(msg) => tx.send(msg.into_owned()).unwrap(),
                None => todo!(),
            };
        }
    });

    let mut input_line: String = String::new();
    loop {
        use Message::*;
        use std::sync::mpsc::TryRecvError;
        match net_rx.try_recv() {
            Ok(Disconnect) => {
                message_history.push("Disconnected".into());
                break;
            },
            Ok(ChatMessage(s)) => {
                message_history.push(s.into());
            },
            Ok(NameChangeApproval) => {
                name = new_name.take().unwrap();
                message_history.push(format!("New name: {}", name).into());
            },
            Ok(NameChangeDenial(reason)) => {
                let denied_name = new_name.take().unwrap();
                message_history.push(format!("Name request ({}) denied: {}.", denied_name, reason).into());
            },
            Err(TryRecvError::Empty) => {},
            _ => todo!(),
        };
        use termion::event::Key;
        match input_rx.try_recv() {
            Ok(Key::Char('\n')) => {
                if let Some(name_request) = input_line.strip_prefix("/name ") {
                    let name_request = name_request.trim();
                    new_name = Some(name_request.into());
                    let msg = Message::NameChangeRequest(name_request.into());
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    message_history.push(format!("You requested new name: {}", name_request).into());
                } else if input_line.starts_with("/disconnect") {
                    let msg = Message::Disconnect;
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    message_history.push("Disconnecting".into());
                    break;
                } else if input_line.starts_with("/") {
                    message_history.push(format!("Command not implemented: {}", input_line).into());
                } else if input_line.len() > 0 {
                    let msg = Message::ChatMessage(input_line.as_str().into());
                    let msg_bytes = msg.to_bytes();
                    send_msg(&mut stream, &msg_bytes)?;
                    message_history.push(format!("(you): {}", input_line).into());
                }
                input_line.clear();
            },
            Ok(Key::Backspace) => {
                input_line.pop();
            },
            Ok(Key::Char(c)) => {
                input_line.push(c);
            },
            Ok(Key::Ctrl('d')) | Err(TryRecvError::Disconnected) => {
                let msg = Message::Disconnect;
                let msg_bytes = msg.to_bytes();
                send_msg(&mut stream, &msg_bytes)?;
                message_history.push("Disconnecting".into());
                break;
            },
            Ok(k) => {
                message_history.push(format!("Key not implemented: {:?}", k).into());
            },
            Err(TryRecvError::Empty) => {},
        };

        terminal.draw(|f| {
            use tui::layout::{Constraint, Direction, Layout};
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(2)
                .constraints(
                    [
                        Constraint::Length(1),
                        Constraint::Length(3),
                        Constraint::Min(1),
                    ].as_ref()
                ).split(f.size());

            use tui::text::Text;
            use tui::widgets::{Paragraph, Block, Borders, List, ListItem};
//            use tui::text::{Text, Spans, Span};
//            use tui::style::{Style, Color, Modifier};

            let name_box = Paragraph::new(Text::from(format!("Name: {}", name)));
            f.render_widget(name_box, chunks[0]);

            let input_prompt = Paragraph::new(Text::from(&*input_line))
                .block(Block::default().borders(Borders::ALL).title("Input"));
            f.render_widget(input_prompt, chunks[1]);

            let messages: List = List::new(
                message_history.iter()
                    .map(|s| ListItem::new(&**s))
                    .collect::<Vec<_>>()
            ).block(Block::default().borders(Borders::ALL).title("Messages"));
            f.render_widget(messages, chunks[2]);
        })?;

        std::thread::sleep(std::time::Duration::from_millis(50));
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

    terminal.draw(|f| {
        use tui::layout::{Constraint, Direction, Layout};
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .margin(2)
            .constraints(
                [
                    Constraint::Length(4),
                    Constraint::Min(1),
                ].as_ref()
            ).split(f.size());

        use tui::text::Text;
        use tui::widgets::{Paragraph, Block, Borders, List, ListItem};
//            use tui::text::{Text, Spans, Span};
//            use tui::style::{Style, Color, Modifier};

        let disconnected_box = Paragraph::new(Text::from("Disconnected"));
        f.render_widget(disconnected_box, chunks[0]);

        let messages: List = List::new(
            message_history.iter()
                .map(|s| ListItem::new(&**s))
                .collect::<Vec<_>>()
        ).block(Block::default().borders(Borders::ALL).title("Messages"));
        f.render_widget(messages, chunks[1]);
    })?;

    std::thread::sleep(std::time::Duration::from_millis(1000));
    Ok(())
}
