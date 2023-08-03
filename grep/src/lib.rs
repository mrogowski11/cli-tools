use clap::{App, Arg};
use regex::{Regex, RegexBuilder};
use std::{
    error::Error,
    fs::File,
    io::{self, BufRead, BufReader},
};
use walkdir::WalkDir;

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug)]
pub struct Config {
    pattern: Regex,
    files: Vec<String>,
    recursive: bool,
    count: bool,
    invert_match: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("grep")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com")
        .about("Rust grep")
        .arg(
            Arg::with_name("pattern")
                .value_name("PATTERN")
                .required(true)
                .help("Search pattern"),
        )
        .arg(
            Arg::with_name("files")
                .value_name("FILE")
                .multiple(true)
                .default_value("-")
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("recursive")
                .short("r")
                .long("recursive")
                .help("Recursive search"),
        )
        .arg(
            Arg::with_name("count")
                .short("c")
                .long("count")
                .help("Count occurrences"),
        )
        .arg(
            Arg::with_name("invert_match")
                .short("v")
                .long("invert-match")
                .help("Invert match"),
        )
        .arg(
            Arg::with_name("insensitive")
                .short("i")
                .long("insensitive")
                .help("Case-insensitive"),
        )
        .get_matches();
    let insensitive = matches.is_present("insensitive");
    let pattern_args = &matches.value_of_lossy("pattern").unwrap();
    Ok(Config {
        pattern: RegexBuilder::new(pattern_args)
            .case_insensitive(insensitive)
            .build()
            .map_err(|_| format!("Invalid pattern \"{}\"", pattern_args))?,
        files: matches.values_of_lossy("files").unwrap(),
        recursive: matches.is_present("recursive"),
        count: matches.is_present("count"),
        invert_match: matches.is_present("invert_match"),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let entries = find_files(&config.files, config.recursive);
    for entry in &entries {
        match entry {
            Err(e) => eprintln!("{}", e),
            Ok(filename) => match open(&filename) {
                Err(e) => eprintln!("{}: {}", filename, e),
                Ok(file) => {
                    let matches = find_lines(file, &config.pattern, config.invert_match);
                    match matches {
                        Err(e) => eprintln!("{}", e),
                        Ok(lines) => {
                            print_matches(lines, &filename, entries.len() > 1, config.count)
                        }
                    }
                }
            },
        }
    }
    Ok(())
}

fn print_matches(mut matches: Vec<String>, filename: &str, multiple_entries: bool, count: bool) {
    if count {
        matches = vec![matches.len().to_string()];
    }
    for m in matches {
        println!(
            "{}{}",
            if multiple_entries {
                format!("{}:", filename)
            } else {
                "".to_owned()
            },
            m
        );
    }
}

fn find_files(paths: &[String], recursive: bool) -> Vec<MyResult<String>> {
    let mut files: Vec<MyResult<String>> = Vec::new();

    for path in paths {
        if path == "-" {
            files.push(Ok(path.to_owned()));
            continue;
        }
        let wd = WalkDir::new(path).follow_links(true);
        if recursive {
            for dir in wd {
                match dir {
                    Ok(d) => {
                        if d.path().is_file() {
                            files.push(Ok(d.path().to_string_lossy().to_string()));
                        }
                    }
                    Err(e) => {
                        if let Some(inner) = e.io_error() {
                            files.push(Err(From::from(format!("{}: {}", path, inner))));
                        } else {
                            files.push(Err(From::from(format!("Traversing error: {}", e))));
                        }
                    }
                };
            }
        } else {
            match wd.into_iter().next().unwrap() {
                Err(e) => {
                    if let Some(inner) = e.io_error() {
                        files.push(Err(From::from(format!("{}: {}", path, inner))));
                    } else {
                        files.push(Err(From::from(format!("Traversing error: {}", e))));
                    }
                }
                Ok(d) => match d.file_type().is_dir() {
                    true => files.push(Err(From::from(format!("{} is a directory", path)))),
                    false => files.push(Ok(d.path().to_string_lossy().to_string())),
                },
            }
        }
    }
    files
}

fn open(filename: &str) -> MyResult<Box<dyn BufRead>> {
    match filename {
        "-" => Ok(Box::new(BufReader::new(io::stdin()))),
        _ => Ok(Box::new(BufReader::new(File::open(filename)?))),
    }
}

fn find_lines<T: BufRead>(file: T, pattern: &Regex, invert_match: bool) -> MyResult<Vec<String>> {
    let mut results = Vec::new();
    for line in file.lines() {
        let line = line?;
        let reg = pattern.captures(&line);
        if reg.is_some() ^ invert_match {
            results.push(line);
        }
    }

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::{find_files, find_lines};
    use rand::{distributions::Alphanumeric, Rng};
    use regex::{Regex, RegexBuilder};
    use std::io::Cursor;

    #[test]
    fn test_find_files() {
        let files = find_files(&["./tests/inputs/fox.txt".to_string()], false);
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].as_ref().unwrap(), "./tests/inputs/fox.txt");

        let files = find_files(&["./tests/inputs".to_string()], false);
        assert_eq!(files.len(), 1);
        if let Err(e) = &files[0] {
            assert_eq!(e.to_string(), "./tests/inputs is a directory");
        }

        let res = find_files(&["./tests/inputs".to_string()], true);
        let mut files: Vec<String> = res
            .iter()
            .map(|r| r.as_ref().unwrap().replace("\\", "/"))
            .collect();
        files.sort();
        assert_eq!(files.len(), 4);
        assert_eq!(
            files,
            vec![
                "./tests/inputs/bustle.txt",
                "./tests/inputs/empty.txt",
                "./tests/inputs/fox.txt",
                "./tests/inputs/nobody.txt",
            ]
        );

        let bad: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(7)
            .map(char::from)
            .collect();
        let files = find_files(&[bad], false);
        assert_eq!(files.len(), 1);
        assert!(files[0].is_err());
    }

    #[test]
    fn test_find_lines() {
        let text = b"Lorem\nIpsum\r\nDOLOR";
        // The pattern _or_ should match the one line, "Lorem"
        let re1 = Regex::new("or").unwrap();
        let matches = find_lines(Cursor::new(&text), &re1, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
        // When inverted, the function should match the other two lines
        let matches = find_lines(Cursor::new(&text), &re1, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);
        // This regex will be case-insensitive
        let re2 = RegexBuilder::new("or")
            .case_insensitive(true)
            .build()
            .unwrap();
        // The two lines "Lorem" and "DOLOR" should match
        let matches = find_lines(Cursor::new(&text), &re2, false);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 2);
        // When inverted, the one remaining line should match
        let matches = find_lines(Cursor::new(&text), &re2, true);
        assert!(matches.is_ok());
        assert_eq!(matches.unwrap().len(), 1);
    }
}
