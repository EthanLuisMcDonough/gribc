///
/// Functions related to walking imports, functions, and modules
///
use super::block::walk_ast;
use super::*;
use ast::node::*;
use location::Located;
use std::collections::HashSet;

type Strings<'a> = &'a Vec<String>;

pub fn walk_procedure(
    procedure: &mut Procedure,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> Result<(), WalkError> {
    scope.sub_fnc(
        |scope, _, body| walk_ast(body, scope, lams, cap),
        &mut procedure.param_list,
        &mut procedure.body,
    )
}

/// Checks imports for erroneous identifiers and adds their values to the scope
pub fn walk_import(
    import: &Import,
    modules: &ModuleStore,
    strings: Strings,
    scope: &mut Scope,
) -> WalkResult {
    match &import.kind {
        ImportKind::All => {
            match &import.module {
                Module::Custom(mod_ind) => {
                    for (fn_ind, proc) in modules[*mod_ind].pub_functions().enumerate() {
                        scope.import_function(proc.identifier.data, *mod_ind, fn_ind);
                    }
                }
                Module::Native(_pkg) => {
                    panic!("Branch should be unreachable.  Native all imports are rewritten");
                }
            };
        }
        ImportKind::ModuleObject(Located { data, .. }) => {
            scope.import_module(*data, import.module.clone());
        }
        ImportKind::List(list) => {
            let mut inserted = HashSet::with_capacity(list.len());

            for located in list {
                let name = located.data;

                if inserted.contains(&name) {
                    return Err(WalkError {
                        start: located.start.clone(),
                        end: located.end.clone(),
                        kind: WalkErrorType::InvalidRedefinition(name),
                    });
                }
                inserted.insert(name);

                let contains = match &import.module {
                    Module::Custom(ind) => modules[*ind].get_function(name).is_some(),
                    Module::Native(pkg) => pkg.fn_from_str(&*strings[name]).is_some(),
                };

                if !contains {
                    return Err(WalkError {
                        start: located.start.clone(),
                        end: located.end.clone(),
                        kind: WalkErrorType::IdentifierNotFound(name),
                    });
                }

                match &import.module {
                    Module::Custom(mod_ind) => {
                        if let Some(fnc_ind) = modules[*mod_ind].get_function(name) {
                            scope.import_function(name, *mod_ind, fnc_ind);
                        }
                    }
                    Module::Native(pkg) => {
                        if let Some(fnc) = pkg.fn_from_str(&*strings[name]) {
                            scope.native_function(name, fnc.clone());
                        }
                    }
                }
            }
        }
    }

    Ok(())
}

pub fn walk_module(
    module: &mut CustomModule,
    module_ind: usize,
    modules: &ModuleStore,
    lams: &mut Lams,
    cap: &mut CaptureStack,
    strings: Strings,
) -> WalkResult {
    let mut scope = Scope::new();

    for import in &module.imports {
        walk_import(import, modules, strings, &mut scope)?;
    }

    for (ind, Procedure { identifier, .. }) in module.functions.iter().enumerate() {
        if !scope.insert_fn(identifier.data, ind, Some(module_ind)) {
            return Err(WalkError {
                start: identifier.start.clone(),
                end: identifier.end.clone(),
                kind: WalkErrorType::InvalidRedefinition(identifier.data),
            });
        }
    }

    for p in &mut module.functions {
        walk_procedure(p, &mut scope, lams, cap)?;
    }

    Ok(())
}
