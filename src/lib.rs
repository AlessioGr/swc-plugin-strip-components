use std::collections::HashSet;
use serde::Deserialize;
use swc_core::atoms::JsWord;
use swc_core::common::DUMMY_SP;
use swc_core::ecma::{
    ast::*,
    visit::{as_folder, FoldWith, VisitMut},
};
use swc_core::ecma::ast::{BlockStmt, Callee, Decl, Expr, ExprOrSpread, ExprStmt, KeyValueProp, Lit, Module, ModuleDecl, ModuleItem, Null, Stmt};
use swc_core::ecma::visit::VisitMutWith;
use swc_core::plugin::{plugin_transform, proxies::TransformPluginProgramMetadata};





// Helper function to transform exports
fn transform_exports(module: &mut Module, exported_identifiers: &HashSet<JsWord>) {
    let mut items_to_remove = Vec::new();

    for item in &mut module.body {
        match item {
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ref mut export)) => {
                match &mut export.decl {
                    Decl::Var(var) => {
                        for decl in &mut var.decls {
                            if let Some(init) = &mut decl.init {
                                if let Expr::Arrow(_) | Expr::Fn(_) = &**init {
                                    *init = Box::new(Expr::Arrow(ArrowExpr {
                                        span: DUMMY_SP,
                                        params: vec![],
                                        body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))),
                                        is_async: false,
                                        is_generator: false,
                                        return_type: None,
                                        type_params: None,
                                    }));
                                } else {
                                    *init = Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })));
                                }
                            } else {
                                decl.init = Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))));
                            }
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
            },
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
                var.decls.retain_mut(|decl| {
                    if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                        if !exported_identifiers.contains(sym) {
                            // Mark this variable declaration for removal
                            items_to_remove.push(sym.clone());
                            return false;
                        } else {
                            if let Some(init) = &mut decl.init {
                                if let Expr::Arrow(_) | Expr::Fn(_) = &**init {
                                    *init = Box::new(Expr::Arrow(ArrowExpr {
                                        span: DUMMY_SP,
                                        params: vec![],
                                        body: Box::new(BlockStmtOrExpr::Expr(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))),
                                        is_async: false,
                                        is_generator: false,
                                        return_type: None,
                                        type_params: None,
                                    }));
                                } else {
                                    *init = Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })));
                                }
                            } else {
                                decl.init = Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))));
                            }
                        }
                    }
                    true
                });
            },
            _ => {}
        }
    }

    // Remove marked items
    module.body.retain(|item| {
        if let ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) = item {
            return var.decls.iter().any(|decl| {
                if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                    return !items_to_remove.contains(&sym);
                }
                true
            });
        }
        true
    });
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
            // Check for "use client" declaration at the top
            let mut has_use_client = false;
            for item in &module.body {
                if let ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) = item {
                    if let Expr::Lit(Lit::Str(Str { value, .. })) = &**expr {
                        if value.trim().eq_ignore_ascii_case("use client") {
                            has_use_client = true;
                            break;
                        }
                    }
                }
            }

            if has_use_client {
                // Preserve "use client" and strip imports, except re-exported ones
                let mut reexported_imports = HashSet::new();

                // Collect re-exported imports
                for item in &module.body {
                    if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) = item {
                        for specifier in &export.specifiers {
                            if let ExportSpecifier::Named(ExportNamedSpecifier { orig, exported, .. }) = specifier {
                                if let Some(ModuleExportName::Ident(ident)) = exported {
                                    reexported_imports.insert(ident.sym.clone());
                                } else if let ModuleExportName::Ident(ident) = orig {
                                    reexported_imports.insert(ident.sym.clone());
                                }
                            }
                        }
                    }
                }

                // Filter module body to strip imports and keep necessary parts
                module.body.retain(|item| {
                    match item {
                        ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr, .. })) => {
                            if let Expr::Lit(Lit::Str(Str { value, .. })) = &**expr {
                                if value.trim().eq_ignore_ascii_case("use client") {
                                    return true;
                                }
                            }
                        }
                        ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => {
                            // Retain import if it is re-exported
                            return import.specifiers.iter().any(|specifier| {
                                match specifier {
                                    ImportSpecifier::Named(named) => {
                                        reexported_imports.contains(&named.local.sym)
                                    }
                                    ImportSpecifier::Default(default) => {
                                        reexported_imports.contains(&default.local.sym)
                                    }
                                    ImportSpecifier::Namespace(namespace) => {
                                        reexported_imports.contains(&namespace.local.sym)
                                    }
                                }
                            });
                        }
                        ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => {
                            // Remove named exports that are not directly re-exporting
                            return export.specifiers.iter().any(|specifier| {
                                if let ExportSpecifier::Named(ExportNamedSpecifier { orig, exported, .. }) = specifier {
                                    if let Some(ModuleExportName::Ident(ident)) = exported {
                                        return reexported_imports.contains(&ident.sym);
                                    } else if let ModuleExportName::Ident(ident) = orig {
                                        return reexported_imports.contains(&ident.sym);
                                    }
                                }
                                false
                            });
                        }
                        _ => {}
                    }
                    true
                });

                // Collect all identifiers that are exported
                let mut exported_identifiers = HashSet::new();
                for item in &module.body {
                    if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) = item {
                        for specifier in &export.specifiers {
                            if let ExportSpecifier::Named(ExportNamedSpecifier { orig, .. }) = specifier {
                                let ident = match orig {
                                    ModuleExportName::Ident(ident) => ident.sym.clone(),
                                    ModuleExportName::Str(_) => continue,
                                };
                                exported_identifiers.insert(ident);
                            }
                        }
                    } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl, .. })) = item {
                        if let Decl::Var(var) = decl {
                            for decl in &var.decls {
                                if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                                    exported_identifiers.insert(sym.clone());
                                }
                            }
                        }
                    }
                }

                // Transform the module to retain and modify exported items
                transform_exports(module, &exported_identifiers);
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
