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
            cg.add_indentedln("{\n");
            cg.indent += 1;
            body.gen_rust(ctx, cg);
            cg.indent -= 1;
            cg.add_indentedln("}\n");
        } else {
            cg.add(";");
        }
    }
}
