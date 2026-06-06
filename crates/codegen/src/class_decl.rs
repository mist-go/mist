use mist_parser::ast::*;

use crate::Context;

use crate::{GenRust, GetRust, RustCodegen};

pub fn class_decl(
    ctx: &mut Context,
    cg: &mut RustCodegen,

    visibility: &Visibility,
    name: &Identifier,
    generics: &GenericsDecl,
    inherits: &Option<TypeExpr>,
    fields: &Vec<Spanned<FieldDeclStmt>>,
    constructor: &Spanned<ClassConstructor>,
    items: &Vec<ClassItem>,
) {
    // Struct decl
    cg.addln(&format!(
        "{}struct {}{} {{",
        visibility.get_rust(),
        name.clone().get_rust(),
        generics.clone().get_rust()
    ));
    cg.indent += 1;

    if let Some(inherits) = inherits {
        cg.add_indented("pub _super: ");
        cg.add(&inherits.get_rust());
        cg.addln(",");
    }

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

    (fields, constructor).gen_rust(ctx, cg);

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
                impl_.item.target = TypeExpr::Path(Path(vec![name.clone()]), None);

                impl_.gen_rust(ctx, cg);
            }
            ClassItem::Method(_) => {}
        }
    }

    if let Some(inherits) = inherits {
        cg.add("impl std::ops::Deref for ");
        cg.add(&name.get_rust());

        cg.addln(" {");
        cg.indent += 1;

        cg.add_indented("type Target = ");
        cg.add(&inherits.get_rust());
        cg.addln(";");

        cg.add_indentedln("fn deref(&self) -> &Self::Target {&self._super}");

        cg.indent -= 1;
        cg.addln("}");

        // Mut

        cg.add("impl std::ops::DerefMut for ");
        cg.add(&name.get_rust());

        cg.addln(" {");
        cg.indent += 1;

        cg.add_indentedln("fn deref_mut(&mut self) -> &mut Self::Target {&mut self._super}");

        cg.indent -= 1;
        cg.addln("}");
    }
}

impl GenRust for (&Vec<Spanned<FieldDeclStmt>>, &Spanned<ClassConstructor>) {
    fn gen_rust(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.add_indented(&format!(
            "{}fn new{}(",
            self.1.item.visibility.get_rust(),
            self.1.item.generics.get_rust()
        ));

        let params = self
            .1
            .item
            .params
            .0
            .clone()
            .into_iter()
            .enumerate()
            .map(|(idx, mut v)| {
                v.name = construct_pattern(&v.name, idx);
                (idx, v)
            })
            .collect::<Vec<_>>();

        for (i, param) in &params {
            if *i > 0 {
                cg.add(", ");
            }

            param.gen_rust(ctx, cg);
        }

        cg.addln(") -> Self {");
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

        cg.add_indented(&format!("this.constructor("));

        for (i, param) in params {
            if i > 0 {
                cg.add(", ");
            }

            ctx.expr_ensure_semicolon = false;
            param.name.gen_rust(ctx, cg);
        }

        cg.addln(");");

        cg.add_indentedln("this");

        cg.indent -= 1;
        cg.add_indentedln("}\n");

        let mut constructor_params = vec![VarDecl {
            name: Pattern::Path(false, Path(vec![Identifier(String::from("self"))])),
            type_: Some(TypeExpr::Ref {
                lifetime: None,
                mutable: true,
                ty: Box::new(TypeExpr::Path(
                    Path(vec![Identifier(String::from("Self"))]),
                    None,
                )),
            }),
        }];

        constructor_params.append(&mut self.1.item.params.0.clone());

        Spanned {
            line: self.1.line,
            column: self.1.column,
            item: FunctionDecl {
                visibility: self.1.item.visibility.clone(),
                name: Identifier(String::from("constructor")),
                generics: self.1.item.generics.clone(),
                params: ParamList(constructor_params),
                return_type: Some(TypeExpr::Tuple(Vec::new())),
                body: Some(self.1.item.body.clone()),
            },
        }
        .gen_rust(ctx, cg);
    }
}

fn construct_pattern(pat: &Pattern, idx: usize) -> Pattern {
    match pat {
        Pattern::Literal(v) => Pattern::Literal(v.clone()),
        Pattern::Path(is_mut, v) => Pattern::Path(*is_mut, v.clone().into()),
        _ => Pattern::Path(false, Path(vec![Identifier(format!("_{idx}"))])),
    }
}
