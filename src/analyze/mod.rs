use std::path::{Path, PathBuf};
use std::vec::Vec;

use clang::{Clang, Entity, EntityKind, EntityVisitResult, Index};

mod includes;
use includes::{DirectIncludeUsages, IncludeGraph};

trait EntityExt {
    fn get_sourcefile(&self) -> Option<PathBuf>;
}

impl<'tu> EntityExt for Entity<'tu> {
    fn get_sourcefile(&self) -> Option<PathBuf> {
        if let Some(location) = self.get_location() {
            let location = location.get_expansion_location();
            if let Some(file) = location.file {
                Some(file.get_path())
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
                includes.insert(from, to.get_path());
            }
        }
    }

    EntityVisitResult::Continue
}

fn mark_includes(entity: Entity, includes: &mut DirectIncludeUsages) -> EntityVisitResult {
    if !entity.is_in_main_file() {
        return EntityVisitResult::Continue;
    }
    // println!("ref {:?}", entity);

    match entity.get_kind() {
        EntityKind::DeclRefExpr | EntityKind::TypeRef => {
            println!("ref {:?}", entity);
            if let Some(reference) = entity.get_reference() {
                println!(" -> ref {:?}", reference);
                if let Some(to) = reference.get_sourcefile() {
                    includes.mark_used(&to);
                }
            }
        }
        _ => {}
    }

    EntityVisitResult::Recurse
}

pub fn unused_includes<'a, P, S>(
    clang: &'a Clang,
    file: P,
    args: &[S],
) -> Result<Vec<(usize, PathBuf)>, ()>
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

            println!("includes {:?}", includes);

            let mut includes = includes.flatten(&PathBuf::from(file.as_ref()));

            println!("flatten {:?}", includes);

            tu.get_entity()
                .visit_children(|entity, _| mark_includes(entity, &mut includes));

            println!("unused {:?}", includes.unused());

            if let Some(file) = tu.get_file(&file) {
                let unused = includes.unused();
                let mut result = Vec::with_capacity(unused.len());

                file.visit_includes(|entity, source_range| {
                    if let Some(file) = entity.get_file() {
                        let path = file.get_path();
                        if unused.contains(&&path) {
                            let start = source_range.get_start().get_file_location();
                            result.push((start.line as usize, path));
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
