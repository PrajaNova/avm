//! Console lines every command prints the same way.

/// `✓ <message>` on stdout.
macro_rules! done {
    ($($arg:tt)*) => { println!("✓ {}", format_args!($($arg)*)) };
}

/// `warning: <message>` on stderr.
macro_rules! warn {
    ($($arg:tt)*) => { eprintln!("warning: {}", format_args!($($arg)*)) };
}
