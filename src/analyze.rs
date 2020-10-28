use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::vec::Vec;

use clang::source::{File, SourceRange};
use clang::{Entity, EntityKind, EntityVisitResult, Index};

mod includes;
use includes::{FileID, IncludeGraph};

// Connect to clang library
lazy_static::lazy_static! {
    static ref CLANG: clang::Clang = clang::Clang::new().expect("libclang loading failed");
}

trait EntityExt<'tu> {
    /// Returns the corresponding sourcefile
    fn get_sourcefile(&self) -> Option<File>;

    /// Returns the remaining line after this entity
    fn get_remaining_line(&self) -> Option<String>;
}

impl<'tu> EntityExt<'tu> for Entity<'tu> {
    fn get_sourcefile(&self) -> Option<File> {
        self.get_location()
            .and_then(|l| l.get_expansion_location().file)
    }

    fn get_remaining_line(&self) -> Option<String> {
        if let Some(range) = self.get_range() {
            let end = range.get_end().get_file_location();
            let offset = end.offset as usize;
            if let Some(file) = end.file {
                if let Some(content) = file.get_contents() {
                    if let Some(line_end) = content[offset as usize..].find('\n') {
                        return Some(content[offset as usize..offset + line_end].into());
                    } else {
                        return Some(content[offset as usize..].into());
                    }
                }
            }
        }
        None
    }
}

/// Checks if this include should be ignored
fn include_should_be_ignored(
    entity: &Entity,
    ignore_includes: &regex::Regex,
    from: &File,
    to: &File,
) -> bool {
    lazy_static::lazy_static! {
        static ref RE_KEEP: regex::Regex = regex::Regex::new("^[ \\t]*//[ \\t]*keep").unwrap();
    }

    if entity.is_in_main_file() {
        // Ignore explicitly marked includes `// keep`
        if let Some(line_end) = entity.get_remaining_line() {
            if RE_KEEP.is_match(&line_end)
                || ignore_includes.is_match(&to.get_path().to_string_lossy())
            {
                println!(
                    "{}: ignore {}",
                    from.get_path().to_string_lossy(),
                    entity.get_name().unwrap()
                );
                return true;
            }
        }
        // Ignore corresponding headers in sourcefiles
        from.get_path().file_stem() == to.get_path().file_stem()
    } else {
        false
    }
}

/// Create the include graph
fn find_includes(
    entity: Entity,
    ignore_includes: &regex::Regex,
    includes: &mut IncludeGraph,
) -> EntityVisitResult {
    if entity.get_kind() == EntityKind::InclusionDirective {
        if let Some(from) = entity.get_sourcefile() {
            if let Some(to) = entity.get_file() {
                if !include_should_be_ignored(&entity, ignore_includes, &from, &to) {
                    includes.insert(from.get_id(), to.get_id());
                }
            }
        }
    }

    EntityVisitResult::Continue
}

fn mark_includes_impl(entity: Entity, includes: &mut IncludeGraph) {
    match entity.get_kind() {
        EntityKind::DeclRefExpr
        | EntityKind::TypeRef
        | EntityKind::TemplateRef
        | EntityKind::MacroExpansion => {
            // Prefer definitions over declarations (if existing)
            if let Some(reference) = entity.get_definition().or_else(|| entity.get_reference()) {
                if reference == entity {
                    return;
                }
                if !reference.is_in_main_file() {
                    if let Some(to) = reference.get_sourcefile() {
                        includes.mark_used(&to.get_id());
                    }
                }
                mark_includes_impl(reference, includes)
            }
        }
        EntityKind::TypeAliasDecl | EntityKind::TypedefDecl => {
            if let Some(typeref) = entity.get_typedef_underlying_type() {
                if let Some(declaration) = typeref.get_declaration() {
                    if declaration == entity {
                        return;
                    }
                    if !declaration.is_in_main_file() {
                        if let Some(to) = declaration.get_sourcefile() {
                            includes.mark_used(&to.get_id());
                        }
                    }
                    mark_includes_impl(declaration, includes)
                }
            }
        }
        _ => {}
    }
}

/// Marks all necessary includes
fn mark_includes(entity: Entity, includes: &mut IncludeGraph) -> EntityVisitResult {
    if !entity.is_in_main_file() {
        return EntityVisitResult::Continue;
    }

    mark_includes_impl(entity, includes);

    EntityVisitResult::Recurse
}

#[derive(Debug)]
pub struct Include {
    pub name: String,
    pub path: PathBuf,
    pub line: usize,
}

impl Include {
    pub fn new(name: String, path: PathBuf, line: usize) -> Include {
        Include { name, path, line }
    }

    /// Returns the include path relative to the file if possible
    pub fn get_local<P: AsRef<Path>>(&self, file: P, include_paths: &[PathBuf]) -> Option<String> {
        // Prefer relative includes if possible
        // Allow relative includes from /src/ and /include/
        if let Some(filedir) = file.as_ref().parent() {
            for ancestor in filedir.ancestors() {
                if ancestor.ends_with("src")
                    || ancestor.ends_with("include")
                    || ancestor.ends_with("src/main")
                    || ancestor.ends_with("include/main")
                {
                    if let Ok(file_relpath) = filedir.strip_prefix(ancestor) {
                        for include_path in include_paths {
                            let path: PathBuf = [include_path, file_relpath].iter().collect();
                            if let Ok(relpath) = self.path.strip_prefix(path) {
                                return Some(relpath.to_string_lossy().into());
                            }
                        }
                    }
                }
            }
        }

        // Check for include paths
        for include_path in include_paths {
            if let Ok(relpath) = self.path.strip_prefix(include_path) {
                return Some(relpath.to_string_lossy().into());
            }
        }

        None
    }
}

/// Locate the unused includes in the sourcefile
fn collect_unused_includes(
    entity: Entity,
    source_range: SourceRange,
    unused: &HashSet<&FileID>,
    result: &mut Vec<Include>,
) {
    if let Some(file) = entity.get_file() {
        let path = file.get_path();
        if unused.contains(&&file.get_id()) {
            let start = source_range.get_start().get_file_location();
            result.push(Include::new(
                entity.get_name().unwrap(),
                path,
                start.line as usize,
            ));
        }
    }
}

/// Returns all includes that exist but are not referenced from the sourcefile
pub fn unused_includes<P, S>(
    filepath: P,
    args: &[S],
    ignore_includes: &regex::Regex,
) -> Result<Vec<Include>, ()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    match Index::new(&CLANG, false, true)
        .parser(filepath.as_ref())
        .arguments(args)
        .detailed_preprocessing_record(true)
        .parse()
    {
        Ok(tu) => {
            for diag in tu.get_diagnostics() {
                println!("clang: {}", diag);
            }

            let mut includes = IncludeGraph::new();

            tu.get_entity()
                .visit_children(|entity, _| find_includes(entity, ignore_includes, &mut includes));

            tu.get_entity()
                .visit_children(|entity, _| mark_includes(entity, &mut includes));

            if let Some(file) = tu.get_file(&filepath) {
                let unused = includes.unused(&file.get_id());
                let mut result = Vec::with_capacity(unused.len());

                file.visit_includes(|entity, source_range| {
                    collect_unused_includes(entity, source_range, &unused, &mut result);
                    true
                });

                Ok(result)
            } else {
                Err(())
            }
        }
        Err(err) => {
            eprintln!("Parsing error: {}", err);
            Err(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env::current_dir;
    use std::fs;

    #[test]
    fn test_include_relpath() {
        let dir = current_dir().unwrap().join("tests");
        let include_paths = [dir.join("src"), dir.join("include")];

        let include = Include::new("Base.hpp".into(), dir.join("src/Base.hpp"), 0);
        assert_eq!(
            include.get_local(dir.join("src/Main.cpp"), &include_paths),
            Some("Base.hpp".into())
        );

        let include = Include::new("Classes.hpp".into(), dir.join("src/refs/Classes.hpp"), 0);
        assert_eq!(
            include.get_local(dir.join("src/Main.cpp"), &include_paths),
            Some("refs/Classes.hpp".into())
        );

        let include = Include::new(
            "ExternalRef.hpp".into(),
            dir.join("include/ref/ExternalRef.hpp"),
            0,
        );
        assert_eq!(
            include.get_local(dir.join("src/ref/Main.cpp"), &include_paths),
            Some("ExternalRef.hpp".into())
        );

        let include = Include::new("vector".into(), dir.join("/usr/lib/include/vector"), 0);
        assert_eq!(
            include.get_local(dir.join("Main.cpp"), &include_paths),
            None
        );
    }

    #[test]
    fn test_unused_includes() {
        let dir = current_dir().unwrap().join("tests/src/refs");
        let ignore_includes = regex::Regex::new("(/private/|[_/]impl[_\\./])").unwrap();
        let args: [&str; 0] = [];

        for file in fs::read_dir(dir).unwrap() {
            let file = file.unwrap();
            if let Some(ext) = file.path().extension() {
                if ext == "cpp" {
                    let unused =
                        unused_includes(file.path(), &args, &ignore_includes).expect("Include Err");
                    assert!(unused.is_empty(), "{:?}", &unused);
                }
            }
        }
    }

    #[test]
    fn test_unused_includes_single() {
        let file = current_dir().unwrap().join("tests/src/refs/UsingT.cpp");
        let ignore_includes = regex::Regex::new("(/private/|[_/]impl[_\\./])").unwrap();
        let args: [&str; 0] = [];
        let unused = unused_includes(&file, &args, &ignore_includes).expect("Include Err");
        assert!(unused.is_empty(), "{:?}", &unused);
    }
}
