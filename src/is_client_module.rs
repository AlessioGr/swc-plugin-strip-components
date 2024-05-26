/// This code has been taken from https://github.com/vercel/turbo/blob/main/crates/turbopack-ecmascript-plugins/src/transform/directives/mod.rs

use swc_core::ecma::ast::Program;
use swc_core::ecma::ast::{Lit};

macro_rules! has_directive {
    ($stmts:expr, $name:literal) => {
        $stmts
            .map(|item| {
                if let Lit::Str(str) = item?.as_expr()?.expr.as_lit()? {
                    Some(str)
                } else {
                    None
                }
            })
            .take_while(Option::is_some)
            .map(Option::unwrap)
            .any(|s| &*s.value == $name)
    };
}

pub fn is_client_module(program: &Program) -> bool {
    match program {
        Program::Module(m) => {
            has_directive!(m.body.iter().map(|item| item.as_stmt()), "use client")
        }
        Program::Script(s) => has_directive!(s.body.iter().map(Some), "use client"),
    }
}
