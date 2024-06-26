#![feature(box_patterns)]

mod is_client_module;

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
use crate::is_client_module::is_client_module;


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
                        call_expr.args = vec![ExprOrSpread::from(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP }))))];
                    }
                }
            }
        }
    }



    /// This method is used to lobotomize client components (if 'use client' is at the top of the file),
    /// by making sure it only contains stuff which is exported, and that those exports only return null.
    fn visit_mut_program(&mut self, program: &mut Program) {
        program.visit_mut_children_with(self);

        if !self.lobotomize_use_client_files {
            return;
        }

        // Check for "use client" declaration at the top
        let is_client_module = is_client_module(program);

        if let Program::Module(ref mut module) = program {

            if is_client_module {
                // Collect re-exported imports
                let reexported_imports = collect_reexported_imports(module);
                // Collect identifiers of items that are exported
                let exported_identifiers = collect_exported_identifiers(module);

                // Filter module body to strip unnecessary parts and keep necessary parts. Everything else is retained.
                module.body.retain(|item| match item {
                    // Keep the "use client" directive
                    ModuleItem::Stmt(Stmt::Expr(ExprStmt { expr: box Expr::Lit(Lit::Str(Str { value, .. })), .. })) if value.trim().eq_ignore_ascii_case("use client") => true,
                    // Keep imports that are re-exported
                    ModuleItem::ModuleDecl(ModuleDecl::Import(import)) => import.specifiers.iter().any(|specifier| {
                        match specifier {
                            // Check named imports
                            ImportSpecifier::Named(named) => reexported_imports.contains(&named.local.sym),
                            // Check default imports
                            ImportSpecifier::Default(default) => reexported_imports.contains(&default.local.sym),
                            // Check namespace imports
                            ImportSpecifier::Namespace(namespace) => reexported_imports.contains(&namespace.local.sym),
                        }
                    }),
                    // Keep re-exported named exports
                    ModuleItem::ModuleDecl(ModuleDecl::ExportNamed(export)) => export.specifiers.iter().any(|specifier| {
                        matches!(specifier, ExportSpecifier::Named(ExportNamedSpecifier { orig: ModuleExportName::Ident(ident), .. }) if reexported_imports.contains(&ident.sym))
                    }),
                    // Keep exported variable declarations
                    ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => {
                        var.decls.iter().any(|decl| {
                            if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                                // Retain if the variable is exported
                                return exported_identifiers.contains(&sym);
                            } else {
                                return true
                            }
                        })
                    },
                    // Keep exported function declarations
                    ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func))) => exported_identifiers.contains(&func.ident.sym),
                    // Retain other items
                    _ => true,
                });

                // Transform the module to modify exported items
                // The `transform_exports` function processes the remaining items in `module.body`.
                // It performs the following transformations:
                // 1. For exported functions, it converts them to empty functions returning `null`.
                // 2. For exported variables, it sets their values to `null`.
                // This ensures that any function or variable that remains in the module is
                // effectively lobotomized, preventing any actual implementation from being executed while
                // still allowing other files that import these exports to function without errors.
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

/// Collects identifiers of items that are exported from the module.
///
/// # Parameters
/// - `module`: The module to collect exported identifiers from.
///
/// # Returns
/// A set of `JsWord` containing the identifiers of exported items.
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
        } else if let ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl: Decl::Fn(func), .. })) = item {
            exported_identifiers.insert(func.ident.sym.clone());
        }
    }
    exported_identifiers
}


/// Solely handles transformation of items which are retained to be set to null or return null.
/// This does not handle removing any items from the module body completely.
fn transform_exports(module: &mut Module, exported_identifiers: &HashSet<JsWord>) {
    for item in &mut module.body {
        match item {
            // Handle variable declarations (e.g., `const myVar = ...;` and `export const myVar = ...;`)
            //
            // Example:
            // const myVar = 42;
            // =>
            // const myVar = null;
            //
            // Example:
            // export const myVar = 42;
            // =>
            // export const myVar = null;
            //
            // Example:
            // const myFunc = () => 42;
            // =>
            // const myFunc = () => null;
            //
            // Example:
            // export const myFunc = () => 42;
            // =>
            // export const myFunc = () => null;
            ModuleItem::Stmt(Stmt::Decl(Decl::Var(var))) => { // For non-exports
                for decl in &mut var.decls {
                    if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                        // Transform the variable if it's in the exported identifiers
                        if exported_identifiers.contains(sym) {
                            transform_decl_init(decl);
                        }
                    }
                }
            },
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl: Decl::Var(var), .. })) => {  // For exports - no need to check exported_identifiers
                for decl in &mut var.decls {
                    if let Pat::Ident(BindingIdent { id: Ident { sym, .. }, .. }) = &decl.name {
                        // Transform the variable if it's directly exported
                        transform_decl_init(decl);
                    }
                }
            },
            // Handle function declarations (e.g., `function myFunc() { ... }` and `export function myFunc() { ... }`)
            //
            // Example:
            // function myFunc() {
            //     return 42;
            // }
            // =>
            // function myFunc() {
            //     return null;
            //
            // Example:
            // export function myFunc() {
            //     return 42;
            // }
            // =>
            // export function myFunc() {
            //     return null;
            ModuleItem::Stmt(Stmt::Decl(Decl::Fn(func))) => { // For non-exports
                // Transform the function if it's in the exported identifiers
                if exported_identifiers.contains(&func.ident.sym) {
                    func.function.body = Some(empty_function_body());
                }
            },
            ModuleItem::ModuleDecl(ModuleDecl::ExportDecl(ExportDecl { decl: Decl::Fn(func), .. })) => { // For exports - no need to check exported_identifiers
                // Transform the function if it's directly exported
                func.function.body = Some(empty_function_body());
            },
            _ => {}
        }
    }
}

/// Transforms the initializer of a variable declaration to `null` or an empty
/// function returning `null`. This handles both constant variables and functions assigned
/// to variables.
///
/// # Parameters
/// - `decl`: The variable declarator to transform.
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

/// Creates an empty function body that returns `null`.
///
/// # Returns
/// A `BlockStmt` representing an empty function body returning `null`.
fn empty_function_body() -> BlockStmt {
    BlockStmt {
        span: DUMMY_SP,
        stmts: vec![Stmt::Return(ReturnStmt {
            span: DUMMY_SP,
            arg: Some(Box::new(Expr::Lit(Lit::Null(Null { span: DUMMY_SP })))),
        })],
    }
}


// This is the entry point
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
