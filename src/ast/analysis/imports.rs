///
/// Functions related to walking imports, functions, and modules
///
use super::block::walk_ast;
use super::*;
use ast::node::*;
use location::Located;
use std::collections::HashSet;

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
    strings: &StringStore,
    scope: &mut Scope,
) -> WalkResult {
    match &import.kind {
        ImportKind::All => {
            match &import.module.data {
                Module::Custom(mod_ind) => {
                    for (fn_ind, proc) in modules[*mod_ind].procedures.iter_enumerated() {
                        if proc.public {
                            scope.import_function(proc.identifier.data, *mod_ind, fn_ind);
                        }
                    }
                }
                Module::Native(_pkg) => {
                    // Branch should be unreachable.  Native all imports are rewritten
                    unreachable!();
                }
            };
        }
        ImportKind::ModuleObject(Located { data, .. }) => {
            scope.import_module(*data, import.module.data.clone());
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

                let contains = match &import.module.data {
                    Module::Custom(ind) => modules[*ind].get_procedure(name).is_some(),
                    Module::Native(pkg) => {
                        pkg.fn_from_str(&*strings[StringVal::from(name)]).is_some()
                    }
                };

                if !contains {
                    return Err(WalkError {
                        start: located.start.clone(),
                        end: located.end.clone(),
                        kind: WalkErrorType::IdentifierNotFound(name),
                    });
                }

                match &import.module.data {
                    Module::Custom(mod_ind) => {
                        if let Some(fnc_ind) = modules[*mod_ind].get_procedure(name) {
                            scope.import_function(name, *mod_ind, fnc_ind);
                        }
                    }
                    Module::Native(pkg) => {
                        if let Some(fnc) = pkg.fn_from_str(&*strings[StringVal::from(name)]) {
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
    module_ind: ModuleInd,
    modules: &ModuleStore,
    lams: &mut Lams,
    cap: &mut CaptureStack,
    strings: &StringStore,
) -> WalkResult {
    let mut scope = Scope::new();

    for import in &module.imports {
        if Module::Custom(module_ind) == import.module.data {
            return Err(WalkError {
                start: import.module.start.clone(),
                end: import.module.end.clone(),
                kind: WalkErrorType::SelfImport,
            });
        }
        walk_import(import, modules, strings, &mut scope)?;
    }

    for (ind, Procedure { identifier, .. }) in module.procedures.iter_enumerated() {
        if !scope.insert_fn(identifier.data, ind, Some(module_ind)) {
            return Err(WalkError {
                start: identifier.start.clone(),
                end: identifier.end.clone(),
                kind: WalkErrorType::InvalidRedefinition(identifier.data),
            });
        }
    }

    for p in &mut module.procedures {
        walk_procedure(p, &mut scope, lams, cap)?;
    }

    Ok(())
}
