use clap::{App, Arg};
use std::error::Error;
use std::fs::File;
use std::io::{self, BufRead, BufReader};

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    number_lines: bool,
    number_nonblank_lines: bool,
}

type MyResult<T> = Result<T, Box<dyn Error>>;

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("catr")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com>")
        .about("Rust cat")
        .arg(
            Arg::with_name("files")
                .value_name("FILES")
                .help("Input files")
                .min_values(1)
                .required(true)
                .default_value("-"),
        )
        .arg(
            Arg::with_name("number_lines")
                .help("Number lines")
                .takes_value(false)
                .short("n")
                .long("number"),
        )
        .arg(
            Arg::with_name("number_nonblank_lines")
                .help("Number nonblank lines")
                .takes_value(false)
                .short("b")
                .long("number-nonblank")
                .conflicts_with("number_lines"),
        )
        .get_matches();

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        number_lines: matches.is_present("number_lines"),
        number_nonblank_lines: matches.is_present("number_nonblank_lines"),
    })
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in config.files {
        match open(&filename) {
            Err(err) => eprintln!("Failed to open {}: {}", filename, err),
            Ok(buffer) => {
                if config.number_lines {
                    print_number_lines(buffer)?;
                } else if config.number_nonblank_lines {
                    print_number_nonblank_lines(buffer)?;
                } else {
                    print_lines(buffer)?;
                }
            }
        }
    }

    Ok(())
}

fn print_number_lines(buffer: Box<dyn BufRead>) -> MyResult<()> {
    for (i, line) in buffer.lines().enumerate() {
        println!("{:>6}\t{}", i + 1, line?);
    }

    Ok(())
}

fn print_number_nonblank_lines(buffer: Box<dyn BufRead>) -> MyResult<()> {
    let mut empty_line_count = 0;

    for (i, line) in buffer.lines().enumerate() {
        let line = line?;

        if line.is_empty() {
            empty_line_count += 1;
            println!();
        } else {
            let line_number = i + 1 - empty_line_count;
            println!("{:>6}\t{}", line_number, line);
        }
    }

    Ok(())
}

fn print_lines(buffer: Box<dyn BufRead>) -> MyResult<()> {
    for line in buffer.lines() {
        println!("{}", line?);
    }

    Ok(())
}
