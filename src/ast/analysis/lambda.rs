use super::*;
use super::{block::walk_ast, expression::walk_expression};
use ast::node::*;

pub trait LambdaLike {
    fn add_params(&self, scope: &mut Scope);
    fn check(&mut self, scope: &mut Scope, cap: &mut CaptureStack, top_stack: &Scope);
    fn get_body(&mut self) -> &mut LambdaBody;
}

impl LambdaLike for Lambda {
    fn add_params(&self, scope: &mut Scope) {
        scope.add_params(&self.param_list);
    }

    fn check(&mut self, scope: &mut Scope, cap: &mut CaptureStack, top_stack: &Scope) {
        scope.check_params(&mut self.param_list);
        self.capture = cap.pop(top_stack);
    }

    fn get_body(&mut self) -> &mut LambdaBody {
        &mut self.body
    }
}

impl LambdaLike for GetProp {
    fn add_params(&self, _: &mut Scope) {}

    fn check(&mut self, _: &mut Scope, cap: &mut CaptureStack, top_stack: &Scope) {
        self.capture = cap.pop(top_stack);
    }

    fn get_body(&mut self) -> &mut LambdaBody {
        &mut self.block
    }
}

impl LambdaLike for SetProp {
    fn add_params(&self, scope: &mut Scope) {
        scope.insert_mut(self.param);
    }

    fn check(&mut self, scope: &mut Scope, cap: &mut CaptureStack, top_stack: &Scope) {
        self.param_captured = scope.take_captured(self.param);
        self.capture = cap.pop(top_stack);
    }

    fn get_body(&mut self) -> &mut LambdaBody {
        &mut self.block
    }
}

fn walk_lambda_block(
    block: &mut LambdaBody,
    scope: &mut Scope,
    lams: &mut Lams,
    cap: &mut CaptureStack,
) -> WalkResult {
    match block {
        LambdaBody::Block(block) => {
            let res = walk_ast(block, scope, lams, cap);
            scope.check_decls(block);
            res
        }
        LambdaBody::ImplicitReturn(expr) => walk_expression(expr, scope, lams, cap),
    }
}

pub fn eval_lambda<T: LambdaLike>(
    lam: &mut T,
    scope: &mut Scope,
    cap: &mut CaptureStack,
    lams: &mut Lams,
) -> WalkResult {
    scope.sub_with(SubState::InFunc, |scope| {
        let second = scope.clone();
        cap.add(scope.level);

        lam.add_params(scope);
        walk_lambda_block(lam.get_body(), scope, lams, cap)?;

        lam.check(scope, cap, &second);
        Ok(())
    })
}
