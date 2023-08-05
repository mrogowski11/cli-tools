fn main() {
    if let Err(e) = tail::get_args().and_then(tail::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
