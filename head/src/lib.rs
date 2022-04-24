use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: usize,
    bytes: Option<usize>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("head")
        .version("0.1.0")
        .author("Marcin Rogowski<rogowskimarcin11@gmail.com")
        .about("Rust head")
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .default_value("10")
                .number_of_values(1)
                .takes_value(true)
                .value_name("LINES")
                .help("Number of lines"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .number_of_values(1)
                .takes_value(true)
                .conflicts_with("lines")
                .value_name("BYTES")
                .help("Number of bytes"),
        )
        .arg(
            Arg::with_name("files")
                .default_value("-")
                .min_values(1)
                .takes_value(true)
                .value_name("FILE")
                .help("File name to be read"),
        )
        .get_matches();

    let lines = matches
        .value_of("lines")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| format!("illegal line count -- {}", e))?;

    let bytes = matches
        .value_of("bytes")
        .map(parse_positive_int)
        .transpose()
        .map_err(|e| format!("illegal byte count -- {}", e))?;

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines: lines.unwrap(),
        bytes,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let len = config.files.len();    
    let is_not_len_1: bool = len > 1;

    for (i, filename) in config.files.into_iter().enumerate() {
        match open(&filename) {
            Err(e) => eprintln!("Failed to open {}: {}", filename, e),
            Ok(file) => {   
                if is_not_len_1 {
                    println!("==> {} <==", filename);
                }

                match config.bytes {
                    Some(c) => print_bytes(file, c)?,
                    None => print_lines(file, config.lines)?,
                };

                if is_not_len_1 && i+1 < len {
                    println!("");
                }
            }
        }
    }

    Ok(())
}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    match val.parse() {
        Ok(n) if n > 0 => Ok(n),
        _ => Err(From::from(val)),
    }
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn print_bytes(file: Box<dyn BufRead>,byte_count: usize) -> MyResult<()> {
    let mut handle = file.take(TryFrom::try_from(byte_count)?);
    let mut buffer = vec![0;byte_count];
    let n = handle.read(&mut buffer)?;

    print!("{}", String::from_utf8_lossy(&buffer[..n]));

    Ok(())
}

fn print_lines(mut file: Box<dyn BufRead>, line_count: usize) -> MyResult<()> {
    let mut buffer = String::new();
    for _ in 0..line_count {
        file.read_line(&mut buffer)?;
    }
    print!("{}", buffer);

    Ok(())
}

#[test]
fn test_parse_positive_int() {
    let res = parse_positive_int("3");
    assert!(res.is_ok());
    assert_eq!(res.unwrap(), 3);

    let res = parse_positive_int("foo");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "foo".to_string());

    let res = parse_positive_int("0");
    assert!(res.is_err());
    assert_eq!(res.unwrap_err().to_string(), "0".to_string());
}
