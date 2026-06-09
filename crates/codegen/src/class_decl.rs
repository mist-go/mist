use mist_parser::ast::*;

use crate::{Context, GenRust, GetRust, RustCodegen};

// ── Stage 1: Data Collection & Metadata Analysis ──────────────────────

pub struct ClassProcessedData {
    visibility: Visibility,
    name: Identifier,
    generics: GenericsDecl,
    inherits: Option<ExprPath>,
    self_path: ExprPath,
    self_ty: TypeExpr,
    fields: Vec<Spanned<FieldDeclStmt>>,
    constructor: Spanned<ClassConstructor>,
    items: Vec<ClassItem>,
    methods: Vec<Spanned<FunctionDecl>>,
    v_table: Vec<(Identifier, Option<Override>)>,
}

impl ClassProcessedData {
    pub fn analyze(
        visibility: &Visibility,
        name: &Identifier,
        generics: &GenericsDecl,
        inherits: &Option<ExprPath>,
        fields: &Vec<Spanned<FieldDeclStmt>>,
        constructor: &Spanned<ClassConstructor>,
        items: &Vec<ClassItem>,
    ) -> Self {
        let self_path = ExprPath(vec![ExprPathSegment {
            ident: name.clone(),
            generics: generics.clone().into(),
        }]);

        let self_ty = get_type_from_path(&self_path);

        let methods = items
            .clone()
            .into_iter()
            .filter_map(|item| match item {
                ClassItem::ImplDecl(_) => None,
                ClassItem::Method(method) => Some(method),
            })
            .collect::<Vec<_>>();

        let v_table = methods
            .iter()
            .filter_map(|method| match method.item.visibility {
                Visibility::Public => {
                    Some((method.item.name.clone(), method.item.is_override.clone()))
                }
                _ => None,
            })
            .collect::<Vec<_>>();

        ClassProcessedData {
            visibility: visibility.clone(),
            name: name.clone(),
            generics: generics.clone(),
            inherits: inherits.clone(),
            self_path,
            self_ty,
            fields: fields.clone(),
            constructor: constructor.clone(),
            items: items.clone(),
            methods,
            v_table,
        }
    }

    // ── Stage 2: Code Emission ───────────────────────────────────────

    pub fn emit(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        ctx.expr_super = self.inherits.clone();

        self.emit_struct_decl(cg);
        self.emit_impl_block(ctx, cg);
        self.emit_impl_decls(ctx, cg);
        self.emit_deref_impls(cg);

        ctx.expr_super = None;
    }

    fn emit_struct_decl(&self, cg: &mut RustCodegen) {
        cg.addln(&format!(
            "{}struct {}{} {{",
            self.visibility.get_rust(),
            self.name.get_rust(),
            self.generics.get_rust()
        ));
        cg.indent += 1;

        cg.add_indentedln(
            "pub _m_oop: (&'static [*const std::ffi::c_void], *mut std::ffi::c_void),",
        );

        if let Some(ref inherits) = self.inherits {
            cg.add_indented("pub _super: Box<");
            cg.add(&get_type_from_path(inherits).get_rust());
            cg.addln(">,");
        }

        for field in &self.fields {
            cg.add_indentedln(&field.get_comment());
            cg.add_indentedln(&field.item.decl.get_rust());
        }

        cg.indent -= 1;
        cg.addln("}\n");
    }

    fn emit_impl_block(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        cg.addln(&format!(
            "impl{} {} {{",
            self.generics.get_rust(),
            self.self_ty.get_rust()
        ));
        cg.indent += 1;

        self.emit_v_table(cg);

        if let Some(ref inherits) = self.inherits {
            self.emit_super_v_table(cg, inherits);
        }

        self.emit_constructor(ctx, cg);
        self.emit_methods(ctx, cg);

        cg.indent -= 1;
        cg.addln("}\n");
    }

    fn emit_v_table(&self, cg: &mut RustCodegen) {
        for (i, (method_name, _)) in self.v_table.iter().enumerate() {
            cg.add_indentedln(&format!(
                "pub const __FN_{}: usize = {i};",
                method_name.0.to_uppercase()
            ));
        }

        cg.add_indentedln(&format!(
            "pub const __V_TABLE: [*const std::ffi::c_void; {}] = [",
            self.v_table.len()
        ));
        cg.indent += 1;

        for (method_name, _) in &self.v_table {
            cg.add_indented("Self::__m_");
            cg.add(&method_name.get_rust());
            cg.add(" as *const std::ffi::c_void");
            cg.addln(",");
        }

        cg.indent -= 1;
        cg.add_indentedln("];");
    }

    fn emit_super_v_table(&self, cg: &mut RustCodegen, inherits: &ExprPath) {
        cg.add_indented("pub const __SUPER_V_TABLE: &'static [*const std::ffi::c_void] = &{");
        cg.indent += 1;

        cg.add_indented("let mut table = ");
        cg.add(&inherits.get_rust());
        cg.addln("::__V_TABLE;");

        for (name, is_override) in &self.v_table {
            if is_override.is_some() {
                cg.add_indentedln(&format!(
                    "table[{}::__FN_{}] = {}::__m_{} as *const std::ffi::c_void;",
                    inherits.get_rust(),
                    name.0.to_uppercase(),
                    self.self_path.get_rust(),
                    name.get_rust()
                ));
            }
        }

        cg.add_indentedln("table");

        cg.indent -= 1;
        cg.add_indentedln("};");

        cg.add_indentedln("const fn __test_vt() {");
        cg.indent += 1;

        for method in &self.methods {
            if method.item.is_override.is_some() {
                let mut params = method
                    .item
                    .params
                    .0
                    .clone()
                    .into_iter()
                    .filter_map(|v| v.type_)
                    .collect::<Vec<_>>();

                if params.is_empty() {
                    continue;
                }

                match params.remove(0) {
                    TypeExpr::Ref { mutable, .. } => {
                        cg.add_indented(&inherits.get_rust());
                        cg.add("::__m_");
                        cg.add(&method.item.name.get_rust());
                        cg.add(" as ");

                        params.insert(
                            0,
                            TypeExpr::Ref {
                                lifetime: None,
                                mutable,
                                ty: Box::new(get_type_from_path(inherits)),
                            },
                        );

                        cg.add(
                            &TypeExpr::StaticFn(
                                params,
                                method.item.return_type.clone().map(Box::new),
                            )
                            .get_rust(),
                        );
                        cg.addln(";");
                    }
                    _ => {}
                }
            }
        }

        cg.indent -= 1;
        cg.add_indentedln("}");
    }

    fn emit_constructor(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        let constructor_comment = self.constructor.get_comment();

        cg.add_indentedln("#[allow(invalid_value)]");
        cg.add_indentedln(&constructor_comment);

        (&self.fields, &self.constructor, self.inherits.is_some()).gen_rust(ctx, cg);
    }

    fn emit_methods(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for method in &self.methods {
            match method.item.visibility {
                Visibility::Public => {
                    if method.item.is_override.is_none() {
                        gen_method_point(&method.item, ctx, cg);
                    }

                    let mut prefixed = method.clone();
                    prefixed.item.name.0.insert_str(0, "__m_");
                    prefixed.gen_rust(ctx, cg);
                }
                _ => {
                    method.gen_rust(ctx, cg);
                }
            }
        }
    }

    fn emit_impl_decls(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for item in &self.items {
            if let ClassItem::ImplDecl(impl_) = item {
                let mut impl_ = impl_.clone();
                impl_.item.trait_ = Some(impl_.item.target);
                impl_.item.target = TypeExpr::Path(Path(vec![self.name.clone()]), None);
                impl_.gen_rust(ctx, cg);
            }
        }
    }

    fn emit_deref_impls(&self, cg: &mut RustCodegen) {
        if let Some(ref inherits) = self.inherits {
            cg.add("impl std::ops::Deref for ");
            cg.add(&self.name.get_rust());
            cg.addln(" {");
            cg.indent += 1;

            cg.add_indented("type Target = ");
            cg.add(&inherits.get_rust());
            cg.addln(";");

            cg.add_indentedln("fn deref(&self) -> &Self::Target {&self._super}");

            cg.indent -= 1;
            cg.addln("}");

            cg.add("impl std::ops::DerefMut for ");
            cg.add(&self.name.get_rust());
            cg.addln(" {");
            cg.indent += 1;

            cg.add_indentedln("fn deref_mut(&mut self) -> &mut Self::Target {&mut self._super}");

            cg.indent -= 1;
            cg.addln("}");
        }
    }
}

// ── Public entry point (delegates to the two-stage pipeline) ──────────

pub fn class_decl(
    ctx: &mut Context,
    cg: &mut RustCodegen,
    visibility: &Visibility,
    name: &Identifier,
    generics: &GenericsDecl,
    inherits: &Option<ExprPath>,
    fields: &Vec<Spanned<FieldDeclStmt>>,
    constructor: &Spanned<ClassConstructor>,
    items: &Vec<ClassItem>,
) {
    let data = ClassProcessedData::analyze(
        visibility,
        name,
        generics,
        inherits,
        fields,
        constructor,
        items,
    );
    data.emit(ctx, cg);
}

// ── Constructor code generation (kept as a standalone impl) ───────────

impl GenRust
    for (
        &Vec<Spanned<FieldDeclStmt>>,
        &Spanned<ClassConstructor>,
        bool,
    )
{
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

        cg.addln(") -> Box<Self> {");
        cg.indent += 1;

        cg.add_indentedln("let mut this = Box::new(unsafe { std::mem::MaybeUninit::<Self>::zeroed().assume_init() });");
        cg.add_indentedln("let this_ptr = &mut *this as *mut Self as *mut std::ffi::c_void;");
        cg.add_indentedln("this._m_oop = (&Self::__V_TABLE, this_ptr);");

        for field in self.0 {
            let comment = field.get_comment();

            if let Some(init) = &field.item.init {
                cg.add_indentedln(&comment);

                cg.add_indentedln(&format!("this.{} = ", field.item.decl.name.get_rust()));

                init.gen_rust(ctx, cg);
            }
        }

        cg.add_indented("this.constructor(");

        for (i, param) in params {
            if i > 0 {
                cg.add(", ");
            }

            ctx.expr_ensure_semicolon = false;
            param.name.gen_rust(ctx, cg);
        }

        cg.addln(");");

        if self.2 {
            cg.add_indentedln("this._super._m_oop.0 = &Self::__SUPER_V_TABLE;");
        }

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
                is_override: None,
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

pub fn gen_method_point(method: &FunctionDecl, ctx: &mut Context, cg: &mut RustCodegen) {
    cg.add(&format!(
        "{}fn {}{}(",
        method.visibility.get_rust(),
        method.name.get_rust(),
        method.generics.get_rust(),
    ));

    for (i, param) in method.params.0.iter().enumerate() {
        if i > 0 {
            cg.add(", ");
        }

        param.gen_rust(ctx, cg);
    }

    cg.add(") ");
    if let Some(return_type) = &method.return_type {
        cg.add("-> ");
        cg.add(&return_type.get_rust());
    }

    cg.addln("{");
    cg.indent += 1;

    cg.add_indentedln("unsafe {");
    cg.indent += 1;

    cg.add_indentedln(&format!(
        "let func_ptr = self._m_oop.0[Self::__FN_{}];",
        method.name.0.to_uppercase()
    ));

    cg.add_indented("let func: ");

    let mut param_types: Vec<TypeExpr> = method
        .params
        .clone()
        .0
        .into_iter()
        .filter_map(|v| v.type_)
        .collect();

    param_types.remove(0);

    param_types.insert(
        0,
        TypeExpr::UnsafePtr {
            mutable: true,
            ty: Box::new(TypeExpr::Path(
                Path(vec![
                    Identifier(String::from("std")),
                    Identifier(String::from("ffi")),
                    Identifier(String::from("c_void")),
                ]),
                None,
            )),
        },
    );

    cg.add(&TypeExpr::StaticFn(param_types, method.return_type.clone().map(Box::new)).get_rust());

    cg.addln(" = std::mem::transmute(func_ptr);");

    cg.add_indentedln("func(self._m_oop.1)");

    cg.indent -= 1;
    cg.add_indentedln("}");

    cg.indent -= 1;
    cg.add_indentedln("}");
}

pub fn get_type_from_path(path: &ExprPath) -> TypeExpr {
    TypeExpr::Path(
        Path(path.0.iter().map(|v| v.ident.clone()).collect::<Vec<_>>()),
        path.0.last().unwrap().generics.clone(),
    )
}
