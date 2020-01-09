use std::collections::HashSet;
use std::env::args;
use std::fs::{self, File};
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::path::{Path, PathBuf};
use std::vec::Vec;

#[macro_use]
extern crate lazy_static;
use clang::Clang;
use multimap::MultiMap;
use structopt::StructOpt;

mod analyze;
use analyze::Include;
mod compilations;
use compilations::CompilationsExt;
mod dependencies;
mod util;

#[derive(StructOpt)]
struct ToolArgs {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    #[structopt(short, long, default_value = "**/*")]
    filter: glob::Pattern,
    #[structopt(short, long = "compilations", parse(from_os_str))]
    comp: Option<PathBuf>,
    #[structopt(long, parse(from_os_str))]
    index: Option<PathBuf>,
}

fn main() {
    // Split command line args at '--'
    let (tool_args, ci_args) = {
        let mut args = args().collect::<Vec<_>>();
        if let Some(pos) = args.iter().position(|a| a == "--") {
            let ci_args = args.split_off(pos + 1);
            args.pop();
            (args, ci_args)
        } else {
            (args, vec![])
        }
    };

    let ToolArgs {
        file,
        filter,
        comp,
        index,
    } = ToolArgs::from_iter(tool_args.iter());

    let file = file.canonicalize().unwrap();

    let (include_paths, deps, ci_args) = if let Some(comp) = comp {
        let compilations =
            compilations::parse(comp, &filter).expect("Error parsing compilation database");

        let include_paths = compilations.collect_include_paths();
        println!("include paths {:?}", include_paths);

        let deps = if let Some(index) = index {
            serde_yaml::from_reader::<File, MultiMap<PathBuf, PathBuf>>(
                File::open(index).expect("Error opening include index"),
            )
            .expect("Error opening include index")
        } else {
            dependencies::index(&compilations.keys().collect::<Vec<_>>(), &include_paths)
        };

        let mut new_ci_args = compilations
            .get_related_args(&file, &deps)
            .expect("Missing compiler args in compilation database");
        // add custom args
        new_ci_args.extend(ci_args.into_iter());

        (include_paths, deps, new_ci_args)
    } else {
        let include_paths = util::include_paths(&ci_args.join(" "))
            .map(|e| PathBuf::from(e))
            .collect::<Vec<_>>();
        (include_paths, MultiMap::new(), ci_args)
    };

    let clang = Clang::new().expect("Could not load libclang");
    println!("libclang: {}", clang::get_version());

    remove_unused_includes(file, &ci_args, &include_paths, &deps, &clang);
}

fn remove_unused_includes<'a, P, S>(
    file: P,
    args: &[S],
    include_paths: &[PathBuf],
    deps: &MultiMap<PathBuf, PathBuf>,
    clang: &'a Clang,
) where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    if let Ok(includes) = analyze::unused_includes(clang, &file, args) {
        println!("Remove includes:");
        for include in &includes {
            println!(
                " - {}: {} -> {:?}",
                include.line, include.name, include.path
            );
        }
        if !includes.is_empty() {
            remove_includes(&file, &includes).expect("Could not remove includes");
        }

        if let Some(dependencies) = deps.get_vec(file.as_ref()) {
            for dependency in dependencies {
                println!("Check dependency {:?}", dependency);
                for include in &includes {
                    println!(" -- add {}", include.get_include(dependency, include_paths))
                }
                add_includes(dependency, &includes).expect("Could not propagate includes");

                remove_unused_includes(dependency, args, include_paths, deps, clang);
            }
        }
    }
}

fn add_includes<P: AsRef<Path>>(file: P, includes: &[Include]) -> io::Result<()> {
    // TODO: add includes
    Err(std::io::ErrorKind::Other.into())
}

fn remove_includes<P: AsRef<Path>>(file: P, includes: &[Include]) -> io::Result<()> {
    let temppath = file.as_ref().with_extension(".tmp");
    {
        let original = BufReader::new(File::open(&file)?);
        let mut tempfile = BufWriter::new(File::create(&temppath)?);

        // line numbers starting with 1
        let lines_to_remove = includes.iter().map(|i| i.line - 1).collect::<HashSet<_>>();

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
