use std::{collections::HashMap, sync::Arc};

use crate::top_level::{FunctionSymbol, StructSymbol, TopLevelSymbolScope, TypeSymbol, VarSymbol};

#[derive(Clone, Debug)]
pub enum TypeRef {
    Struct(StructRef),
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

#[derive(Clone, Debug)]
pub struct TopLevelHirScope {
    pub types: HashMap<String, Arc<TypeRef>>,
    pub functions: HashMap<String, Arc<FunctionRef>>,
}

impl TopLevelHirScope {
    pub fn from_tlss(tlss: &TopLevelSymbolScope) -> Self {
        let mut scope = Self {
            types: HashMap::new(),
            functions: HashMap::new(),
        };

        for (_, symbol) in &tlss.functions {
            scope.function_ref(tlss, symbol);
        }

        for (_, symbol) in &tlss.structs {
            scope.struct_ref(tlss, symbol);
        }

        scope
    }

    pub fn function_ref(
        &mut self,
        tlss: &TopLevelSymbolScope,
        symbol: &FunctionSymbol,
    ) -> Arc<FunctionRef> {
        if let Some(rf) = self.functions.get(&symbol.name) {
            rf.clone()
        } else {
            if let Some(tlss_rf) = tlss.functions.get(&symbol.name) {
                let rf = Arc::new(FunctionRef {
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
                });

                self.functions.insert(symbol.name.clone(), rf.clone());

                rf
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

        self.types.insert(symbol.name.clone(), rf.clone());

        rf
    }

    pub fn var_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &VarSymbol) -> Arc<VarRef> {
        Arc::new(VarRef {
            var_type: self.type_ref(tlss, &symbol.var_type),
            name: symbol.name.clone(),
        })
    }

    pub fn type_ref(&mut self, tlss: &TopLevelSymbolScope, symbol: &TypeSymbol) -> Arc<TypeRef> {
        if let Some(rf) = self.types.get(&symbol.0) {
            rf.clone()
        } else {
            if let Some(tlss_rf) = tlss.structs.get(&symbol.0) {
                let rf = Arc::new(TypeRef::Struct(StructRef {
                    export: tlss_rf.export,
                    name: tlss_rf.name.clone(),
                    fields: tlss_rf
                        .fields
                        .iter()
                        .map(|(name, v)| {
                            (
                                name.clone(),
                                Arc::new(VarRef {
                                    name: name.clone(),
                                    var_type: self.type_ref(tlss, &v.var_type),
                                }),
                            )
                        })
                        .collect(),
                    methods: HashMap::new(),
                }));

                self.types.insert(symbol.0.clone(), rf.clone());

                rf
            } else {
                unimplemented!()
            }
        }
    }
}
