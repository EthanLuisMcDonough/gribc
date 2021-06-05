use ast::node::*;
use runtime::memory::*;

pub fn execute(program: &Program, config: GcConfig) {
    let mut gc = Gc::new(config);
    let mut scope = Scope::new();

    for import in &program.imports {
        match import.kind {
            ImportKind::All => 
        }
    }

    for proc in &program.functions {
        scope.set_constant(proc.identifier.data, GribValue::)
    }
}