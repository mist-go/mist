use mist_parser::ast::*;

use crate::{Context, class_decl};

use crate::{GenRust, GetRust, RustCodegen};

impl GetRust for GenericsDecl {
    fn get_rust(&self) -> String {
        if self.0.len() == 0 {
            String::new()
        } else {
            format!(
                "<{}>",
                self.0
                    .iter()
                    .map(|v| v.get_rust())
                    .collect::<Vec<_>>()
                    .join(", ")
            )
        }
    }
}

impl GetRust for GenericDecl {
    fn get_rust(&self) -> String {
        match self {
            GenericDecl::Lifetime(name) => format!("'{}", name.get_rust()),
            GenericDecl::Type(name, requirements) => {
                name.get_rust()
                    + &(if requirements.len() != 0 {
                        format!(
                            ": {}",
                            requirements
                                .into_iter()
                                .map(TypeExpr::get_rust)
                                .collect::<Vec<_>>()
                                .join("+")
                        )
                    } else {
                        String::new()
                    })
            }
        }
    }
}

impl GenRust for ImplDecl {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        if let Some(trait_) = &self.trait_ {
            cg.add_indentedln(&format!(
                "impl{} {} for {} {{",
                self.generics.get_rust(),
                trait_.get_rust(),
                self.target.get_rust()
            ));
        } else {
            cg.add_indentedln(&format!(
                "impl{} {} {{",
                self.generics.get_rust(),
                self.target.get_rust()
            ));
        }
        cg.indent += 1;

        for method in &self.methods {
            method.gen_rust(ctx, cg);
        }

        cg.indent -= 1;
        cg.add_indentedln("}");
    }
}

impl GenRust for FunctionDecl {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.add_indented(&format!(
            "{}fn {}{}(",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.generics.get_rust(),
        ));

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

            cg.add("self,");
        }

        for (i, param) in self.params.0.iter().enumerate() {
            if i > 0 {
                cg.add(", ");
            }

            param.gen_rust(ctx, cg);
        }

        cg.add(") ");
        if let Some(return_type) = &self.return_type {
            cg.add("-> ");
            cg.add(&return_type.get_rust());
        }

        if let Some(body) = &self.body {
            cg.add(" ");
            body.gen_rust(ctx, cg);
            cg.addln("");
        } else {
            cg.addln(";");
        }
    }
}

impl GetRust for FieldDecl {
    fn get_rust(&self) -> String {
        format!(
            "{}{}: {},",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.type_.get_rust()
        )
    }
}

impl GetRust for EnumItem {
    fn get_rust(&self) -> String {
        match self {
            Self::Named(id) => id.get_rust(),
            Self::Struct(id, s) => format!(
                "{} {{{}}}",
                id.get_rust(),
                s.into_iter()
                    .map(|field| format!("{}: {}", field.name.get_rust(), field.type_.get_rust()))
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
            Self::Tuple(id, t) => format!(
                "{} ({})",
                id.get_rust(),
                t.into_iter()
                    .map(TypeExpr::get_rust)
                    .collect::<Vec<_>>()
                    .join(", ")
            ),
        }
    }
}

impl GenRust for ParamList {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for (i, param) in self.0.iter().enumerate() {
            if i > 0 {
                cg.add(", ");
            }

            param.gen_rust(ctx, cg);
        }
    }
}

impl GenRust for TopLevel {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        if let TopLevelKind::ModAttribute = self.0.item {
            for attr in &self.1 {
                cg.add("#![");
                attr.gen_rust(ctx, cg);
                cg.addln("]");
            }
        } else {
            for attr in &self.1 {
                cg.add("#[");
                attr.gen_rust(ctx, cg);
                cg.addln("]");
            }
        }

        self.0.gen_rust(ctx, cg);
    }
}

impl GenRust for TopLevelKind {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::ModAttribute => {}
            Self::TypeAlias { name, generics, ty } => {
                cg.add("type ");
                name.gen_rust(ctx, cg);
                if let Some(g) = generics {
                    g.gen_rust(ctx, cg);
                }
                cg.add(" = ");
                ty.gen_rust(ctx, cg);
                cg.addln(";");
            }
            Self::ConstDecl(decl) => {
                cg.add("const ");
                decl.decl.gen_rust(ctx, cg);

                if let Some(init) = &decl.init {
                    cg.add(" = ");
                    init.gen_rust(ctx, cg);
                }

                cg.addln(";");
            }
            Self::StaticDecl(decl) => {
                cg.add("static ");
                decl.decl.gen_rust(ctx, cg);

                if let Some(init) = &decl.init {
                    cg.add(" = ");
                    init.gen_rust(ctx, cg);
                }

                cg.addln(";");
            }
            Self::Import(vis, path) => {
                cg.addln(&format!("{}use {};", vis.get_rust(), path.get_rust()))
            }
            Self::DeclareModule(_, _) => {}
            Self::FunctionDecl(decl) => decl.gen_rust(ctx, cg),
            Self::ImplDecl(impl_) => impl_.gen_rust(ctx, cg),
            Self::StructDecl {
                visibility,
                name,
                generics,
                fields,
            } => {
                cg.addln(&format!(
                    "{}struct {}{} {{",
                    visibility.get_rust(),
                    name.get_rust(),
                    generics.get_rust()
                ));
                cg.indent += 1;

                for field in fields {
                    cg.add_indented("");
                    field.gen_rust(ctx, cg);
                    cg.addln("");
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }
            Self::EnumDecl {
                visibility,
                name,
                generics,
                fields,
            } => {
                cg.addln(&format!(
                    "{}enum {}{} {{",
                    visibility.get_rust(),
                    name.get_rust(),
                    generics.get_rust()
                ));
                cg.indent += 1;

                for field in fields {
                    cg.add_indented("");
                    field.gen_rust(ctx, cg);
                    cg.addln(",");
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }
            Self::TraitDecl {
                visibility,
                name,
                generics,
                requirements,
                items,
            } => {
                cg.addln(&format!(
                    "{}trait {}{}{} {{",
                    visibility.get_rust(),
                    name.get_rust(),
                    generics.get_rust(),
                    if requirements.len() != 0 {
                        String::from(": ")
                            + &requirements
                                .into_iter()
                                .map(TypeExpr::get_rust)
                                .collect::<Vec<_>>()
                                .join("+")
                    } else {
                        String::new()
                    },
                ));
                cg.indent += 1;

                for item in items {
                    item.gen_rust(ctx, cg);
                }

                cg.indent -= 1;
                cg.addln("}\n");
            }
            Self::ClassDecl {
                visibility,
                name,
                generics,
                inherits,
                fields,
                constructor,
                items,
            } => class_decl::class_decl(
                ctx,
                cg,
                visibility,
                name,
                generics,
                inherits,
                fields,
                constructor,
                items,
            ),
            Self::IncludeGlobal(incl) => {
                cg.crates
                    .entry(incl.0[0].clone())
                    .or_insert(Vec::new())
                    .push(incl.clone());
            }
        }
    }
}
