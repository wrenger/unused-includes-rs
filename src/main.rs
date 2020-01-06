use std::env::args;
use std::path::{Path, PathBuf};
use std::vec::Vec;

#[macro_use]
extern crate lazy_static;
use clang::Clang;
use multimap::MultiMap;
use structopt::StructOpt;

mod analyze;
mod dependencies;
mod util;

#[derive(StructOpt)]
struct ToolArgs {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    #[structopt(long, short, default_value = "**/*")]
    filter: String,
    #[structopt(long, short, default_value = "compile_commands.json")]
    compilations: String,
}

fn main() {
    // Split command line args at '--'
    let (tool_args, mut ci_args) = {
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
        compilations,
    } = ToolArgs::from_iter(tool_args.iter());

    let deps = MultiMap::new();
    // let deps = dependencies::index(&filter);
    // println!("deps: {:?}", deps);

    let clang = Clang::new().expect("Could not load libclang");
    println!("libclang: {}", clang::get_version());

    println!("ci args {:?}", ci_args);
    ci_args.pop();
    let filter = glob::Pattern::new(&filter).expect("Malformed filter pattern");
    let include_paths = util::include_paths(&ci_args, &filter);

    remove_unused_includes(file, &ci_args, &include_paths, &deps, &clang);
}

fn remove_unused_includes<'a, P, S>(
    file: P,
    args: &[S],
    include_paths: &[PathBuf],
    deps: &MultiMap<String, PathBuf>,
    clang: &'a Clang,
) where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    if let Ok(includes) = analyze::unused_includes(clang, &file, args) {
        remove_includes(&file, &includes);

        // TODO: Find deps and propergate
    }
}

fn remove_includes<P: AsRef<Path>>(file: P, includes: &[(usize, PathBuf)]) {
    println!("Remove includes:");
    for (line, file) in includes {
        println!(" - {}: {:?}", line, file);
    }
}
