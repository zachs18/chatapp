use std::io::{self, prelude::*};

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
