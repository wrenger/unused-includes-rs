use std::collections::HashSet;
use std::fs::{self, File, OpenOptions};
use std::io::{self, BufRead, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use std::usize;

use super::util;
use super::{RE_ENDIF, RE_IF, RE_INCLUDE, RE_LOCAL_INCLUDE, RE_PRAGMA_ONCE};

/// Collect includes ignoring those defined in #if..#endif blocks.
pub fn parse_includes<P: AsRef<Path>>(path: P) -> HashSet<String> {
    let is_header = util::is_header_file(&path);
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
        if RE_PRAGMA_ONCE.is_match(line) {
            depth += 1;
        } else if RE_IF.is_match(line) {
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

/// Adds the given `includes` in front of the old includes of the given file
pub fn add_includes<P, I>(filepath: P, includes: I) -> io::Result<()>
where
    P: AsRef<Path>,
    I: IntoIterator<Item = (bool, String)>
{
    let mut file = OpenOptions::new().read(true).write(true).open(&filepath)?;

    let (offset, old_includes) =
        parse_includes_file(&mut file, &RE_INCLUDE, util::is_header_file(&filepath))?;

    file.seek(SeekFrom::Start(offset as u64))?;
    let mut buffer = String::new();
    file.read_to_string(&mut buffer)?;
    file.seek(SeekFrom::Start(offset as u64))?;

    for (local, path) in includes
        .into_iter()
        .filter(|(_, i)| !old_includes.contains(i))
    {
        if local {
            writeln!(file, "#include \"{}\"", path)?;
        } else {
            writeln!(file, "#include <{}>", path)?;
        }
    }
    write!(file, "{}", &buffer)?;

    Ok(())
}

/// Removes the `includes` at the given lines from the `file`.
pub fn remove_includes<P>(file: P, includes: &[usize]) -> io::Result<()>
where
    P: AsRef<Path>,
{
    let temppath = file.as_ref().with_extension(".tmp");
    {
        let original = BufReader::new(File::open(&file)?);
        let mut tempfile = BufWriter::new(File::create(&temppath)?);

        // line numbers starting with 1
        let lines_to_remove = includes.iter().map(|i| i - 1).collect::<HashSet<_>>();

        for (i, line) in original.split(b'\n').enumerate() {
            let line = line?;
            if !lines_to_remove.contains(&i) {
                tempfile.write(&line)?;
                tempfile.write(b"\n")?;
            }
        }
    };

    fs::remove_file(&file)?;
    fs::rename(&temppath, &file)?;

    Ok(())
}
