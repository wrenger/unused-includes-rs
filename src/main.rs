use std::env::args;
use std::fs::File;
use std::path::{Path, PathBuf};
use std::vec::Vec;

use clang::Clang;
use multimap::MultiMap;
use structopt::StructOpt;

mod analyze;
mod compilations;
use compilations::Compilations;
mod clangfmt;
mod dependencies;
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
            let file = File::open(index).expect("Error opening include index");
            serde_yaml::from_reader(file).expect("Error opening include index")
        } else {
            println!("Creating dependency tree...");
            let index = dependencies::index(&compilations.sources(), &include_paths);
            let file = File::create("dependencies.json").expect("Could not backup index");
            serde_yaml::to_writer(file, &index).expect("Could not backup index");
            index
        };

        println!("Analyzing sources:");
        dependencies::print_dependency_tree(&file, &index, 0);

        let mut new_ci_args = compilations
            .get_related_args(&file, &index)
            .expect("Missing compiler args in compilation database");
        // add custom args
        new_ci_args.extend(ci_args);

        (include_paths, index, new_ci_args)
    } else {
        println!("No compilation database provided. Analyzing only the given source.");
        let include_paths = util::include_paths(&ci_args.join(" "))
            .map(|e| PathBuf::from(e))
            .collect::<Vec<_>>();
        (include_paths, MultiMap::new(), ci_args)
    };

    let clang = Clang::new().expect("Could not load libclang");
    println!("libclang: {}", clang::get_version());

    remove_unused_includes(
        &clang,
        file,
        &ci_args,
        &ignore_includes,
        &include_paths,
        &index,
    );
}

fn remove_unused_includes<'a, P, S>(
    clang: &'a Clang,
    file: P,
    args: &[S],
    ignore_includes: &regex::Regex,
    include_paths: &[PathBuf],
    index: &MultiMap<PathBuf, PathBuf>,
) where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    if let Ok(includes) = analyze::unused_includes(clang, &file, args, ignore_includes) {
        if !includes.is_empty() {
            for include in &includes {
                println!(
                    "{}: remove {}",
                    file.as_ref().to_string_lossy(),
                    include.name
                );
            }

            let lines = includes.iter().map(|i| i.line).collect::<Vec<_>>();
            fileio::remove_includes(&file, &lines).expect("Could not remove includes");
            // Sort includes
            clangfmt::includes(&file).expect("Clang-format failed");
        }

        if let Some(dependencies) = index.get_vec(file.as_ref()) {
            for dependency in dependencies {
                println!("Check dependency {:?}", dependency);
                // Add removed includes
                if !includes.is_empty() {
                    fileio::add_includes(
                        dependency,
                        includes
                            .iter()
                            .map(|i| i.get_include(&dependency, include_paths)),
                    )
                    .expect("Could not propagate includes");
                }

                remove_unused_includes(
                    clang,
                    dependency,
                    args,
                    ignore_includes,
                    include_paths,
                    index,
                );
            }
        }
    }
}
