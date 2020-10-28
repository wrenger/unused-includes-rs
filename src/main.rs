use std::collections::HashSet;
use std::env::args;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::vec::Vec;

use structopt::StructOpt;

mod analyze;
mod compilations;
use compilations::Compilations;
mod clangfmt;
mod dependencies;
use dependencies::Dependencies;
mod fileio;
mod util;

#[derive(StructOpt)]
struct ToolArgs {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    #[structopt(short, long, default_value = ".")]
    filter: regex::Regex,
    #[structopt(short, long = "compilations", parse(from_os_str))]
    comp: Option<PathBuf>,
    #[structopt(long, parse(from_os_str))]
    index: Option<PathBuf>,
    #[structopt(long, default_value = "clang-format")]
    clang_format: String,
    #[structopt(long, default_value = "(/private/|[_/]impl[_\\./])")]
    ignore_includes: regex::Regex,
}

fn main() {
    println!("libclang: {}", clang::get_version());

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
        clang_format,
        ignore_includes,
    } = ToolArgs::from_iter(tool_args.iter());

    if let Ok(mut val) = clangfmt::EXEC.write() {
        *val = clang_format;
    }

    let file = file.canonicalize().unwrap();

    let (include_paths, index, ci_args) = if let Some(comp) = comp {
        println!("Parsing compilaton database...");
        let compilations =
            Compilations::parse(comp, &filter).expect("Error parsing compilation database");

        let include_paths = compilations.collect_include_paths();
        println!("Include paths: {:?}", include_paths);

        let index = if let Some(index) = index {
            println!("Loading dependency tree...");
            let file = File::open(index).expect("Error opening include index");
            serde_json::from_reader(file).expect("Error parsing include index")
        } else {
            println!("Creating dependency tree...");
            let index = Dependencies::create(&compilations.sources(), &include_paths, &filter);
            let file = File::create("dependencies.json").expect("Could not backup index");
            serde_json::to_writer(file, &index).expect("Could not backup index");
            index
        };

        index.print(&file);

        let mut new_ci_args = compilations
            .get_related_args(&file, &index)
            .expect("Missing compiler args in compilation database");
        // add custom args
        new_ci_args.extend(ci_args);

        (include_paths, index, new_ci_args)
    } else {
        println!("No compilation database provided. Analyzing only the given source.");
        let include_paths = util::include_paths(&ci_args.join(" "))
            .map(PathBuf::from)
            .collect::<Vec<_>>();
        (include_paths, Dependencies::new(), ci_args)
    };

    println!("Analyzing {}", file.to_string_lossy());
    let mut visited = HashSet::new();

    remove_unused_includes(
        file,
        &ci_args,
        &ignore_includes,
        &include_paths,
        &index,
        &mut visited,
    );
}

fn remove_unused_includes<P, S>(
    file: P,
    args: &[S],
    ignore_includes: &regex::Regex,
    include_paths: &[PathBuf],
    index: &Dependencies,
    visited: &mut HashSet<PathBuf>,
) where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    if !visited.insert(PathBuf::from(file.as_ref())) {
        println!(" -> Circular includes: {}", file.as_ref().to_string_lossy());
    } else if let Ok(includes) = analyze::unused_includes(&file, args, ignore_includes) {
        println!(" -> Remove {:?}", includes);

        if !includes.is_empty() {
            let lines = includes.iter().map(|i| i.line).collect::<Vec<_>>();
            fileio::remove_includes(&file, &lines).expect("Could not remove includes");
            // Sort includes
            clangfmt::includes(&file).expect("Clang-format failed");
        }

        for dependency in index.get(file.as_ref()) {
            println!("Analyzing {}", dependency.to_string_lossy());
            // Add removed includes
            if !includes.is_empty() {
                let includes = includes.iter().map(|i| {
                    i.get_local(&dependency, include_paths).map_or_else(
                        || fileio::IncludeStatement::Global(i.name.clone()),
                        fileio::IncludeStatement::Local,
                    )
                });
                fileio::add_includes(dependency, includes).expect("Could not propagate includes");
            }

            remove_unused_includes(
                dependency,
                args,
                ignore_includes,
                include_paths,
                index,
                visited,
            );
        }
    }
}
