fn main() {
    if let Err(e) = uniq::get_args().and_then(uniq::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
