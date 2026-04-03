fn main() {
    if let Err(error) = strix::run(std::env::args()) {
        eprintln!("error: {error}");
        std::process::exit(1);
    }
}
