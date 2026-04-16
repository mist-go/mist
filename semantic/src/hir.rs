use std::{collections::HashMap, sync::Arc};

use crate::top_level::{FunctionSymbol, StructSymbol, TopLevelSymbolScope, TypeSymbol, VarSymbol};

#[derive(Clone, Debug)]
pub enum TypeRef {
    Struct(StructRef),
    Function(FunctionRef),
    Int,
}

#[derive(Clone, Debug)]
pub struct VarRef {
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
    pub methods: HashMap<String, FunctionRef>,
}

#[derive(Debug)]
pub struct TopLevelHirScope {
    pub variables: HashMap<String, Arc<VarRef>>,
}

impl TopLevelHirScope {
    pub fn from_tlss(tlss: &TopLevelSymbolScope) -> Self {
        let mut scope = Self {
            variables: HashMap::new(),
        };

        for (_, symbol) in &tlss.functions {
            scope.function_ref(tlss, symbol);
        }

        for (_, symbol) in &tlss.structs {
            scope.struct_ref(tlss, symbol);
        }

        scope
    }

    pub fn function_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &FunctionSymbol) {
        if self.variables.get(&symbol.name).is_none() {
            if let Some(_) = tlss.functions.get(&symbol.name) {
                let rf = FunctionRef {
                    export: symbol.export,
                    name: symbol.name.clone(),
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
                        name: symbol.name.clone(),
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
        let rf = Arc::new(TypeRef::Struct(StructRef {
            export: symbol.export,
            name: symbol.name.clone(),
            fields: symbol
                .fields
                .iter()
                .map(|(name, v)| (name.clone(), self.var_ref(tlss, v)))
                .collect(),
            methods: HashMap::new(),
        }));

        self.variables.insert(
            symbol.name.clone(),
            Arc::new(VarRef {
                name: symbol.name.clone(),
                var_type: rf.clone(),
            }),
        );

        rf
    }

    pub fn var_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &VarSymbol) -> Arc<VarRef> {
        Arc::new(VarRef {
            var_type: self.type_ref(tlss, &symbol.var_type),
            name: symbol.name.clone(),
        })
    }

    pub fn type_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &TypeSymbol) -> Arc<TypeRef> {
        if let Some(var_ref) = self.variables.get(&symbol.0) {
            var_ref.var_type.clone()
        } else {
            if let Some(tlss_rf) = tlss.structs.get(&symbol.0) {
                let struct_ref = self.struct_ref(tlss, tlss_rf);

                let var_ref = Arc::new(VarRef {
                    name: symbol.0.clone(),
                    var_type: struct_ref,
                });
                self.variables.insert(symbol.0.clone(), var_ref.clone());

                var_ref.var_type.clone()
            } else {
                match symbol.0.as_str() {
                    "int" => {
                        let var_ref = Arc::new(VarRef {
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
