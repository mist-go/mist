use std::sync::Arc;

use crate::scope::{LocalScope, Scope};

pub mod hir;
pub mod scope;
pub mod top_level;

pub fn walk_ast(top_scope: Arc<Scope>, tl: &mut Vec<parser::ast::TopLevel>) {
    for tl in tl {
        match tl {
            parser::ast::TopLevel::Import(_) => unimplemented!(),

            parser::ast::TopLevel::FunctionDecl {
                export,
                name,
                params,
                return_type,
                body,
            } => {
                let scope = LocalScope::new(top_scope.clone());

                scope.with_params(params);

                scope.with_block(body);

                println!("{:?}", scope);
            }

            parser::ast::TopLevel::StructDecl {
                export,
                name,
                fields,
            } => {}
        }
    }
}
