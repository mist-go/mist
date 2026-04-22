use std::{
    collections::HashMap,
    sync::{
        Arc,
        atomic::{AtomicUsize, Ordering},
    },
};

use crate::top_level::{
    FunctionSymbol, JSONScope, StructSymbol, TopLevelSymbolScope, TypeSymbol, VarSymbol,
};

#[derive(Clone, Debug)]
pub enum TypeRef {
    Struct(StructRef),
    Function(FunctionRef),
    Package(PackageRef),
    Int,
}

#[derive(Clone, Debug)]
pub struct PackageRef {
    pub name: String,
    pub variables: HashMap<String, Arc<VarRef>>,
}

#[derive(Clone, Debug)]
pub struct VarRef {
    pub export: bool,
    pub var_type: Arc<TypeRef>,
    pub name: String,
}

#[derive(Clone, Debug)]
pub struct FunctionRef {
    pub export: bool,
    pub name: String,
    pub params: HashMap<String, Arc<VarRef>>,
    pub return_type: Option<Arc<TypeRef>>,
}

#[derive(Clone, Debug)]
pub struct StructRef {
    pub export: bool,
    pub name: String,
    pub fields: HashMap<String, Arc<VarRef>>,
}

#[derive(Debug)]
pub struct TopLevelHirScope {
    pub variables: HashMap<String, Arc<VarRef>>,
    pub var_idx: AtomicUsize,
}

impl TopLevelHirScope {
    pub fn from_tlss(tlss: &TopLevelSymbolScope) -> Self {
        let mut scope = Self {
            variables: HashMap::new(),
            var_idx: AtomicUsize::new(0),
        };

        for (_, symbol) in &tlss.functions {
            scope.function_ref(tlss, symbol);
        }

        for (_, symbol) in &tlss.structs {
            scope.struct_ref(tlss, symbol);
        }

        for (_, symbol) in &tlss.imports {
            scope.variables.insert(
                symbol.package_name.clone(),
                Arc::new(VarRef {
                    export: false,
                    name: symbol.package_name.clone(),
                    var_type: Arc::new(TypeRef::Package(PackageRef::from_tlss(symbol))),
                }),
            );
        }

        scope
    }

    pub fn function_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &FunctionSymbol) {
        if self.variables.get(&symbol.name).is_none() {
            let name = self.get_name(symbol.export);

            if let Some(_) = tlss.functions.get(&symbol.name) {
                let rf = FunctionRef {
                    export: symbol.export,
                    name: name.clone(),
                    params: symbol
                        .params
                        .iter()
                        .map(|(name, v)| (name.clone(), self.var_ref(tlss, v)))
                        .collect(),
                    return_type: symbol
                        .return_type
                        .clone()
                        .map(|rt| self.type_ref(tlss, &rt)),
                };

                self.variables.insert(
                    symbol.name.clone(),
                    Arc::new(VarRef {
                        export: false,
                        name: name.clone(),
                        var_type: Arc::new(TypeRef::Function(rf)),
                    }),
                );
            } else {
                unimplemented!()
            }
        }
    }

    pub fn struct_ref(
        &mut self,
        tlss: &TopLevelSymbolScope,
        symbol: &StructSymbol,
    ) -> Arc<TypeRef> {
        if let Some(rf) = self.variables.get(&symbol.name) {
            rf.var_type.clone()
        } else {
            let name = self.get_name(symbol.export);

            let rf = Arc::new(TypeRef::Struct(StructRef {
                export: symbol.export,
                name: name.clone(),
                fields: symbol
                    .fields
                    .iter()
                    .map(|(name, v)| (name.clone(), self.var_ref(tlss, v)))
                    .collect(),
            }));

            self.variables.insert(
                symbol.name.clone(),
                Arc::new(VarRef {
                    export: symbol.export,
                    name: name.clone(),
                    var_type: rf.clone(),
                }),
            );

            rf
        }
    }

    pub fn var_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &VarSymbol) -> Arc<VarRef> {
        Arc::new(VarRef {
            export: symbol.export,
            var_type: self.type_ref(tlss, &symbol.var_type),
            name: self.get_name(symbol.export),
        })
    }

    pub fn type_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &TypeSymbol) -> Arc<TypeRef> {
        if let Some(var_ref) = self.variables.get(&symbol.0) {
            var_ref.var_type.clone()
        } else {
            if let Some(tlss_rf) = tlss.structs.get(&symbol.0) {
                self.struct_ref(tlss, tlss_rf)
            } else {
                match symbol.0.as_str() {
                    "int" => {
                        let var_ref = Arc::new(VarRef {
                            export: false,
                            name: symbol.0.clone(),
                            var_type: Arc::new(TypeRef::Int),
                        });

                        self.variables.insert(symbol.0.clone(), var_ref.clone());

                        var_ref.var_type.clone()
                    }

                    _ => {
                        unimplemented!("{:?}", symbol)
                    }
                }
            }
        }
    }

    pub fn get_reference(&self, name: &String) -> Option<Arc<VarRef>> {
        self.variables.get(name).cloned()
    }

    pub fn next_var_idx(&self) -> usize {
        self.var_idx.fetch_add(1, Ordering::Relaxed)
    }

    pub fn get_name(&self, export: bool) -> String {
        format!("{}{}", if export { 'V' } else { 'v' }, self.next_var_idx())
    }
}

impl TypeRef {
    pub fn get_name(&self) -> String {
        match self {
            TypeRef::Function(f) => f.name.clone(),
            TypeRef::Struct(s) => s.name.clone(),
            TypeRef::Package(p) => p.name.clone(),
            TypeRef::Int => "int".to_string(),
        }
    }
}

impl PackageRef {
    pub fn from_tlss(json_scope: &JSONScope) -> Self {
        let mut scope = Self {
            name: json_scope.package_name.clone(),
            variables: HashMap::new(),
        };

        for (_, symbol) in &json_scope.functions {
            scope.function_ref(json_scope, symbol);
        }

        for (_, symbol) in &json_scope.structs {
            scope.struct_ref(json_scope, symbol);
        }

        scope
    }

    pub fn function_ref(&mut self, json_scope: &JSONScope, symbol: &FunctionSymbol) {
        if self.variables.get(&symbol.name).is_none() {
            let name = symbol.name.clone();

            if let Some(_) = json_scope.functions.get(&symbol.name) {
                let rf = FunctionRef {
                    export: symbol.export,
                    name: name.clone(),
                    params: symbol
                        .params
                        .iter()
                        .map(|(name, v)| (name.clone(), self.var_ref(json_scope, v)))
                        .collect(),
                    return_type: symbol
                        .return_type
                        .clone()
                        .map(|rt| self.type_ref(json_scope, &rt)),
                };

                self.variables.insert(
                    symbol.name.clone(),
                    Arc::new(VarRef {
                        export: false,
                        name: name.clone(),
                        var_type: Arc::new(TypeRef::Function(rf)),
                    }),
                );
            } else {
                unimplemented!()
            }
        }
    }

    pub fn struct_ref(&mut self, json_scope: &JSONScope, symbol: &StructSymbol) -> Arc<TypeRef> {
        if let Some(rf) = self.variables.get(&symbol.name) {
            rf.var_type.clone()
        } else {
            let name = symbol.name.clone();

            let rf = Arc::new(TypeRef::Struct(StructRef {
                export: symbol.export,
                name: name.clone(),
                fields: symbol
                    .fields
                    .iter()
                    .map(|(name, v)| (name.clone(), self.var_ref(json_scope, v)))
                    .collect(),
            }));

            self.variables.insert(
                symbol.name.clone(),
                Arc::new(VarRef {
                    export: symbol.export,
                    name: name.clone(),
                    var_type: rf.clone(),
                }),
            );

            rf
        }
    }

    pub fn var_ref(&mut self, json_scope: &JSONScope, symbol: &VarSymbol) -> Arc<VarRef> {
        Arc::new(VarRef {
            export: symbol.export,
            var_type: self.type_ref(json_scope, &symbol.var_type),
            name: symbol.name.clone(),
        })
    }

    pub fn type_ref(&mut self, json_scope: &JSONScope, symbol: &TypeSymbol) -> Arc<TypeRef> {
        if let Some(var_ref) = self.variables.get(&symbol.0) {
            var_ref.var_type.clone()
        } else {
            if let Some(json_scope_rf) = json_scope.structs.get(&symbol.0) {
                self.struct_ref(json_scope, json_scope_rf)
            } else {
                match symbol.0.as_str() {
                    // TODO: This is a hack, we should have a better way to handle built-in types
                    "int" | "[]any" => {
                        let var_ref = Arc::new(VarRef {
                            export: false,
                            name: symbol.0.clone(),
                            var_type: Arc::new(TypeRef::Int),
                        });

                        self.variables.insert(symbol.0.clone(), var_ref.clone());

                        var_ref.var_type.clone()
                    }

                    _ => {
                        unimplemented!("{:?}", symbol)
                    }
                }
            }
        }
    }

    pub fn get_reference(&self, name: &String) -> Option<Arc<VarRef>> {
        self.variables.get(name).cloned()
    }
}
