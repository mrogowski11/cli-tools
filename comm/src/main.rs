fn main() {
    if let Err(e) = comm::get_args().and_then(comm::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
