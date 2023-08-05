use clap::{App, Arg};
use std::{
    cmp::Ordering::*,
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    file1: String,
    file2: String,
    show_col1: bool,
    show_col2: bool,
    show_col3: bool,
    insensitive: bool,
    delimiter: String,
}

enum Column<'a> {
    Col1(&'a str),
    Col2(&'a str),
    Col3(&'a str),
}
pub fn get_args() -> MyResult<Config> {
    let matches = App::new("comm")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com")
        .about("Rust comm")
        .arg(
            Arg::with_name("file1")
                .value_name("FILE1")
                .required(true)
                .help("Input file1"),
        )
        .arg(
            Arg::with_name("file2")
                .value_name("FILE2")
                .required(true)
                .help("Input file2"),
        )
        .arg(
            Arg::with_name("supress_col1")
                .short("1")
                .takes_value(false)
                .help("Supress printing of column 1"),
        )
        .arg(
            Arg::with_name("supress_col2")
                .short("2")
                .takes_value(false)
                .help("Supress printing of column 2"),
        )
        .arg(
            Arg::with_name("supress_col3")
                .short("3")
                .takes_value(false)
                .help("Supress printing of column 3"),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .takes_value(false)
                .help("Case-insensitive comparison of lines"),
        )
        .arg(
            Arg::with_name("delimiter")
                .short("d")
                .long("output-delimiter")
                .value_name("DELIM")
                .default_value("\t")
                .help("Output delimiter"),
        )
        .get_matches();

    Ok(Config {
        file1: matches.value_of_lossy("file1").unwrap().to_string(),
        file2: matches.value_of_lossy("file2").unwrap().to_string(),
        show_col1: !matches.is_present("supress_col1"),
        show_col2: !matches.is_present("supress_col2"),
        show_col3: !matches.is_present("supress_col3"),
        insensitive: matches.is_present("insensitive"),
        delimiter: matches.value_of("delimiter").unwrap().to_string(),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let file1 = &config.file1;
    let file2 = &config.file2;
    if file1 == "-" && file2 == "-" {
        return Err(From::from("Both input files cannot be STDIN (\"-\")"));
    }
    let mut lines1 = open(file1)?
        .lines()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|l| {
            if config.insensitive {
                l.to_lowercase()
            } else {
                l
            }
        });
    let mut lines2 = open(file2)?
        .lines()
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .map(|l| {
            if config.insensitive {
                l.to_lowercase()
            } else {
                l
            }
        });
    let mut line1 = lines1.next();
    let mut line2 = lines2.next();

    let print = |col: Column| {
        let mut columns = vec![];
        match col {
            Column::Col1(val) => {
                if config.show_col1 {
                    columns.push(val);
                }
            }
            Column::Col2(val) => {
                if config.show_col2 {
                    if config.show_col1 {
                        columns.push("");
                    }
                    columns.push(val);
                }
            }
            Column::Col3(val) => {
                if config.show_col3 {
                    if config.show_col1 {
                        columns.push("");
                    }
                    if config.show_col2 {
                        columns.push("");
                    }
                    columns.push(val);
                }
            }
        };
        if !columns.is_empty() {
            println!("{}", columns.join(&config.delimiter));
        }
    };
    loop {
        match (&line1, &line2) {
            (None, None) => break,
            (Some(l1), None) => {
                print(Column::Col1(l1));
                line1 = lines1.next();
            }
            (None, Some(l2)) => {
                print(Column::Col2(l2));
                line2 = lines2.next();
            }
            (Some(l1), Some(l2)) => match l1.cmp(l2) {
                Equal => {
                    print(Column::Col3(l1));
                    line1 = lines1.next();
                    line2 = lines2.next();
                }
                Less => {
                    print(Column::Col1(l1));
                    line1 = lines1.next();
                }
                Greater => {
                    print(Column::Col2(l2));
                    line2 = lines2.next();
                }
            },
        }
    }
    Ok(())
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(
            File::open(filename).map_err(|e| format!("{}: {}", filename, e))?,
        ))),
    }
}
