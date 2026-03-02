mod block;
mod expression;
mod imports;
mod lambda;
mod scope;

use self::block::walk_ast;
use self::imports::*;
use self::scope::*;

use ast::node::*;
use location::Located;
use serde::{Deserialize, Serialize};

pub type WalkResult = Result<(), WalkError>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum WalkErrorType {
    ImmutableModification(Identifier),
    IdentifierNotFound(Identifier),
    InvalidRedefinition(Identifier),
    SelfImport,
    InvalidBreak,
    InvalidReturn,
    InvalidContinue,
    InvalidThis,
}

// #[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
// pub struct WalkError {
//     kind: WalkErrorType,
//     loc: LocationSpan,
// }

type WalkError = Located<WalkErrorType>;

pub struct Lams<'a> {
    lambdas: &'a mut Lambdas,
    getters: &'a mut Getters,
    setters: &'a mut Setters,
}

pub fn ref_check(program: &mut Program) -> Result<(), WalkError> {
    let body = &mut program.body;
    let modules = &mut program.modules;

    let mut scope = Scope::new();
    let mut stack = CaptureStack::new();

    let mut lambdas = Lams {
        lambdas: &mut program.lambdas,
        getters: &mut program.getters,
        setters: &mut program.setters,
    };

    for mod_ind in (0..modules.len()).map(ModuleInd::from) {
        let mut module = std::mem::take(&mut modules[mod_ind]);
        walk_module(
            &mut module,
            mod_ind,
            modules,
            &mut lambdas,
            &mut stack,
            &program.strings,
        )?;
        modules[mod_ind] = module;
    }

    for import in &program.imports {
        walk_import(import, &program.modules, &program.strings, &mut scope)?;
    }

    for (ind, Procedure { identifier, .. }) in program.procedures.iter_enumerated() {
        if !scope.insert_fn(identifier.data, ind, None) {
            return Err(WalkError {
                data: WalkErrorType::InvalidRedefinition(identifier.data),
                loc: identifier.loc,
            });
        }
    }

    for procedure in &mut program.procedures {
        walk_procedure(procedure, &mut scope, &mut lambdas, &mut stack)?;
    }

    walk_ast(body, &mut scope, &mut lambdas, &mut stack)?;
    scope.check_decls(body);

    Ok(())
}
