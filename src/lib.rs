use swc_core::common::comments::Comments;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::Program,
    visit::{FoldWith, VisitMut},
};
use swc_core::ecma::ast::{Callee, Expr, ExprOrSpread, KeyValueProp, Lit, Null};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};
use swc_core::plugin::proxies::PluginCommentsProxy;


#[plugin_transform]
pub fn loadable_components_plugin(mut program: Program, _metadata: TransformPluginProgramMetadata) -> Program {
    program.visit_mut_with(&mut loadable_transform(PluginCommentsProxy));

    program
}





pub fn loadable_transform<C>(comments: C) -> impl VisitMut
    where
        C: Comments,
{
    Loadable { comments }
}

struct Loadable<C>
    where
        C: Comments,
{
    comments: C,
}


// https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
impl<C> VisitMut for Loadable<C>
    where
        C: Comments,
{
    fn visit_mut_key_value_prop(&mut self, e: &mut KeyValueProp) {
        if !e.value.is_call() {
            return e.visit_mut_children_with(self);
        }

        // Since you have mutable access and need to possibly modify children,
        // You need to use a mutable pattern in if let:
        if let Expr::Call(call_expr) = &mut *e.value {
            println!("Running plugin transform on KeyValueProp: {:?}", call_expr.callee);
            // callee is identifier
            // Correctly handling the Callee type
            if let Callee::Expr(expr) = &call_expr.callee {
                if let Expr::Ident(ident) = &**expr {
                    println!("Callee is an identifier: {:?}", ident);

                    if ident.sym.to_string() == "Component" {
                        println!("Yes3 {:?}", call_expr);
                        call_expr.args = vec![ExprOrSpread::from(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))];
                    }
                }
            }
            call_expr.visit_mut_children_with(self);
        }

    }
}
