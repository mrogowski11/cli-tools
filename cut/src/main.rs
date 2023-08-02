fn main() {
    if let Err(e) = cut::get_args().and_then(cut::run) {
        eprintln!("{}", e);
        std::process::exit(1);
    }
}
