use clap::{App, Arg};
use std::io::{self, BufRead, BufReader};
use std::{error::Error, fs::File, ops::Range};

type MyResult<T> = Result<T, Box<dyn Error>>;
type PositionList = Vec<Range<usize>>;

#[derive(Debug)]
pub enum Extract {
    Fields(PositionList),
    Bytes(PositionList),
    Chars(PositionList),
}

#[derive(Debug)]
pub struct Config {
    files: Vec<String>,
    delimiter: u8,
    extract: Extract,
}

pub fn get_args() -> MyResult<Config> {
    let matches = App::new("cut")
        .version("0.1.0")
        .author("Marcin Rogowski <rogowskimarcin11@gmail.com")
        .about("Rust cut")
        .arg(
            Arg::with_name("files")
                .multiple(true)
                .value_name("FILE")
                .default_value("-")
                .help("Prints help information"),
        )
        .arg(
            Arg::with_name("delimiter")
                .short("d")
                .long("delim")
                .value_name("DELIMITER")
                .default_value("\t")
                .help("Field delimiter"),
        )
        .arg(
            Arg::with_name("bytes")
                .short("b")
                .long("bytes")
                .value_name("BYTES")
                .conflicts_with("chars")
                .conflicts_with("fields")
                .help("Selected bytes"),
        )
        .arg(
            Arg::with_name("chars")
                .short("c")
                .long("chars")
                .value_name("CHARS")
                .conflicts_with("bytes")
                .conflicts_with("fields")
                .help("Selected characters"),
        )
        .arg(
            Arg::with_name("fields")
                .short("f")
                .long("fields")
                .value_name("FIELDS")
                .conflicts_with("bytes")
                .conflicts_with("chars")
                .help("Selected fields"),
        )
        .get_matches();

    let extract = vec![
        matches.value_of_lossy("bytes"),
        matches.value_of_lossy("chars"),
        matches.value_of_lossy("fields"),
    ]
    .into_iter()
    .find_map(|t| t)
    .ok_or("Must have --fields, --bytes, or --chars")?;

    let pos_vec = parse_pos(&extract)?;
    let extract = if let true = matches.is_present("bytes") {
        Extract::Bytes(pos_vec)
    } else if let true = matches.is_present("chars") {
        Extract::Chars(pos_vec)
    } else {
        Extract::Fields(pos_vec)
    };

    let delimiter: MyResult<u8> = match matches.value_of_lossy("delimiter").unwrap().as_bytes() {
        b if b.len() == 1 => Ok(b[0]),
        b => Err(From::from(format!(
            "--delim \"{}\" must be a single byte",
            std::str::from_utf8(b)?
        ))),
    };

    Ok(Config {
        files: matches.values_of_lossy("files").unwrap(),
        delimiter: delimiter?,
        extract,
    })
}

pub fn run(config: Config) -> MyResult<()> {
    for filename in &config.files {
        match open(filename) {
            Err(err) => eprintln!("{}: {}", filename, err),
            Ok(file) => {
                let buf_reader = BufReader::new(file);
                match &config.extract {
                    Extract::Bytes(pos) => {
                        for line in buf_reader.lines() {
                            let extracted = extract_bytes(&(line?), pos);
                            println!("{}", extracted);
                        }
                    }
                    Extract::Chars(pos) => {
                        for line in buf_reader.lines() {
                            let extracted = extract_chars(&(line?), pos);
                            println!("{}", extracted);
                        }
                    }
                    Extract::Fields(pos) => {
                        let mut reader = csv::ReaderBuilder::new()
                            .delimiter(config.delimiter)
                            .has_headers(false)
                            .from_reader(buf_reader);
                        let mut writer = csv::WriterBuilder::new()
                            .delimiter(config.delimiter)
                            .from_writer(io::stdout());
                        for record in reader.records() {
                            let extracted_fields = extract_fields(&record?, pos);
                            writer.write_record(extracted_fields)?;
                        }
                        writer.flush()?;
                    }
                }
            }
        }
    }
    Ok(())
}

fn parse_pos(ranges: &str) -> MyResult<PositionList> {
    ranges
        .split(',')
        .map(|range| range.split('-').collect())
        .map(|e: Vec<&str>| match e.len() {
            n if n == 2 => match (parse_positive_int(e[0]), parse_positive_int(e[1])) {
                (Ok(start), Ok(end)) if end > start => Ok(Range {
                    start: start - 1,
                    end,
                }),
                (Ok(start), Ok(end)) if end <= start => Err(From::from(format!(
                    "First number in range ({}) must be lower than second number ({})",
                    e[0], e[1]
                ))),
                _ => Err(From::from(format!(
                    "illegal list value: \"{}-{}\"",
                    e[0], e[1]
                ))),
            },
            n if n == 1 => match parse_positive_int(e[0]) {
                Ok(start) => Ok(Range {
                    start: start - 1,
                    end: start,
                }),
                _ => Err(From::from(format!("illegal list value: \"{}\"", e[0]))),
            },
            _ => Err(From::from(format!("illegal list value: \"{:#?}\"", e))),
        })
        .collect::<Result<Vec<_>, _>>()
}

fn parse_positive_int(val: &str) -> MyResult<usize> {
    if !val.chars().all(char::is_numeric) {
        return Err(From::from(val));
    }
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

fn extract_chars(line: &str, char_pos: &[Range<usize>]) -> String {
    let chars: Vec<_> = line.chars().collect();
    char_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| chars.get(i)))
        .collect()
}

fn extract_bytes(line: &str, byte_pos: &[Range<usize>]) -> String {
    let bytes = line.as_bytes();
    let extracted: Vec<_> = byte_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| bytes.get(i).copied()))
        .collect();
    String::from_utf8_lossy(&extracted).into_owned()
}

fn extract_fields(record: &csv::StringRecord, field_pos: &[Range<usize>]) -> Vec<String> {
    field_pos
        .iter()
        .cloned()
        .flat_map(|range| range.filter_map(|i| record.get(i)))
        .map(|field| field.to_owned())
        .collect()
}

#[cfg(test)]
mod unit_tests {
    use super::extract_bytes;
    use super::extract_chars;
    use super::extract_fields;
    use super::parse_pos;

    #[test]
    fn test_parse_pos() {
        assert!(parse_pos("").is_err());

        let res = parse_pos("0");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0\"",);

        let res = parse_pos("0-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"0-1\"",);

        let res = parse_pos("+1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1\"",);

        let res = parse_pos("+1-2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"+1-2\"",);

        let res = parse_pos("1-+2");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-+2\"",);

        let res = parse_pos("a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1,a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a\"",);

        let res = parse_pos("1-a");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"1-a\"",);

        let res = parse_pos("a-1");
        assert!(res.is_err());
        assert_eq!(res.unwrap_err().to_string(), "illegal list value: \"a-1\"",);

        let res = parse_pos("-");
        assert!(res.is_err());

        let res = parse_pos(",");
        assert!(res.is_err());

        let res = parse_pos("1,");
        assert!(res.is_err());

        let res = parse_pos("1-");
        assert!(res.is_err());

        let res = parse_pos("1-1-1");
        assert!(res.is_err());

        let res = parse_pos("1-1-a");
        assert!(res.is_err());

        let res = parse_pos("1-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (1) must be lower than second number (1)"
        );

        let res = parse_pos("2-1");
        assert!(res.is_err());
        assert_eq!(
            res.unwrap_err().to_string(),
            "First number in range (2) must be lower than second number (1)"
        );

        let res = parse_pos("1");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("01");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1]);

        let res = parse_pos("1,3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("001,0003");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 2..3]);

        let res = parse_pos("1-3");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("0001-03");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..3]);

        let res = parse_pos("1,7,3-5");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![0..1, 6..7, 2..5]);

        let res = parse_pos("15,19-20");
        assert!(res.is_ok());
        assert_eq!(res.unwrap(), vec![14..15, 18..20]);
    }
    #[test]
    fn test_extract_chars() {
        assert_eq!(extract_chars("", &[0..1]), "".to_string());
        assert_eq!(extract_chars("ábc", &[0..1]), "á".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 2..3]), "ác".to_string());
        assert_eq!(extract_chars("ábc", &[0..3]), "ábc".to_string());
        assert_eq!(extract_chars("ábc", &[2..3, 1..2]), "cb".to_string());
        assert_eq!(extract_chars("ábc", &[0..1, 1..2, 4..5]), "áb".to_string());
    }

    #[test]
    fn test_extract_bytes() {
        assert_eq!(extract_bytes("ábc", &[0..1]), "�".to_string());
        assert_eq!(extract_bytes("ábc", &[0..2]), "á".to_string());
        assert_eq!(extract_bytes("ábc", &[0..3]), "áb".to_string());
        assert_eq!(extract_bytes("ábc", &[0..4]), "ábc".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
        assert_eq!(extract_bytes("ábc", &[3..4, 2..3]), "cb".to_string());
    }

    #[test]
    fn test_extract_fields() {
        let rec = csv::StringRecord::from(vec!["Captain", "Sham", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2]), &["Sham"]);
        assert_eq!(extract_fields(&rec, &[0..1, 2..3]), &["Captain", "12345"]);
        assert_eq!(extract_fields(&rec, &[0..1, 3..4]), &["Captain"]);
        assert_eq!(extract_fields(&rec, &[1..2, 0..1]), &["Sham", "Captain"]);
    }
}
