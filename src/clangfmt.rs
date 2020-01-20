use std::fs::File;
use std::io::{self, BufRead, BufReader, Write};
use std::path::Path;
use std::process::Command;
use std::sync::{Arc, RwLock};

lazy_static::lazy_static! {
    pub static ref EXEC: Arc<RwLock<String>> = Arc::new(RwLock::new(String::new()));
}

pub fn includes<P: AsRef<Path>>(file: P) -> io::Result<()> {
    let fmt_ranges = include_ranges(&file)?;

    match Command::new(&*EXEC.read().unwrap())
        .arg(file.as_ref())
        .arg("-i")
        .arg("-sort-includes")
        .args(
            fmt_ranges
                .iter()
                .map(|(s, e)| format!("-lines={}:{}", s + 1, e + 1)),
        )
        .output()
    {
        Ok(output) => {
            if !output.status.success() {
                eprintln!(
                    "{} failed with {}",
                    *EXEC.read().unwrap(),
                    output.status.code().unwrap_or_default(),
                );
                io::stdout().write_all(&output.stdout)?;
                io::stderr().write_all(&output.stderr)?;
            }
        }
        Err(err) => {
            eprintln!("{} failed: {}", *EXEC.read().unwrap(), err);
        }
    }

    Ok(())
}

fn include_ranges<P: AsRef<Path>>(file: P) -> io::Result<Vec<(usize, usize)>> {
    let file = File::open(file)?;
    let reader = BufReader::new(file);

    let mut fmt_ranges = Vec::new();
    let mut start = 0;
    let mut has_preprocessor_stmt = false;

    for (i, line) in reader.lines().enumerate() {
        let line = line?;
        let trimmed_line = line.trim_start();
        if trimmed_line.starts_with('#') {
            has_preprocessor_stmt = true;
        } else if !trimmed_line.is_empty() {
            if has_preprocessor_stmt && start < i - 1 {
                fmt_ranges.push((start, i - 1));
            }
            start = i + 1;
            has_preprocessor_stmt = false;
        }
    }
    Ok(fmt_ranges)
}
