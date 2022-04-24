fn main() {
    if let Err(e) = head::get_args().and_then(head::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
