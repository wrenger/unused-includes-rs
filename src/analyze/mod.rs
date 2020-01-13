use std::path::{Path, PathBuf};
use std::vec::Vec;

use clang::source::File;
use clang::{Clang, Entity, EntityKind, EntityVisitResult, Index};

mod includes;
use includes::IncludeGraph;

trait EntityExt {
    fn get_sourcefile(&self) -> Option<File>;
}

impl<'tu> EntityExt for Entity<'tu> {
    fn get_sourcefile(&self) -> Option<File> {
        if let Some(location) = self.get_location() {
            let location = location.get_expansion_location();
            if let Some(file) = location.file {
                Some(file)
            } else {
                None
            }
        } else {
            None
        }
    }
}

fn find_includes(entity: Entity, includes: &mut IncludeGraph) -> EntityVisitResult {
    if entity.get_kind() == EntityKind::InclusionDirective {
        if let Some(from) = entity.get_sourcefile() {
            if let Some(to) = entity.get_file() {
                // Ignore corresponding headers in sourcefiles
                // TODO: allow filter pattern, ignore `// keep`
                if !entity.is_in_main_file()
                    || from.get_path().file_stem() != to.get_path().file_stem()
                {
                    includes.insert(from.get_id(), to.get_id());
                }
            }
        }
    }

    EntityVisitResult::Continue
}

fn mark_includes(entity: Entity, includes: &mut IncludeGraph) -> EntityVisitResult {
    if !entity.is_in_main_file() {
        return EntityVisitResult::Continue;
    }

    match entity.get_kind() {
        EntityKind::DeclRefExpr
        | EntityKind::TypeRef
        | EntityKind::TemplateRef
        | EntityKind::MacroExpansion => {
            if let Some(reference) = entity.get_reference() {
                if !reference.is_in_main_file() {
                    if let Some(to) = reference.get_sourcefile() {
                        includes.mark_used(&to.get_id());
                    }
                }
            }
        }
        _ => (),
    }

    EntityVisitResult::Recurse
}

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
    pub fn get_include<P: AsRef<Path>>(
        &self,
        file: P,
        include_paths: &[PathBuf],
    ) -> (bool, String) {
        // Prefer relative includes if possible
        // Allow relative includes from /src/ and /include/
        if let Some(filedir) = file.as_ref().parent() {
            for ancestor in filedir.ancestors() {
                if ancestor.ends_with("src")
                    || ancestor.ends_with("include")
                    || ancestor.ends_with("src/main")
                    || ancestor.ends_with("include/main")
                {
                    if let Some(file_relpath) = filedir.strip_prefix(ancestor).ok() {
                        for include_path in include_paths {
                            let path: PathBuf = [include_path, file_relpath].iter().collect();
                            if let Ok(relpath) = self.path.strip_prefix(path) {
                                return (true, relpath.to_string_lossy().into());
                            }
                        }
                    }
                }
            }
        }

        // Check for include paths
        for include_path in include_paths {
            if let Ok(relpath) = self.path.strip_prefix(include_path) {
                return (true, relpath.to_string_lossy().into());
            }
        }
        // Fallback for global includes
        (false, self.name.clone())
    }
}

pub fn unused_includes<'a, P, S>(clang: &'a Clang, file: P, args: &[S]) -> Result<Vec<Include>, ()>
where
    P: AsRef<Path>,
    S: AsRef<str>,
{
    let index = Index::new(clang, false, true);

    let result = match index
        .parser(file.as_ref())
        .arguments(args)
        .detailed_preprocessing_record(true)
        .parse()
    {
        Ok(tu) => {
            let mut includes = IncludeGraph::new();

            tu.get_entity()
                .visit_children(|entity, _| find_includes(entity, &mut includes));

            tu.get_entity()
                .visit_children(|entity, _| mark_includes(entity, &mut includes));

            if let Some(file) = tu.get_file(&file) {
                let unused = includes.unused(&file.get_id());
                let mut result = Vec::with_capacity(unused.len());

                file.visit_includes(|entity, source_range| {
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
    };
    result
}

#[cfg(test)]
mod test {
    use super::*;
    use std::env::current_dir;

    #[test]
    fn test_include_relpath() {
        let dir = current_dir().unwrap().join("tests");
        let include_paths = [dir.join("src"), dir.join("include")];

        let include = Include::new("Base.hpp".into(), dir.join("src/Base.hpp"), 0);
        assert_eq!(
            include.get_include(dir.join("src/Main.cpp"), &include_paths),
            (true, "Base.hpp".into())
        );

        let include = Include::new("Classes.hpp".into(), dir.join("src/refs/Classes.hpp"), 0);
        assert_eq!(
            include.get_include(dir.join("src/Main.cpp"), &include_paths),
            (true, "refs/Classes.hpp".into())
        );

        let include = Include::new(
            "ExternalRef.hpp".into(),
            dir.join("include/ref/ExternalRef.hpp"),
            0,
        );
        assert_eq!(
            include.get_include(dir.join("src/ref/Main.cpp"), &include_paths),
            (true, "ExternalRef.hpp".into())
        );

        let include = Include::new("vector".into(), dir.join("/usr/lib/include/vector"), 0);
        assert_eq!(
            include.get_include(dir.join("Main.cpp"), &include_paths),
            (false, "vector".into())
        );
    }
}
