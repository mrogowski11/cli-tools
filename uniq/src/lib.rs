use clap::{App, Arg};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader, BufWriter, Write},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    in_file: String,
    out_file: Option<String>,
    count: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("uniq")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com")
        .about("Rust uniq")
        .arg(
            Arg::with_name("in_file")
                .default_value("-")
                .help("Input file")
                .value_name("IN_FILE"),
        )
        .arg(
            Arg::with_name("out_file")
                .help("Output file")
                .value_name("OUT_FILE"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .takes_value(false)
                .help("Show counts"),
        )
        .get_matches();

    Ok(Config {
        in_file: matches.value_of("in_file").unwrap().to_string(),
        out_file: matches.value_of("out_file").map(str::to_string),
        count: matches.is_present("count"),
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn write(filename: &Option<String>) -> MyResult<Box<dyn Write>> {
    match filename {
        Some(n) => Ok(Box::new(BufWriter::new(File::create(n)?))),
        None => Ok(Box::new(BufWriter::new(io::stdout().lock()))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    let mut file = open(&config.in_file).map_err(|e| format!("{}: {}", config.in_file, e))?;
    let mut outfile =
        write(&config.out_file).map_err(|e| format!("{:#?}: {}", config.out_file, e))?;
    let mut line = String::new();
    let mut bytes = file.read_line(&mut line)?;
    let mut prev_line = line.clone();
    let mut count: usize = 0;
    loop {
        if bytes == 0 && prev_line.len() == 0 {
            break;
        }

        if line.trim() == prev_line.trim() {
            count += 1;
        } else {
            outfile.write_all(
                format!("{}{}", format_field(count, config.count), prev_line).as_bytes(),
            )?;
            count = 1;
            prev_line = line.clone();
        }

        line.clear();
        bytes = file.read_line(&mut line)?;
    }
    outfile.flush().unwrap();
    Ok(())
}

fn format_field(value: usize, is_present: bool) -> String {
    if is_present {
        format!("{:>4} ", value)
    } else {
        "".to_string()
    }
}
