fn main() {
    if let Err(e) = fortune::get_args().and_then(fortune::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
