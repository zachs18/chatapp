use std::io::{self, prelude::*};
use std::os::unix::io::AsRawFd;
use libc::{poll, pollfd, POLLIN};
use std::convert::TryInto;

//use crate::messages::Message;
// TODO: maybe make send_msg and recv_msg use Message? Maybe by having Message keep a cached to_bytes?
// But then it couldn't really be an enum so maybe don't do that and keep how I have it now
// where to_bytes is called explicitly

/// Repeatedly prompts user with prompt on output and gets a line of user input from input
/// until parser(line of input) returns Some(value), then returns Ok(value)
pub fn get_user_input<T>(
    mut output: impl Write,
    mut input: impl BufRead,
    prompt: &str,
    errmsg: &str,
    parser: impl Fn(&str) -> Option<T>
) -> io::Result<T> {
    let mut string = String::new();
    loop {
        write!(output, "{}", prompt)?;
        output.flush()?;
        string.clear();
        input.read_line(&mut string)?;
        match parser(&string) {
            Some(value) => break Ok(value),
            None => write!(output, "{}", errmsg)?,
        };
    }
}

/// timeout < 0 -> block forever
/// timeout == 0 -> return immediately
/// timeout > 0 -> block for timeout milliseconds
/// Returns the index of the first Fd that was ready for reading,
/// or None if none were ready before the timeout, or some errored.
#[allow(dead_code)] // only used in server
pub fn poll_in<'a, K, F: AsRawFd + ?Sized>(fds: impl Iterator<Item=(K, &'a mut F)>, timeout: i32) -> io::Result<Option<(K, &'a mut F)>> {
    let (mut pollfds, refs): (Vec<pollfd>, Vec<(K, &'a mut F)>) = fds.map(
        |arf| { (
            pollfd {
                fd: arf.1.as_raw_fd(),
                events: POLLIN,
                revents: 0,
            },
            arf
        )}
    ).unzip();
    
    let ret = unsafe {
        poll(pollfds.as_mut_ptr(), pollfds.len().try_into().unwrap(), timeout)
    };

    if ret == 0 {
        Ok(None)
    } else if ret > 0 {
        Ok(pollfds.iter().zip(refs.into_iter()).filter_map(
            |(pollfd, r)| if pollfd.revents & POLLIN != 0 {
                Some(r)
            } else {
                None
            }
        ).next())
    } else { // ret < 0
        Err(io::Error::last_os_error())
    }
}

pub fn send_msg(destination: &mut impl io::Write, msg: &[u8]) -> io::Result<()> {
    let len: u32 = msg.len().try_into().unwrap();
    destination.write_all(&len.to_le_bytes())?;
    destination.write_all(msg)?;
    Ok(())
}

pub fn recv_msg(src: &mut impl io::Read) -> io::Result<Vec<u8>> {
    let mut len_buf: [u8; 4] = [0; 4];
    src.read_exact(&mut len_buf[..])?;
    let len: u32 = u32::from_le_bytes(len_buf);
    let mut data = vec![0u8; len.try_into().unwrap()];
    src.read_exact(&mut data[..])?;
    Ok(data)
}
