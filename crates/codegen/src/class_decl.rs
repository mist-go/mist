use std::collections::HashMap;

use mist_parser::ast::*;

use crate::{Context, GenRust, GenSpanTranslation, GetRust, RustCodegen};

pub struct ClassProcessedData {
    visibility: Visibility,
    name: Identifier,
    generics: GenericsDecl,
    inherits: Option<ExprPath>,
    self_path: ExprPath,
    self_ty: TypeExpr,
    fields: Vec<Spanned<FieldDeclStmt>>,
    constructor: Option<Spanned<ClassConstructor>>,
    items: Vec<ClassItem>,
    methods: Vec<Spanned<FunctionDecl>>,
    v_table: Vec<Identifier>,
    override_v_table: HashMap<Override, Spanned<Vec<Identifier>>>,
}

impl ClassProcessedData {
    pub fn analyze(
        visibility: &Visibility,
        name: &Identifier,
        generics: &GenericsDecl,
        inherits: &Option<ExprPath>,
        fields: &Vec<Spanned<FieldDeclStmt>>,
        constructor: &Option<Spanned<ClassConstructor>>,
        items: &Vec<ClassItem>,
    ) -> Self {
        let self_path = ExprPath(vec![ExprPathSegment {
            ident: name.clone(),
            generics: generics.clone().into(),
        }]);

        let self_ty = get_type_from_path(&self_path);

        let methods = items
            .iter()
            .filter_map(|item| match item {
                ClassItem::ImplDecl(_) => None,
                ClassItem::Method(method) => Some(method.clone()),
            })
            .collect::<Vec<Spanned<FunctionDecl>>>();

        let mut v_table = Vec::new();
        let mut override_v_table = std::collections::HashMap::new();

        for method in &methods {
            if matches!(method.item.visibility, Visibility::Public) {
                match &method.item.is_override {
                    None => {
                        if method.item.is_using_self() {
                            v_table.push(method.item.name.clone());
                        }
                    }
                    Some(override_spec) => {
                        override_v_table
                            .entry(override_spec.clone())
                            .or_insert_with(|| Spanned {
                                line: method.line,
                                column: method.column,
                                item: Vec::new(),
                            })
                            .item
                            .push(method.item.name.clone());
                    }
                }
            }
        }

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
            override_v_table,
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

        if let Some(ref inherits) = self.inherits {
            cg.add_indented("pub _super: ");
            cg.add(&get_type_from_path(inherits).get_rust());
            cg.addln(",");
        } else {
            cg.add_indentedln("pub _vptr: &'static [*const std::ffi::c_void],");
        }

        for field in &self.fields {
            field.gen_span(cg);
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

        self.emit_unified_vtable(cg);
        self.emit_super_v_tests(cg);

        if let Some(constructor) = &self.constructor {
            self.emit_constructor(constructor, ctx, cg);
        }
        self.emit_methods(ctx, cg);

        cg.indent -= 1;
        cg.addln("}\n");
    }

    fn emit_unified_vtable(&self, cg: &mut RustCodegen) {
        let has_parent = self.inherits.is_some();
        let parent_path = self
            .inherits
            .as_ref()
            .map(|p| p.get_rust())
            .unwrap_or_default();

        if has_parent {
            cg.add_indentedln(&format!(
                "pub const __PARENT_V_COUNT: usize = {}::__V_COUNT;",
                parent_path
            ));
        } else {
            cg.add_indentedln("pub const __PARENT_V_COUNT: usize = 0;");
        }

        for (i, method_name) in self.v_table.iter().enumerate() {
            cg.add_indentedln(&format!(
                "pub const __FN_{}: usize = Self::__PARENT_V_COUNT + {i};",
                method_name.0.to_uppercase()
            ));
        }

        cg.add_indentedln(&format!(
            "pub const __V_COUNT: usize = Self::__PARENT_V_COUNT + {};",
            self.v_table.len()
        ));

        cg.add_indentedln("pub const __V_TABLE: &'static [*const std::ffi::c_void] = &{");
        cg.indent += 1;

        if has_parent {
            cg.add_indentedln(&format!(
                "let mut table = [std::ptr::null(); {}::__V_COUNT + {}];",
                parent_path,
                self.v_table.len()
            ));

            cg.add_indentedln(&format!("let parent_table = {}::__V_TABLE;", parent_path));
            cg.add_indentedln(&format!(
                "let mut i = 0; while i < {}::__V_COUNT {{ table[i] = parent_table[i]; i += 1; }}",
                parent_path
            ));

            for (override_tier, overriden_method_idents) in &self.override_v_table {
                let base_class_path = override_tier
                    .0
                    .as_ref()
                    .unwrap_or(self.inherits.as_ref().unwrap())
                    .get_rust();
                for method_ident in &overriden_method_idents.item {
                    cg.add_indentedln(&format!(
                        "table[{}::__FN_{}] = {}::__m_{} as *const std::ffi::c_void;",
                        base_class_path,
                        method_ident.0.to_uppercase(),
                        self.self_path.get_rust(),
                        method_ident.get_rust()
                    ));
                }
            }

            for method_name in &self.v_table {
                cg.add_indentedln(&format!(
                    "table[Self::__FN_{}] = Self::__m_{} as *const std::ffi::c_void;",
                    method_name.0.to_uppercase(),
                    method_name.get_rust()
                ));
            }

            cg.add_indentedln("table");
        } else {
            cg.add_indentedln("[");
            cg.indent += 1;
            for method_name in &self.v_table {
                cg.add_indentedln(&format!(
                    "Self::__m_{} as *const std::ffi::c_void,",
                    method_name.get_rust()
                ));
            }
            cg.indent -= 1;
            cg.add_indentedln("]");
        }

        cg.indent -= 1;
        cg.add_indentedln("};");
    }

    fn emit_super_v_tests(&self, cg: &mut RustCodegen) {
        cg.add_indentedln("#[allow(invalid_value)]");
        cg.add_indentedln(&format!("fn __test_vt() {{"));
        cg.indent += 1;

        for (override_tier, _) in self.override_v_table.iter() {
            let target_path = match &override_tier.0 {
                Some(path) => path.clone(),

                None => {
                    if let Some(parent_path) = &self.inherits {
                        parent_path.clone()
                    } else {
                        continue;
                    }
                }
            };

            let target_rust_path = target_path.get_rust();

            for method in &self.methods {
                if method.item.is_override.as_ref() == Some(override_tier) {
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

                    if let TypeExpr::Ref { mutable, .. } = params.remove(0) {
                        method.gen_span(cg);
                        cg.add_indented(&format!("{}::__m_", target_rust_path));
                        cg.add(&method.item.name.get_rust());
                        cg.add(" as ");

                        params.insert(
                            0,
                            TypeExpr::Ref {
                                lifetime: None,
                                mutable,
                                ty: Box::new(get_type_from_path(&target_path)),
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
                }
            }
        }

        // Deref tests for override targets
        if self.inherits.is_some() && !self.override_v_table.is_empty() {
            cg.add_indentedln("let this: &Self = &unsafe { std::mem::MaybeUninit::<Self>::zeroed().assume_init() };");

            for (override_tier, v) in &self.override_v_table {
                if let Some(path) = &override_tier.0 {
                    v.gen_span(cg);
                    // This forces the compiler to statically verify that &Self can Deref into &Target
                    cg.add_indentedln(&format!("let _: &{} = this;", path.get_rust()));
                }
            }
        }

        cg.indent -= 1;
        cg.add_indentedln("}");
    }

    fn emit_constructor(
        &self,
        constructor: &Spanned<ClassConstructor>,
        ctx: &mut Context,
        cg: &mut RustCodegen,
    ) {
        cg.add_indentedln("#[allow(invalid_value)]");
        constructor.gen_span(cg);

        cg.add_indented(&format!(
            "{}fn new{}(",
            constructor.item.visibility.get_rust(),
            constructor.item.generics.get_rust()
        ));

        let params = constructor
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
        cg.add_indentedln("this._vptr = &Self::__V_TABLE;");

        for field in &self.fields {
            if let Some(init) = &field.item.init {
                field.gen_span(cg);
                cg.add_indentedln(&format!("this.{} = ", field.item.decl.name.get_rust()));
                init.gen_rust(ctx, cg);
            }
        }

        cg.add_indented("this.constructor(");
        for (i, param) in &params {
            if *i > 0 {
                cg.add(", ");
            }
            ctx.expr_ensure_semicolon = false;
            param.name.gen_rust(ctx, cg);
        }
        cg.addln(");");

        cg.add_indentedln("this._vptr = &Self::__V_TABLE;");

        constructor.gen_span(cg);

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

        constructor_params.append(&mut constructor.item.params.0.clone());

        Spanned {
            line: constructor.line,
            column: constructor.column,
            item: FunctionDecl {
                visibility: constructor.item.visibility.clone(),
                is_override: None,
                name: Identifier(String::from("constructor")),
                generics: constructor.item.generics.clone(),
                params: ParamList(constructor_params),
                return_type: Some(TypeExpr::Tuple(Vec::new())),
                body: Some(constructor.item.body.clone()),
            },
        }
        .gen_rust(ctx, cg);
    }

    fn emit_methods(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        for method in &self.methods {
            match method.item.visibility {
                Visibility::Public => {
                    if method.item.is_using_self() {
                        if method.item.is_override.is_none() {
                            gen_method_point(&method.item, ctx, cg);
                        }

                        let mut prefixed = method.clone();
                        prefixed.item.name.0.insert_str(0, "__m_");
                        prefixed.gen_rust(ctx, cg);
                    } else {
                        method.gen_rust(ctx, cg);
                    }
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
            let generics_str = self.generics.get_rust();
            let generics_expr_str = Generics::from(self.generics.clone()).get_rust();

            cg.add(&format!(
                "impl{} std::ops::Deref for {}{}",
                generics_str,
                self.name.get_rust(),
                generics_expr_str
            ));

            cg.addln(" {");
            cg.indent += 1;

            cg.add_indented("type Target = ");
            cg.add(&inherits.get_rust());
            cg.addln(";");

            cg.add_indentedln("fn deref(&self) -> &Self::Target { &self._super }");

            cg.indent -= 1;
            cg.addln("}");

            cg.add(&format!(
                "impl{} std::ops::DerefMut for {}{}",
                generics_str,
                self.name.get_rust(),
                generics_expr_str
            ));

            cg.addln(" {");
            cg.indent += 1;

            cg.add_indentedln("fn deref_mut(&mut self) -> &mut Self::Target { &mut self._super }");

            cg.indent -= 1;
            cg.addln("}");
        }
    }
}

pub fn class_decl(
    ctx: &mut Context,
    cg: &mut RustCodegen,
    visibility: &Visibility,
    name: &Identifier,
    generics: &GenericsDecl,
    inherits: &Option<ExprPath>,
    fields: &Vec<Spanned<FieldDeclStmt>>,
    constructor: &Option<Spanned<ClassConstructor>>,
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

fn construct_pattern(pat: &Pattern, idx: usize) -> Pattern {
    match pat {
        Pattern::Literal(v) => Pattern::Literal(v.clone()),
        Pattern::Path(is_mut, v) => Pattern::Path(*is_mut, v.clone().into()),
        _ => Pattern::Path(false, Path(vec![Identifier(format!("_{idx}"))])),
    }
}

pub fn gen_method_point(method: &FunctionDecl, ctx: &mut Context, cg: &mut RustCodegen) {
    cg.add_indented(&format!(
        "{}fn {}{}(",
        method.visibility.get_rust(),
        method.name.get_rust(),
        method.generics.get_rust(),
    ));

    let params = method
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
        "let func_ptr = self._vptr[Self::__FN_{}];",
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

    let TypeExpr::Ref { mutable, .. } = param_types.remove(0) else {
        panic!("")
    };

    param_types.insert(
        0,
        TypeExpr::UnsafePtr {
            mutable,
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

    if mutable {
        cg.add_indented("func(self as *mut Self as *const std::ffi::c_void");
    } else {
        cg.add_indented("func(self as *const Self as *const std::ffi::c_void");
    }

    for (i, param) in &params {
        if *i == 0 {
            continue; // self already fulfills it
        }
        cg.add(", ");
        ctx.expr_ensure_semicolon = false;
        param.name.gen_rust(ctx, cg);
    }

    cg.addln(")");

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
