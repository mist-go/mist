use std::collections::HashMap;

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
    v_table: Vec<Identifier>,
    override_v_table: HashMap<Override, Vec<Identifier>>,
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

        // 1. Gather all raw methods from the class items
        let methods = items
            .iter() // Switched to reference iteration to prevent unnecessary cloning
            .filter_map(|item| match item {
                ClassItem::ImplDecl(_) => None,
                ClassItem::Method(method) => Some(method.clone()),
            })
            .collect::<Vec<Spanned<FunctionDecl>>>();

        let mut v_table = Vec::new();
        let mut override_v_table = std::collections::HashMap::new();

        // 2. Distribute public methods between base table slots and inheritance overrides
        for method in &methods {
            if matches!(method.item.visibility, Visibility::Public) {
                match &method.item.is_override {
                    // It's a brand new virtual method introduced by this class!
                    None => {
                        v_table.push(method.item.name.clone());
                    }
                    // It's an override targeting either the immediate super or a deep ancestor
                    Some(override_spec) => {
                        override_v_table
                            .entry(override_spec.clone())
                            .or_insert_with(Vec::new)
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

        self.emit_super_v_table(cg);
        self.emit_super_v_tests(cg);

        self.emit_constructor(ctx, cg);
        self.emit_methods(ctx, cg);

        cg.indent -= 1;
        cg.addln("}\n");
    }

    fn emit_v_table(&self, cg: &mut RustCodegen) {
        for (i, method_name) in self.v_table.iter().enumerate() {
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

        for method_name in &self.v_table {
            cg.add_indented("Self::__m_");
            cg.add(&method_name.get_rust());
            cg.add(" as *const std::ffi::c_void");
            cg.addln(",");
        }

        cg.indent -= 1;
        cg.add_indentedln("];");
    }

    fn emit_super_v_table(&self, cg: &mut RustCodegen) {
        // If we aren't overriding anything across the tree, we don't need the table array
        if self.override_v_table.is_empty() {
            return;
        }

        // 1. Emit the outer 2D array declaration
        cg.add_indentedln(&format!(
            "pub const __SUPER_V_TABLES: [&'static [*const std::ffi::c_void]; {}] = [",
            self.override_v_table.len()
        ));
        cg.indent += 1;

        // We enumerate over the map so each target class gets a stable outer index
        for (override_tier, overriden_method_idents) in &self.override_v_table {
            // Resolve the target path for this specific sub-table
            let target_path = match &override_tier.0 {
                // Override(Some(path)) / Override::Path(path) -> Targets deep ancestor
                Some(path) => path.clone(),

                // Override(None) / Override::Default -> Targets our immediate parent
                None => {
                    if let Some(parent_path) = &self.inherits {
                        parent_path.clone()
                    } else {
                        continue; // Safeguard if AST has a dangling override without inheritance
                    }
                }
            };

            let target_rust_path = target_path.get_rust();

            // 2. Emit the block for this specific index table
            cg.add_indentedln("&{");
            cg.indent += 1;

            // Initialize this sub-table with the target parent class's base vtable
            cg.add_indentedln(&format!("let mut table = {}::__V_TABLE;", target_rust_path));

            // Patch the slots for every method registered under this specific override tier
            for method_ident in overriden_method_idents {
                cg.add_indentedln(&format!(
                    "table[{}::__FN_{}] = {}::__m_{} as *const std::ffi::c_void;",
                    target_rust_path,
                    method_ident.0.to_uppercase(),
                    self.self_path.get_rust(),
                    method_ident.get_rust()
                ));
            }

            cg.add_indentedln("table");
            cg.indent -= 1;
            cg.add_indentedln("},");
        }

        cg.indent -= 1;
        cg.add_indentedln("];");
    }

    fn emit_super_v_tests(&self, cg: &mut RustCodegen) {
        for (idx, (override_tier, _)) in self.override_v_table.iter().enumerate() {
            // Resolve the target path for this specific sub-table
            let target_path = match &override_tier.0 {
                // Override(Some(path)) / Override::Path(path) -> Targets deep ancestor
                Some(path) => path.clone(),

                // Override(None) / Override::Default -> Targets our immediate parent
                None => {
                    if let Some(parent_path) = &self.inherits {
                        parent_path.clone()
                    } else {
                        continue; // Safeguard if AST has a dangling override without inheritance
                    }
                }
            };

            let target_rust_path = target_path.get_rust();

            // 3. Emit the compile-time signature layout validation test for this specific index
            cg.add_indentedln(&format!("const fn __test_vt_{}() {{", idx));
            cg.indent += 1;

            for method in &self.methods {
                // Only validate the methods that actually belong to the current target slice tier
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

            cg.indent -= 1;
            cg.add_indentedln("}");
        }
    }

    fn emit_constructor(&self, ctx: &mut Context, cg: &mut RustCodegen) {
        let constructor_comment = self.constructor.get_comment();

        cg.add_indentedln("#[allow(invalid_value)]");
        cg.add_indentedln(&constructor_comment);

        cg.add_indented(&format!(
            "{}fn new{}(",
            self.constructor.item.visibility.get_rust(),
            self.constructor.item.generics.get_rust()
        ));

        // Enumerate and build constructor call parameters
        let params = self
            .constructor
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

        // Allocate and zero-initialize state pointer
        cg.add_indentedln("let mut this = Box::new(unsafe { std::mem::MaybeUninit::<Self>::zeroed().assume_init() });");
        cg.add_indentedln("let this_ptr = &mut *this as *mut Self as *mut std::ffi::c_void;");
        cg.add_indentedln("this._m_oop = (&Self::__V_TABLE, this_ptr);");

        // Inline field declarations and initializers
        for field in &self.fields {
            let comment = field.get_comment();

            if let Some(init) = &field.item.init {
                cg.add_indentedln(&comment);
                cg.add_indentedln(&format!("this.{} = ", field.item.decl.name.get_rust()));
                init.gen_rust(ctx, cg);
            }
        }

        // Call Mist internal execution hook
        cg.add_indented("this.constructor(");
        for (i, param) in &params {
            if *i > 0 {
                cg.add(", ");
            }
            ctx.expr_ensure_semicolon = false;
            param.name.gen_rust(ctx, cg);
        }
        cg.addln(");");

        // --- MULTI-INHERITANCE 2D VTABLE ASSIGNMENTS ---
        if self.inherits.is_some() && !self.override_v_table.is_empty() {
            for (idx, (override_tier, _)) in self.override_v_table.iter().enumerate() {
                match &override_tier.0 {
                    // Direct base class layout updates
                    None => {
                        cg.add_indentedln(&format!(
                            "this._super._m_oop.0 = Self::__SUPER_V_TABLES[{}];",
                            idx
                        ));
                    }
                    // Deep ancestor trait table updates
                    Some(path) => {
                        cg.add_indentedln(&format!(
                            "(|v: &mut {}| {{v._m_oop.0 = Self::__SUPER_V_TABLES[{}];}})(&mut this);",
                            path.get_rust(),
                            idx
                        ));
                    }
                }
            }
        }

        cg.add_indentedln("this");
        cg.indent -= 1;
        cg.add_indentedln("}\n");

        // Generate matching inner initialization body block
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

        constructor_params.append(&mut self.constructor.item.params.0.clone());

        Spanned {
            line: self.constructor.line,
            column: self.constructor.column,
            item: FunctionDecl {
                visibility: self.constructor.item.visibility.clone(),
                is_override: None,
                name: Identifier(String::from("constructor")),
                generics: self.constructor.item.generics.clone(),
                params: ParamList(constructor_params),
                return_type: Some(TypeExpr::Tuple(Vec::new())),
                body: Some(self.constructor.item.body.clone()),
            },
        }
        .gen_rust(ctx, cg);
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
