use mist_parser::ast::*;

use crate::Context;

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
        cg.add_indentedln(&format!(
            "{}fn {}{}(",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.generics.get_rust(),
        ));

        for (i, param) in self.params.0.iter().enumerate() {
            if i > 0 {
                cg.add(",");
            }

            param.gen_rust(ctx, cg);
        }

        cg.add(") -> ");
        cg.add(&self.return_type.get_rust());

        if let Some(body) = &self.body {
            cg.add_indentedln(" {\n");
            cg.indent += 1;
            body.gen_rust(ctx, cg);
            cg.indent -= 1;
            cg.add_indentedln("}\n");
        } else {
            cg.add(";");
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

impl GenRust for (&Vec<Spanned<FieldDeclStmt>>, &ClassConstructor) {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.add_indentedln(&format!(
            "{}fn new{}(",
            self.1.visibility.get_rust(),
            self.1.generics.get_rust()
        ));

        self.1.params.gen_rust(ctx, cg);

        cg.add(") -> Self {");
        cg.indent += 1;

        cg.add_indentedln("let mut this: Self = unsafe { std::mem::MaybeUninit::<Self>::zeroed().assume_init() };");

        for field in self.0 {
            let comment = field.get_comment();

            if let Some(init) = &field.item.init {
                cg.add_indentedln(&comment);

                cg.add_indentedln(&format!("this.{} = ", field.item.decl.name.get_rust()));

                init.gen_rust(ctx, cg);
            }
        }

        cg.add_indentedln(&format!("this.constructor("));

        for (i, param) in self.1.params.0.iter().enumerate() {
            if i > 0 {
                cg.add(", ");
            }

            param.name.gen_rust(ctx, cg);
        }

        cg.add(");");

        cg.add_indentedln("this");

        cg.indent -= 1;
        cg.add_indentedln("}\n");

        let mut constructor_params = vec![VarDecl {
            mutable: false,
            name: Pattern::Id(Identifier(String::from("self"))),
            type_: Some(TypeExpr(
                TypeExprKind::Path(Path(vec![Identifier(String::from("Self"))])),
                vec![TypePostfix::RefMut],
            )),
        }];

        constructor_params.append(&mut self.1.params.0.clone());

        FunctionDecl {
            visibility: self.1.visibility.clone(),
            name: Identifier(String::from("constructor")),
            generics: self.1.generics.clone(),
            params: ParamList(constructor_params),
            return_type: TypeExpr::no_px(TypeExprKind::Tuple(Vec::new())),
            body: Some(self.1.body.clone()),
        }
        .gen_rust(ctx, cg);
    }
}

impl GenRust for TopLevelKind {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        match self {
            Self::ModAttribute => {}
            Self::Import(vis, path) => {
                cg.addln(&format!("{}use {};", vis.get_rust(), path.get_rust()))
            }
            Self::Mod(vis, id) => cg.addln(&format!("{}mod {};", vis.get_rust(), id.get_rust())),
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
                    cg.add_indentedln(&field.get_rust());
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
                    cg.add_indentedln(&(field.get_rust() + ","));
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
                fields,
                constructor,
                items,
            } => {
                // Struct decl
                cg.addln(&format!(
                    "{}struct {}{} {{",
                    visibility.get_rust(),
                    name.clone().get_rust(),
                    generics.clone().get_rust()
                ));
                cg.indent += 1;

                for field in fields.clone() {
                    cg.add_indentedln(&field.get_comment());
                    cg.add_indentedln(&field.item.decl.get_rust());
                }

                cg.indent -= 1;
                cg.addln("}\n");

                // Constructor
                cg.addln(&format!(
                    "impl{} {}{} {{",
                    generics.clone().get_rust(),
                    name.clone().get_rust(),
                    format!(
                        "<{}>",
                        generics
                            .clone()
                            .0
                            .into_iter()
                            .map(|v| Generic::from(v).get_rust())
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                ));
                cg.indent += 1;

                let constructor_comment = constructor.get_comment();

                cg.add_indentedln("#[allow(invalid_value)]");
                cg.add_indentedln(&constructor_comment);

                (fields, &constructor.item).gen_rust(ctx, cg);

                for item in items.clone() {
                    match item {
                        ClassItem::ImplDecl(_) => {}
                        ClassItem::Method(method) => method.gen_rust(ctx, cg),
                    }
                }

                cg.indent -= 1;
                cg.addln("}\n");

                for item in items {
                    match item {
                        ClassItem::ImplDecl(impl_) => {
                            let mut impl_ = impl_.clone();

                            impl_.item.trait_ = Some(impl_.item.target);
                            impl_.item.target =
                                TypeExpr(TypeExprKind::Path(Path(vec![name.clone()])), Vec::new());

                            impl_.gen_rust(ctx, cg);
                        }
                        ClassItem::Method(_) => {}
                    }
                }
            }
        }
    }
}
