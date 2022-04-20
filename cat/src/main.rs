fn main() {
    if let Err(e) = cat::get_args().and_then(cat::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
