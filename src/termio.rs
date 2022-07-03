//! Terminal I/O, with colors!

use std::io::Write;
use termcolor::{Color, ColorChoice, ColorSpec, StandardStream, WriteColor};

/// Print an OK message, in green.
pub fn print_ok(msg: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Green)))
        .expect("set color");
    writeln!(&mut stdout, "OK    {}", msg).expect("write message");

    stdout.reset().expect("reset color");
}

/// Print a warning message, in yellow.
pub fn print_warn(msg: &str) {
    let mut stdout = StandardStream::stdout(ColorChoice::Auto);

    stdout
        .set_color(ColorSpec::new().set_fg(Some(Color::Yellow)))
        .expect("set color");
    writeln!(&mut stdout, "WARN  {}", msg).expect("write message");

    stdout.reset().expect("reset color");
}
