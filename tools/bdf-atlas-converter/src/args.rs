use std::ops::RangeInclusive;
use std::path::PathBuf;
use clap::Parser;

#[derive(Parser)]
#[command(about = "Convert BDF fonts using eg-font-converter")]
pub struct Args {
    /// Input BDF file
    #[arg(value_parser = validate_file_exists)]
    pub input: PathBuf,

    /// Output file
    #[arg(short, long)]
    pub output: Option<PathBuf>,

    /// Additional Unicode ranges in hex format (e.g., 0x20..0x7f)
    #[arg(short, long = "range", value_parser = parse_unicode_range)]
    pub ranges: Vec<RangeInclusive<char>>,
}

fn parse_unicode_range(s: &str) -> std::result::Result<RangeInclusive<char>, String> {
    if let Some((start_str, end_str)) = s.split_once("..") {
        let start_code = parse_hex(start_str.trim())
            .map_err(|e| format!("Invalid start value '{}': {}", start_str, e))?;
        let end_code = parse_hex(end_str.trim())
            .map_err(|e| format!("Invalid end value '{}': {}", end_str, e))?;

        let start_char = char::from_u32(start_code)
            .ok_or_else(|| format!("Invalid Unicode code point: 0x{:x}", start_code))?;
        let end_char = char::from_u32(end_code)
            .ok_or_else(|| format!("Invalid Unicode code point: 0x{:x}", end_code))?;

        if start_code > end_code {
            return Err(format!("Start value (0x{:x}) cannot be greater than end value (0x{:x})", start_code, end_code));
        }

        Ok(start_char..=end_char)
    } else {
        Err(format!("Invalid range format '{}'. Expected format: 0x20..0x7f", s))
    }
}

fn parse_hex(s: &str) -> std::result::Result<u32, String> {
    s.strip_prefix("0x")
        .ok_or_else(|| format!("Expected hexadecimal format (0x...), got: {}", s))
        .map(|hex_str| u32::from_str_radix(hex_str, 16))?
        .map_err(|_| format!("Invalid hexadecimal number: {}", s))
}

fn validate_file_exists(s: &str) -> std::result::Result<PathBuf, String> {
    let path = PathBuf::from(s);
    if !path.exists() {
        return Err(format!("Input file does not exist: {}", s));
    }
    if !path.is_file() {
        return Err(format!("Path is not a file: {}", s));
    }

    Ok(path)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_unicode_range_hex() {
        let range = parse_unicode_range("0x20..0x7f").unwrap();
        assert_eq!(*range.start(), '\u{20}');
        assert_eq!(*range.end(), '\u{7f}');
    }
}