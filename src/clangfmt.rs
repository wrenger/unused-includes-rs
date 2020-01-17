use std::cell::RefCell;
use std::ffi::OsStr;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;

pub const EXEC: RefCell<String> = RefCell::new(String::new());

pub fn includes<P: AsRef<Path>>(file: P) -> io::Result<()> {
    let fmt_ranges = include_ranges(&file)?;

    let output = {
        Command::new(EXEC.borrow().as_ref() as &OsStr)
            .arg(file.as_ref())
            .arg("-i")
            .arg("-sort-includes")
            .args(
                fmt_ranges
                    .iter()
                    .map(|(s, e)| format!("-lines={}:{}", s + 1, e + 1)),
            )
            .output()?
    };

    if !output.status.success() {
        println!(
            "Clang format failed with {}",
            output.status.code().unwrap_or_default()
        );
        io::stdout().write_all(&output.stdout)?;
        io::stderr().write_all(&output.stderr)?;
    } else {
        println!("Includes formatted");
    }

    Ok(())
}

fn include_ranges<P: AsRef<Path>>(file: P) -> io::Result<Vec<(usize, usize)>> {
    lazy_static::lazy_static! {
        static ref RE_PREPROCESSOR: regex::Regex = regex::Regex::new("^[ \\t]*([#/]|$)").unwrap();
    }

    let file = File::open(file)?;
    let reader = BufReader::new(file);

    let mut fmt_ranges = Vec::new();
    let mut start = 0;
    let mut has_preprocessor_stmt = false;

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        if RE_PREPROCESSOR.is_match(&line) {
            if &RE_PREPROCESSOR.captures(&line).unwrap()[1] == "#" {
                has_preprocessor_stmt = true;
            }
        } else {
            if has_preprocessor_stmt && start < i - 1 {
                fmt_ranges.push((start, i - 1));
            }
            start = i + 1;
            has_preprocessor_stmt = false;
        }
    }
    Ok(fmt_ranges)
}
