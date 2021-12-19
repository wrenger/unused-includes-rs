use std::collections::HashSet;
use std::fmt;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::usize;

use regex::Regex;

use super::util;

// Regexes for several preprocessor directives
lazy_static::lazy_static! {
    static ref RE_INCLUDE: Regex =
        Regex::new("^[ \\t]*#[ \\t]*include[ \\t]*[<\"]([\\./\\w-]+)[>\"]").unwrap();
    static ref RE_LOCAL_INCLUDE: Regex =
        Regex::new("^[ \\t]*#[ \\t]*include[ \\t]*\"([\\./\\w-]+)\"").unwrap();
    static ref RE_IF: Regex = Regex::new("^[ \\t]*#[ \\t]*if").unwrap();
    static ref RE_ENDIF: Regex = Regex::new("^[ \\t]*#[ \\t]*endif").unwrap();
    static ref RE_PRAGMA_ONCE: Regex = Regex::new("^[ \\t]*#[ \\t]*pragma[ \\t]+once").unwrap();
}

/// Collect includes ignoring those defined in #if..#endif blocks.
pub fn parse_includes(path: &Path) -> HashSet<String> {
    let is_header = util::is_header_file(path);
    if let Ok(mut file) = File::open(path) {
        let (_, incudes) =
            parse_includes_file(&mut file, &RE_LOCAL_INCLUDE, is_header).unwrap_or_default();
        incudes
    } else {
        HashSet::new()
    }
}

/// Collect includes ignoring those defined in #if..#endif blocks.
///
/// Also return the offset to the first (or second if its a sourcefile) include.
fn parse_includes_file(
    file: &mut File,
    include_re: &regex::Regex,
    is_header: bool,
) -> io::Result<(usize, HashSet<String>)> {
    let (mut depth, mut skip_first) = if is_header {
        (-1, false) // header guards
    } else {
        (0, true)
    };

    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;

    let mut offset = 0;
    let mut found = false;
    let mut includes = HashSet::new();

    for line in buffer.split('\n') {
        if RE_PRAGMA_ONCE.is_match(line) || RE_IF.is_match(line) {
            depth += 1;
        } else if RE_ENDIF.is_match(line) {
            depth -= 1;
        } else if depth == 0 && include_re.is_match(line) {
            if skip_first {
                skip_first = false;
            } else {
                found = true;
            }
            let include = &include_re.captures(line).unwrap()[1];
            includes.insert(String::from(include));
        }
        if !found {
            offset += line.len() + 1;
        }
    }
    if !found {
        offset = 0;
    }

    Ok((offset, includes))
}

pub enum IncludeStatement {
    Local(String),
    Global(String),
}

impl IncludeStatement {
    fn path(&self) -> &str {
        match self {
            IncludeStatement::Local(path) => path,
            IncludeStatement::Global(path) => path,
        }
    }
}

impl fmt::Display for IncludeStatement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            IncludeStatement::Local(path) => write!(f, "#include \"{}\"", path),
            IncludeStatement::Global(path) => write!(f, "#include <{}>", path),
        }
    }
}

/// Adds the given `includes` in front of the old includes of the given file
pub fn add_includes<I>(filepath: &Path, includes: I) -> io::Result<()>
where
    I: Iterator<Item = IncludeStatement>,
{
    let mut file = OpenOptions::new().read(true).write(true).open(&filepath)?;

    let (offset, old_includes) =
        parse_includes_file(&mut file, &RE_INCLUDE, util::is_header_file(filepath))?;

    file.seek(SeekFrom::Start(offset as u64))?;
    let mut buffer = Vec::new();
    file.read_to_end(&mut buffer)?;
    file.seek(SeekFrom::Start(offset as u64))?;

    for include in includes {
        if !old_includes.contains(include.path()) {
            writeln!(file, "{}", include)?;
        }
    }
    file.write_all(&buffer)?;

    Ok(())
}

/// Removes the `includes` at the given lines from the `file`.
pub fn remove_includes(file: &Path, includes: &[usize]) -> io::Result<()> {
    let temppath = file.with_extension(".tmp");
    {
        let original = BufReader::new(File::open(file)?);
        let mut tempfile = BufWriter::new(File::create(&temppath)?);

        // line numbers starting with 1
        let lines_to_remove = includes.iter().map(|i| i - 1).collect::<HashSet<_>>();

        for (i, line) in original.split(b'\n').enumerate() {
            let line = line?;
            if !lines_to_remove.contains(&i) {
                tempfile.write_all(&line)?;
                tempfile.write_all(b"\n")?;
            }
        }
    };

    fs::remove_file(file)?;
    fs::rename(&temppath, file)?;

    Ok(())
}
