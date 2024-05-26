#![feature(box_patterns)]

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

    ///
    /// This is used to lobotomize client components (so, if 'use client' is at the top of the file), by making sure it only contains stuff which is exported, and that those exports only return null
    ///
    fn visit_mut_program(&mut self, program: &mut Program) {
        program.visit_mut_children_with(self);

        if !self.lobotomize_use_client_files {
            return;
        }

        if let Program::Module(ref mut module) = program {
            // Check for "use client" declaration at the top

            let contains_use_client = module.body.iter().any(|item| {
                matches!(item, ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr: box Expr::Lit(Lit::Str(Str { value, .. })), .. })) if value.trim().eq_ignore_ascii_case("use client"))
            });

            if contains_use_client {
                // Collect re-exported imports
                let mut reexported_imports = collect_reexported_imports(module);


                // Filter module body to strip imports and keep necessary parts
                module.body.retain(|item| match item {
                    ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr: box Expr::Lit(Lit::Str(Str { value, .. })), .. })) if value.trim().eq_ignore_ascii_case("use client") => true,
                    ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import.specifiers.iter().any(|specifier| {
                        match specifier {
                            ImportSpecifier::Named(named) => reexported_imports.contains(&named.local.sym),
                            ImportSpecifier::Default(default) => reexported_imports.contains(&default.local.sym),
                            ImportSpecifier::Namespace(namespace) => reexported_imports.contains(&namespace.local.sym),
                        }
                    }),
                    ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => export.specifiers.iter().any(|specifier| {
                        matches!(specifier, ExportSpecifier::Named(ExportNamedSpecifier { orig: ModuleExportName::Ident(ident), .. }) if reexported_imports.contains(&ident.sym))
                    }),
                    _ => true,
                });

                // Collect exported identifiers
                let exported_identifiers = collect_exported_identifiers(module);

                // Transform the module to retain and modify exported items
                transform_exports(module, &exported_identifiers);
            }
        }

    }

}

fn collect_reexported_imports(module: &Module) -> HashSet<JsWord> {
    let mut reexported_imports = HashSet::new();
    for item in &module.body {
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) = item {
            for specifier in &export.specifiers {
                if let ExportSpecifier::Named(ExportNamedSpecifier { orig: ModuleExportName::Ident(ident), .. }) = specifier {
                    reexported_imports.insert(ident.sym.clone());
                }
            }
        }
    }
    reexported_imports
}

fn collect_exported_identifiers(module: &Module) -> HashSet<JsWord> {
    let mut exported_identifiers = HashSet::new();
    for item in &module.body {
        if let ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) = item {
            for specifier in &export.specifiers {
                if let ExportSpecifier::Named(ExportNamedSpecifier { orig: ModuleExportName::Ident(ident), .. }) = specifier {
                    exported_identifiers.insert(ident.sym.clone());
                }
            }
        } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl: Decl::Var(var), .. })) = item {
            for decl in &var.decls {
                if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                    exported_identifiers.insert(sym.clone());
                }
            }
        }
    }
    exported_identifiers
}


/// Transforms the module by modifying exported variable initializers to `null`
/// or an empty function returning `null`, and removes variable declarations
/// that are not exported.
///
/// This function performs the following steps:
/// 1. Iterates through the module items and processes export declarations to
///    transform their initializers.
/// 2. Transforms standalone variable declarations to ensure they are exported,
///    marking those that are not for removal.
/// 3. Removes variable declarations that were marked for removal.
///
/// # Parameters
/// - `module`: The module to transform.
/// - `exported_identifiers`: A set of identifiers that are exported.
fn transform_exports(module: &mut Module, exported_identifiers: &HashSet<JsWord>) {
    let mut items_to_remove = Vec::new();

    for item in &mut module.body {
        match item {
            // Transform export declarations
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ref mut export)) => {
                if let Decl::Var(var) = &mut export.decl {
                    for decl in &mut var.decls {
                        transform_decl_init(decl);
                    }
                } else if let Decl::Fn(func) = &mut export.decl {
                    func.function.body = Some(empty_function_body());
                }
            },
            // Transform standalone variable declarations
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
                var.decls.retain_mut(|decl| {
                    if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                        if !exported_identifiers.contains(sym) {
                            // Mark this variable declaration for removal
                            items_to_remove.push(sym.clone());
                            return false;
                        } else {
                            transform_decl_init(decl);
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

fn transform_decl_init(decl: &mut VarDeclarator) {
    if let Some(init) = &mut decl.init {
        if matches!(**init, Expr::Arrow(_) | Expr::Fn(_)) {
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

fn empty_function_body() -> BlockStmt {
    BlockStmt {
        span: DUMMY_SP,
        stmts: vec![Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))),
        })],
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
