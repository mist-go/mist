use mist_parser::ast::*;

use crate::fmt::{Context, GenMist, GetMist, MistCodegen};

impl GenMist for ImplDecl {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        cg.add_indented("impl ");
        let generics = self.generics.get_mist();
        if !generics.is_empty() {
            cg.add(&generics);
            cg.add(" ");
        }
        if let Some(trait_) = &self.trait_ {
            cg.add(&trait_.get_mist());
            cg.add(" for ");
        }

        cg.add(&self.target.get_mist());

        cg.start_bracket();

        for method in &self.methods {
            method.gen_mist(ctx, cg);
        }

        cg.indent -= 1;
        cg.add_indentedln("}");
    }
}

impl GenMist for FunctionDecl {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        cg.add_indented(&self.visibility.get_mist());

        if self.return_type.is_none() {
            cg.add("void ");
        } else {
            cg.add(&self.return_type.as_ref().unwrap().get_mist());
            cg.add(" ");
        }

        cg.add(&self.name.get_mist());
        cg.add(&self.generics.get_mist());
        cg.add("(");

        if let Some((is_ref, lifetime, is_mut)) = &self.self_param {
            if *is_ref {
                cg.add("&");
            }
            if let Some(lifetime) = lifetime {
                cg.add(&format!("'{} ", lifetime.0));
            }
            if *is_mut {
                cg.add("mut ");
            }
            cg.add("self");
            if !self.params.0.is_empty() {
                cg.add(", ");
            }
        }

        for (i, param) in self.params.0.iter().enumerate() {
            if i > 0 {
                cg.add(", ");
            }
            param.gen_mist(ctx, cg);
        }

        cg.add(")");

        if let Some(override_spec) = &self.is_override {
            cg.add(" override");
            if let Some(path) = &override_spec.0 {
                cg.add("(");
                cg.add(&path.get_mist());
                cg.add(")");
            }
        }

        if let Some(body) = &self.body {
            cg.start_indent();
            body.gen_mist(ctx, cg);
            cg.addln("");
        } else {
            cg.addln(";");
        }
    }
}

impl GenMist for ParamList {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        for (i, param) in self.0.iter().enumerate() {
            if i > 0 {
                cg.add(", ");
            }
            param.gen_mist(ctx, cg);
        }
    }
}

impl GenMist for VarDecl {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        if let Some(type_) = &self.type_ {
            cg.add(&type_.get_mist());
            cg.add(" ");
        }
        self.name.gen_mist(ctx, cg);
    }
}

impl GetMist for FieldDecl {
    fn get_mist(&self) -> String {
        format!(
            "{}{} {}",
            self.visibility.get_mist(),
            self.type_.get_mist(),
            self.name.get_mist(),
        )
    }
}

impl GetMist for EnumItem {
    fn get_mist(&self) -> String {
        match self {
            Self::Named(id) => id.get_mist(),
            Self::Struct(id, s) => format!(
                "{} {{{}}}",
                id.get_mist(),
                s.iter()
                    .map(FieldDecl::get_mist)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(id, t) => format!(
                "{}({})",
                id.get_mist(),
                t.iter()
                    .map(TypeExpr::get_mist)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GenMist for TopLevel {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        if let TopLevelKind::ModAttribute = self.0.item {
            for attr in &self.1 {
                cg.add("#![");
                attr.gen_mist(ctx, cg);
                cg.addln("]");
            }
        } else {
            for attr in &self.1 {
                cg.add("#[");
                attr.gen_mist(ctx, cg);
                cg.addln("]");
            }
        }

        self.0.gen_mist(ctx, cg);
    }
}

impl GenMist for TopLevelKind {
    fn gen_mist(&self, ctx: &mut Context, cg: &mut MistCodegen) {
        match self {
            Self::ModAttribute => {}
            Self::Import(vis, path) => {
                cg.addln(&format!("{}use {};", vis.get_mist(), path.get_mist()))
            }
            Self::DeclareModule(vis, name) => {
                cg.addln(&format!("{}module {};", vis.get_mist(), name.get_mist()))
            }
            Self::FunctionDecl(decl) => decl.gen_mist(ctx, cg),
            Self::ImplDecl(impl_) => impl_.gen_mist(ctx, cg),
            Self::StructDecl {
                visibility,
                name,
                generics,
                fields,
            } => {
                cg.add(&format!(
                    "{}struct {}{}",
                    visibility.get_mist(),
                    name.get_mist(),
                    generics.get_mist()
                ));

                if fields.is_empty() {
                    cg.add(" {}");
                } else {
                    cg.start_bracket();

                    for field in fields {
                        cg.add_indentedln(&format!("{},", field.item.get_mist()));
                    }

                    cg.indent -= 1;
                    cg.add_indented("}");
                }

                cg.addln("");
            }
            Self::EnumDecl {
                visibility,
                name,
                generics,
                fields,
            } => {
                cg.add(&format!(
                    "{}enum {}{}",
                    visibility.get_mist(),
                    name.get_mist(),
                    generics.get_mist()
                ));

                if fields.is_empty() {
                    cg.add(" {}");
                } else {
                    cg.start_bracket();

                    for field in fields {
                        cg.add_indentedln(&format!("{},", field.item.get_mist()));
                    }

                    cg.indent -= 1;
                    cg.add_indented("}");
                }

                cg.addln("");
            }
            Self::TraitDecl {
                visibility,
                name,
                generics,
                requirements,
                items,
            } => {
                cg.add(&format!(
                    "{}trait {}{}",
                    visibility.get_mist(),
                    name.get_mist(),
                    generics.get_mist()
                ));
                if !requirements.is_empty() {
                    cg.add(" : ");
                    cg.add(
                        &requirements
                            .iter()
                            .map(TypeExpr::get_mist)
                            .collect::<Vec<_>>()
                            .join(" + "),
                    );
                }
                cg.start_bracket();
                for item in items {
                    item.gen_mist(ctx, cg);
                }
                cg.indent -= 1;
                cg.add_indentedln("}");
            }
            Self::ClassDecl {
                visibility,
                name,
                generics,
                inherits,
                fields,
                constructor,
                items,
            } => {
                cg.add(&format!(
                    "{}class {}{}",
                    visibility.get_mist(),
                    name.get_mist(),
                    generics.get_mist()
                ));
                if let Some(inherits) = inherits {
                    cg.add(" : ");
                    cg.add(&inherits.get_mist());
                }

                cg.start_bracket();

                for field in fields {
                    cg.add_indented(&field.item.decl.get_mist());
                    if let Some(init) = &field.item.init {
                        cg.add(" = ");
                        init.gen_mist(ctx, cg);
                    }
                    cg.addln(";");
                }

                if let Some(constructor) = constructor {
                    if fields.len() > 0 {
                        cg.addln("");
                    }

                    cg.add_indented(&constructor.item.visibility.get_mist());
                    cg.add("constructor");
                    cg.add(&constructor.item.generics.get_mist());
                    cg.add("(");
                    for (i, param) in constructor.item.params.0.iter().enumerate() {
                        if i > 0 {
                            cg.add(", ");
                        }
                        param.gen_mist(ctx, cg);
                    }
                    cg.addln(") ");
                    cg.add_indented("");
                    constructor.item.body.gen_mist(ctx, cg);
                    cg.addln("");

                    if items.len() > 0 {
                        cg.addln("");
                    }
                }

                for (i, item) in items.iter().enumerate() {
                    if i > 0 {
                        cg.addln("");
                    }

                    match item {
                        ClassItem::Method(method) => method.gen_mist(ctx, cg),
                        ClassItem::ImplDecl(impl_) => impl_.gen_mist(ctx, cg),
                    }
                }

                cg.indent -= 1;
                cg.add_indentedln("}");
            }
        }
    }
}
