use clap::{App, Arg};
use regex::Regex;
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::str::FromStr;
use walkdir::{DirEntry, WalkDir};

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, Eq, PartialEq)]
enum EntryType {
    Dir,
    File,
    Link,
}

#[derive(Debug)]
pub struct Config {
    paths: Vec<String>,
    names: Vec<Regex>,
    entry_types: Vec<EntryType>,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("find")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com>")
        .about("Rust find")
        .arg(
            Arg::with_name("paths")
                .multiple(true)
                .value_name("PATH")
                .default_value(".")
                .help("Search paths"),
        )
        .arg(
            Arg::with_name("names")
                .short("n")
                .long("name")
                .value_name("NAME")
                .multiple(true)
                .help("Name"),
        )
        .arg(
            Arg::with_name("types")
                .short("t")
                .long("type")
                .value_name("TYPE")
                .multiple(true)
                .possible_values(&["f", "d", "l"])
                .help("Entry type"),
        )
        .get_matches();

    Ok(Config {
        paths: matches
            .values_of("paths")
            .unwrap()
            .map(|p| p.to_string())
            .collect(),
        names: matches
            .values_of("names")
            .unwrap_or_default()
            .map(|n| Regex::new(n).map_err(|_| format!("Invalid --name \"{}\"", n)))
            .collect::<Result<Vec<_>, _>>()?,
        entry_types: matches
            .values_of("types")
            .unwrap_or_default()
            .map(|t| EntryType::from_str(t))
            .collect::<Result<Vec<_>, _>>()?,
    })
}

fn filter_type(entry: DirEntry, entry_types: &Vec<EntryType>) -> Option<DirEntry> {
    if entry_types.is_empty()
        || entry_types.iter().any(|entry_type| match entry_type {
            EntryType::Dir => entry.file_type().is_dir(),
            EntryType::File => entry.file_type().is_file(),
            EntryType::Link => entry.file_type().is_symlink(),
        })
    {
        Some(entry)
    } else {
        None
    }
}

fn filter_name(entry: DirEntry, name: &Vec<Regex>) -> Option<DirEntry> {
    if name.is_empty()
        || name
            .iter()
            .any(|regex| regex.is_match(&entry.file_name().to_string_lossy()))
    {
        Some(entry)
    } else {
        None
    }
}

pub fn run(config: Config) -> MyResult<()> {
    for path in config.paths {
        for entry in WalkDir::new(path) {
            match entry {
                Ok(entry) => {
                    if let Some(entry) = filter_type(entry, &config.entry_types)
                        .and_then(|entry| filter_name(entry, &config.names))
                    {
                        println!("{}", entry.path().display())
                    }
                }
                Err(e) => eprintln!("{}", e),
            }
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct EntryTypeError {
    entry_type: String,
}

impl Display for EntryTypeError {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        write!(f, "{} is not a valid value for EntryType", self.entry_type)
    }
}

impl Error for EntryTypeError {}

impl FromStr for EntryType {
    type Err = EntryTypeError;

    fn from_str(input: &str) -> Result<Self, Self::Err> {
        match input {
            "f" => Ok(EntryType::File),
            "d" => Ok(EntryType::Dir),
            "l" => Ok(EntryType::Link),
            _ => Err(EntryTypeError {
                entry_type: input.to_string(),
            }),
        }
    }
}
