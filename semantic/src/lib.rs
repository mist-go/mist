use std::{collections::HashMap, sync::Arc};

use parser::ast::{ParamList, TypeExpr};

use crate::{
    hir::VarRef,
    scope::{LocalScope, Scope},
};

pub mod hir;
pub mod scope;
pub mod top_level;

pub fn walk_ast(top_scope: Arc<Scope>, tl: &mut Vec<parser::ast::TopLevel>) {
    for tl in tl {
        match tl {
            parser::ast::TopLevel::Import(_) => unimplemented!(),
            parser::ast::TopLevel::Package(_) => {}

            parser::ast::TopLevel::FunctionDecl {
                params, body, name, ..
            } => {
                let rf = top_scope.get_reference(name).unwrap();
                *name = rf.name.clone();

                match &*rf.var_type {
                    hir::TypeRef::Function(f) => walk_param_list(&f.params, params),
                    _ => unimplemented!(),
                }

                let scope = LocalScope::new(top_scope.clone());

                scope.with_params(params);

                scope.with_block(body);
            }

            parser::ast::TopLevel::StructDecl { name, fields, .. } => {
                let rf = top_scope.get_reference(name).unwrap();
                *name = rf.name.clone();

                match &*rf.var_type {
                    hir::TypeRef::Struct(s) => walk_param_list(&s.fields, fields),
                    _ => unimplemented!(),
                }
            }
        }
    }
}

pub fn walk_param_list(fields: &HashMap<String, Arc<VarRef>>, param_list: &mut ParamList) {
    let old_param_list = param_list.clone();
    param_list.0.clear();

    for (name, param) in fields {
        param_list.0.insert(
            param.name.clone(),
            (
                old_param_list.0.get(name).map(|a| a.0).unwrap_or_default(),
                TypeExpr::Identifier(param.var_type.get_name()),
            ),
        );
    }
}
