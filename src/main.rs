use std::path::{Path, PathBuf};

use clang::Clang;
use multimap::MultiMap;
use structopt::StructOpt;

mod analyze;
mod dependencies;
mod util;

#[derive(StructOpt)]
struct Args {
    #[structopt(parse(from_os_str))]
    file: PathBuf,
    #[structopt(default_value = "")]
    args: String,
    #[structopt(long, short, default_value = "**/*")]
    filter: String,
}

fn main() {
    let Args { file, args, filter } = Args::from_args();

    let deps = dependencies::index(&filter);
    println!("deps: {:?}", deps);

    let clang = Clang::new().expect("Could not load libclang");
    println!("libclang: {}", clang::get_version());

    let args = shlex::split(&args).expect("Malformed compiler args");
    let filter = glob::Pattern::new(&filter).expect("Malformed filter pattern");
    let include_paths = util::include_paths(&args, &filter);

    remove_unused_includes(file, &args, &include_paths, &deps, &clang);
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

fn remove_includes<P: AsRef<Path>>(file: P, includes: &[(usize, String)]) {


}
