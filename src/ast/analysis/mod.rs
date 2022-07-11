mod block;
mod expression;
mod imports;
mod lambda;
mod scope;

use self::block::walk_ast;
use self::imports::*;
use self::scope::*;

use ast::node::*;
use location::Location;

pub type WalkResult = Result<(), WalkError>;
type Lambdas = Vec<Lambda>;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize, Copy)]
pub enum WalkErrorType {
    ImmutableModification(usize),
    IdentifierNotFound(usize),
    InvalidRedefinition(usize),
    InvalidBreak,
    InvalidReturn,
    InvalidContinue,
    InvalidThis,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct WalkError {
    start: Location,
    end: Location,
    kind: WalkErrorType,
}

pub struct Lams<'a> {
    lambdas: &'a mut Lambdas,
    getters: &'a mut Vec<GetProp>,
    setters: &'a mut Vec<SetProp>,
}

pub fn ref_check(program: &mut Program) -> WalkResult {
    let body = &mut program.body;
    let modules = &mut program.modules;

    let mut scope = Scope::new();
    let mut stack = CaptureStack::new();

    let mut lambdas = Lams {
        lambdas: &mut program.lambdas,
        getters: &mut program.getters,
        setters: &mut program.setters,
    };

    for mod_ind in 0..modules.len() {
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

    for (ind, Procedure { identifier, .. }) in program.functions.iter().enumerate() {
        if !scope.insert_fn(identifier.data, ind, None) {
            return Err(WalkError {
                start: identifier.start.clone(),
                end: identifier.end.clone(),
                kind: WalkErrorType::InvalidRedefinition(identifier.data),
            });
        }
    }

    for function in &mut program.functions {
        walk_procedure(function, &mut scope, &mut lambdas, &mut stack)?;
    }

    walk_ast(body, &mut scope, &mut lambdas, &mut stack)?;
    scope.check_decls(body);

    Ok(())
}
