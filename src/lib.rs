use serde::Deserialize;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::*,
    visit::{as_folder, FoldWith, VisitMut},
};
use swc_core::ecma::ast::{BlockStmt, Callee, Decl, Expr, ExprOrSpread, ExprStmt, KeyValueProp, Lit, Module, ModuleDecl, ModuleItem, Null, Stmt};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};




// Helper function to transform exports
fn transform_exports(module: &mut Module) {
    for item in &mut module.body {
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ref mut export)) = item {
            match &mut export.decl {
                Decl::Var(var) => {
                    for decl in &mut var.decls {
                        if let Some(init) = &decl.init {
                            if let Expr::Arrow(_) | Expr::Fn(_) = &**init {
                                decl.init = Some(Box::new(Expr::Arrow(ArrowExpr {
                                    span: DUMMY_SP,
                                    params: vec![],
                                    body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))),
                                    is_async: false,
                                    is_generator: false,
                                    return_type: None,
                                    type_params: None,
                                })));
                                continue;
                            }
                        }
                        decl.init = Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))));
                    }
                },
                Decl::Fn(func) => {
                    func.function.body = Some(BlockStmt {
                        span: DUMMY_SP,
                        stmts: vec![Stmt::Return(ReturnStmt {
                            span: DUMMY_SP,
                            arg: Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))),
                        })],
                    });
                },
                _ => {}
            }
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct TransformVisitor
{
    pub identifier: String,
    pub lobotomize_use_client_files: bool
}

// https://rustdoc.swc.rs/swc_ecma_visit/trait.VisitMut.html
impl VisitMut for TransformVisitor {
    /**
     * This method is responsible for nullifying the props of a function call assigned to a component, if the function name matches the identifier.
     * E.g. if the identifier is ClientOnly, then this:
     *
     * const test = {
     *   hello: ClientOnly('world'),
     * }
     *
     * will be transformed to
     *
     * const test = {
     *   hello: ClientOnly(null),
     * }
     */
    fn visit_mut_key_value_prop(&mut self, e: &mut KeyValueProp) {
        e.visit_mut_children_with(self);

        if !e.value.is_call() {
            return
        }

        if let Expr::Call(call_expr) = &mut *e.value {
            if let Callee::Expr(expr) = &call_expr.callee {
                if let Expr::Ident(ident) = &**expr {

                    if ident.sym == self.identifier {
                        //println!("Is component! {:?}", call_expr);
                        call_expr.args = vec![ExprOrSpread::from(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))];
                        return;
                    }
                }
            }
        }
        return
    }


    fn visit_mut_program(&mut self, p: &mut Program) {
        p.visit_mut_children_with(self);

        if !self.lobotomize_use_client_files {
            return;
        }

        if let Program::Module(ref mut module) = p {
            //println!("Module11: {:?}", module.body);
            // Check for "use client" declaration at the top
            let use_client = module.body.iter().any(|item| {
                if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) = item {
                    if let Expr::Lit(Lit::Str(Str { value, .. })) = &**expr {
                        // Making the comparison case-insensitive and whitespace-insensitive
                        return value.trim().eq_ignore_ascii_case("use client");
                    }
                }
                false
            });

            //println!("use_client11: {:?}", use_client);


            if use_client {
                module.body.retain(|item| matches!(item, ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(_))));
                transform_exports(module);
            }
        }

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
