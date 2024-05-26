use serde::Deserialize;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::Program,
    visit::{as_folder, FoldWith, VisitMut},
};
use swc_core::ecma::ast::{Callee, Expr, ExprOrSpread, KeyValueProp, Lit, Null};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};


#[derive(Debug, Deserialize)]
pub struct TransformVisitor
{
    pub identifier: String,
}

// https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
impl VisitMut for TransformVisitor {
    fn visit_mut_key_value_prop(&mut self, e: &mut KeyValueProp) {
        e.visit_mut_children_with(self);

        if !e.value.is_call() {
            return
        }

        if let Expr::Call(call_expr) = &mut *e.value {
            if let Callee::Expr(expr) = &call_expr.callee {
                if let Expr::Ident(ident) = &**expr {

                    if ident.sym == self.identifier {
                        println!("Is component! {:?}", call_expr);
                        call_expr.args = vec![ExprOrSpread::from(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))];
                        return;
                    }
                }
            }
        }
        return
    }
}


#[plugin_transform]
pub fn process_transform(program: Program, _metadata: TransformPluginProgramMetadata) -> Program {

    let strip_components_transform: TransformVisitor = serde_json::from_str(
        &_metadata
            .get_transform_plugin_config()
            .expect("failed to get plugin config"),
    )
        .expect("invalid config");


    program.fold_with(&mut as_folder(strip_components_transform))
}
