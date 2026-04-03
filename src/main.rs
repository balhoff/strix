use std::io::{self, Write};

fn main() {
    strix::bench::restore_sigpipe();
    strix::bench::raise_fd_limit();

    if let Err(error) = strix::run(std::env::args_os()) {
        let message = error.to_string();
        let mut stderr = io::stderr().lock();
        let _ = stderr.write_all(message.as_bytes());
        if !message.ends_with('\n') {
            let _ = stderr.write_all(b"\n");
        }
        std::process::exit(1);
    }
}
