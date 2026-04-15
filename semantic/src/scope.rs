use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::top_level;

pub enum Symbol {
    Function(top_level::FunctionSymbol),
    TypeSymbol(top_level::TypeSymbol),
    StructSymbol(top_level::StructSymbol),
}

pub struct Scope {
    pub parent: Option<Arc<Scope>>,
    pub variables: Mutex<HashMap<String, Symbol>>,
}

impl Scope {
    pub fn from_top_level() {}
}
