use crate::data_mgmt::{Symbol, SymbolExporter};


#[derive(Debug)]
pub struct NewExecutable {
    pub exports: Vec<Symbol>,
}
impl SymbolExporter for NewExecutable {
    fn read_symbols(&self) -> Result<Vec<Symbol>, crate::data_mgmt::Error> {
        Ok(self.exports.clone())
    }
}
