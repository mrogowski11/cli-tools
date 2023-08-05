use clap::{App, Arg};
use once_cell::sync::OnceCell;
use regex::Regex;
use std::{
    error::Error,
    fs::File,
    io::{BufRead, BufReader, Read, Seek},
    ops::Mul,
};

static PLUS_ZERO_REG: OnceCell<Regex> = OnceCell::new();
static PLUS_NUM_REG: OnceCell<Regex> = OnceCell::new();

type MyResult<T> = Result<T, Box<dyn Error>>;

#[derive(Debug, PartialEq)]
enum TakeValue {
    PlusZero,
    TakeNum(i64),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    lines: TakeValue,
    bytes: Option<TakeValue>,
    quiet: bool,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("tail")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com")
        .about("Rust tail")
        .arg(
            Arg::with_name("files")
                .multiple(true)
                .value_name("FILE")
                .required(true)
                .help("Input file(s)"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("c")
                .long("bytes")
                .value_name("BYTES")
                .allow_hyphen_values(true)
                .help("Number of bytes"),
        )
        .arg(
            Arg::with_name("lines")
                .short("n")
                .long("lines")
                .value_name("LINES")
                .allow_hyphen_values(true)
                .conflicts_with("bytes")
                .help("Number of lines"),
        )
        .arg(
            Arg::with_name("quiet")
                .short("q")
                .long("quiet")
                .takes_value(false)
                .help("Suppress headers"),
        )
        .get_matches();

    let lines = if let Some(l) = matches.value_of("lines") {
        parse_count(l).map_err(|e| format!("illegal line count -- {}", e))?
    } else {
        TakeValue::TakeNum(-10)
    };
    let bytes: Option<TakeValue> = if let Some(b) = matches.value_of("bytes") {
        Some(parse_count(b).map_err(|e| format!("illegal byte count -- {}", e))?)
    } else {
        None
    };

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        lines,
        bytes,
        quiet: matches.is_present("quiet"),
    })
}

pub fn run(config: Config) -> MyResult<()> {
    let file_count = config.files.len();
    for (i, filename) in config.files.iter().enumerate() {
        match File::open(&filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => {
                if !config.quiet && file_count > 1 {
                    println!("{}==> {} <==", if i > 0 { "\n" } else { "" }, filename);
                }
                let (total_lines, total_bytes) = count_lines_bytes(&filename)?;
                if let Some(b) = &config.bytes {
                    print_bytes(&file, b, total_bytes)?;
                } else {
                    print_lines(BufReader::new(&file), &config.lines, total_lines)?;
                }
            }
        }
    }
    Ok(())
}

fn parse_count(s: &str) -> MyResult<TakeValue> {
    if PLUS_ZERO_REG
        .get_or_init(|| Regex::new(r"^\+0$").unwrap())
        .is_match(s)
    {
        Ok(TakeValue::PlusZero)
    } else if PLUS_NUM_REG
        .get_or_init(|| Regex::new(r"^(\+|\-)\d+$").unwrap())
        .is_match(s)
    {
        Ok(TakeValue::TakeNum(s.parse::<i64>().map_err(|_| s)?))
    } else {
        Ok(TakeValue::TakeNum(s.parse::<i64>().map_err(|_| s)?.mul(-1)))
    }
}

fn count_lines_bytes(filename: &str) -> MyResult<(i64, i64)> {
    let file = BufReader::new(File::open(filename)?);
    let byte_count = file.bytes().count() as i64;
    let file = BufReader::new(File::open(filename)?);
    let line_count = file.lines().count() as i64;

    Ok((line_count, byte_count))
}

fn print_lines(mut file: impl BufRead, num_lines: &TakeValue, total_lines: i64) -> MyResult<()> {
    let start = get_start_index(num_lines, total_lines);
    if let Some(s) = start {
        let mut buf = String::new();
        for _ in 0..s {
            file.read_line(&mut buf)?;
        }
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        print!("{}", String::from_utf8(buf)?);
    }

    Ok(())
}

fn print_bytes<T>(mut file: T, num_bytes: &TakeValue, total_bytes: i64) -> MyResult<()>
where
    T: Read + Seek,
{
    let start = get_start_index(num_bytes, total_bytes);
    if let Some(s) = start {
        file.seek(std::io::SeekFrom::Start(s))?;
        let mut buf = Vec::new();
        file.read_to_end(&mut buf)?;
        if !buf.is_empty() {
            print!("{}", String::from_utf8_lossy(&buf));
        }
    }
    Ok(())
}

fn get_start_index(take_val: &TakeValue, total: i64) -> Option<u64> {
    match (take_val, total) {
        (_, 0) => None,
        (TakeValue::PlusZero, _) => Some(0),
        (TakeValue::TakeNum(v), t) if v.is_negative() => match v.abs() {
            v if v < t => Some((t - (v.to_owned())) as u64),
            _ => Some(0),
        },
        (TakeValue::TakeNum(v), t) if v.is_positive() && v <= &t => Some((v - 1) as u64),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{count_lines_bytes, get_start_index, parse_count, TakeValue::*};
    #[test]
    fn test_parse_count() {
        // All integers should be interpreted as negative numbers
        let res = parse_count("3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));
        // A leading "+" should result in a positive number
        let res = parse_count("+3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(3));
        // An explicit "-" value should result in a negative number
        let res = parse_count("-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(-3));
        // Zero is Zero
        let res = parse_count("0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(0));
        // Plus zero is special
        let res = parse_count("+0");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), PlusZero);
        // Test boundaries
        let res = parse_count(&i64::MAX.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_count(&(i64::MIN + 1).to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN + 1));
        let res = parse_count(&format!("+{}", i64::MAX));
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MAX));
        let res = parse_count(&i64::MIN.to_string());
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), TakeNum(i64::MIN));
        // A floating-point value is invalid
        let res = parse_count("3.14");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "3.14");
        // Any noninteger string is invalid
        let res = parse_count("foo");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "foo");
    }
    #[test]
    fn test_count_lines_bytes() {
        let res = count_lines_bytes("tests/inputs/one.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (1, 24));

        let res = count_lines_bytes("tests/inputs/ten.txt");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), (10, 49));
    }
    #[test]
    fn test_get_start_index() {
        // +0 from an empty file (0 lines/bytes) returns None
        assert_eq!(get_start_index(&PlusZero, 0), None);
        // +0 from a nonempty file returns an index that
        // is one less than the number of lines/bytes
        assert_eq!(get_start_index(&PlusZero, 1), Some(0));
        // Taking 0 lines/bytes returns None
        assert_eq!(get_start_index(&TakeNum(0), 1), None);
        // Taking any lines/bytes from an empty file returns None
        assert_eq!(get_start_index(&TakeNum(1), 0), None);
        // Taking more lines/bytes than is available returns None
        assert_eq!(get_start_index(&TakeNum(2), 1), None);
        // When starting line/byte is less than total lines/bytes,
        // return one less than starting number
        assert_eq!(get_start_index(&TakeNum(1), 10), Some(0));
        assert_eq!(get_start_index(&TakeNum(2), 10), Some(1));
        assert_eq!(get_start_index(&TakeNum(3), 10), Some(2));
        // When starting line/byte is negative and less than total,
        // return total - start
        assert_eq!(get_start_index(&TakeNum(-1), 10), Some(9));
        assert_eq!(get_start_index(&TakeNum(-2), 10), Some(8));
        assert_eq!(get_start_index(&TakeNum(-3), 10), Some(7));
        // When starting line/byte is negative and more than total,
        // return 0 to print the whole file
        assert_eq!(get_start_index(&TakeNum(-20), 10), Some(0));
    }
}
