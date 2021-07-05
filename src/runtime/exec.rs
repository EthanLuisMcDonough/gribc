use ast::node::*;
use runtime::memory::*;
use runtime::operator::*;
use runtime::values::*;

fn scope_imports<'a>(scope: &mut Scope<'a>, gc: &mut Gc, program: &'a Program, import: &'a Import) {
    let imports = import
        .module
        .iter(program)
        .zip(import.module.names(program));

    match &import.kind {
        ImportKind::All => {
            for (callable, name) in imports {
                scope.declare_stack(gc, name, callable);
            }
        }
        ImportKind::List(hash) => {
            for (callable, name) in imports.filter(|(_, key)| hash.contains_key(*key)) {
                scope.declare_stack(gc, name, callable);
            }
        }
        ImportKind::ModuleObject(name) => scope.declare_stack(
            gc,
            &name.data,
            GribValue::ModuleObject(import.module.clone()),
        ),
    }
}

pub fn execute(program: &Program, config: GcConfig) {
    let mut gc = Gc::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        scope_imports(&mut scope, &mut gc, program, import);
    }

    for (index, fnc) in program.functions.iter().enumerate() {
        scope.declare_stack(
            &mut gc,
            &fnc.identifier.data,
            Callable::Procedure {
                module: None,
                index,
            },
        );
    }

    run_block(&program.body, &mut scope, &mut gc);
}

enum ControlFlow {
    Return(GribValue),
    None,
    Break,
    Continue,
}

fn run_block(block: &Block, scope: &mut Scope, gc: &mut Gc) {
    for node in block {
        match node {
            Node::Block(block) => run_block(block, scope, gc),
            Node::Break | Node::Continue => return,
            //Node::Declaration(decl) => decl.
            _ => unimplemented!(),
        }
    }
}

fn evaluate_hash() {}

pub fn evaluate_expression(expression: &Expression, scope: &mut Scope, gc: &mut Gc) -> GribValue {
    match expression {
        Expression::Bool(b) => GribValue::Bool(*b),
        Expression::Hash(h) => unimplemented!(),
        _ => unimplemented!(),
    }
}
