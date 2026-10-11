//! Test:  cargo test --lib elf
//!
//! Tests build their ELF in memory (see testutil.rs) - no cross-compiler needed.
//! To try a REAL binary : run samples/build.sh, then `cargo run -- samples/out/abs_sum_O0`.

use anyhow::Result;
use object::{Object, ObjectSection, ObjectSymbol, SymbolKind, Architecture};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Function {
    pub name: String,
    pub addr: u64,
    pub size: u64,
}

#[derive(Debug, Clone)]
pub struct ElfInfo {
    pub is_64: bool,
    pub text_addr: u64,
    pub text: Vec<u8>,
    pub functions: Vec<Function>,
}

pub fn parse(bytes: &[u8]) -> Result<ElfInfo> {
    let elf_file = object::File::parse(bytes)?;
    let architecture = elf_file.architecture();
    let is_64 = matches!(architecture, Architecture::Riscv64 | Architecture::Riscv32);
    let text_addr = elf_file.section_by_name(".text").map(|s| s.address()).unwrap_or(0);
    let text = elf_file.section_by_name(".text").map(|s| s.data().unwrap_or(&[]).to_vec()).unwrap_or_default();
    let mut functions: Vec<Function> = elf_file.symbols()
        .filter(|s| s.kind() == SymbolKind::Text && !s.name().unwrap_or("").is_empty())
        .map(|s| Function {
            name: s.name().unwrap_or("").to_string(),
            addr: s.address(),
            size: s.size(),
        })
        .collect();
    functions.sort_by_key(|f| f.addr);
    Ok(ElfInfo { is_64, text_addr, text, functions })

}

impl ElfInfo {
    pub fn find_function(&self, name: &str) -> Option<&Function> {
        self.functions.iter().find(|f| f.name == name)

    }

    pub fn function_bytes(&self, f: &Function) -> Result<&[u8]> {
        let offset = f.addr.checked_sub(self.text_addr).ok_or_else(|| anyhow::anyhow!("Function address is before .text section"))?;
        let end = offset.checked_add(f.size).ok_or_else(|| anyhow::anyhow!("Function size causes overflow"))?;
        if end as usize > self.text.len() {
            return Err(anyhow::anyhow!("Function lies outside .text section"));
        }
        Ok(&self.text[offset as usize..end as usize])

    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testutil::{build_elf, words::*, words_to_bytes};

    fn sample() -> Vec<u8> {
        build_elf(
            &[ADDI_SP_SP_M16, SW_RA_12_SP, RET, ADD_A0_A0_A1, RET],
            &[("foo", 0, 3), ("bar", 3, 2)],
        )
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse(b"this is not an ELF file").is_err());
        assert!(parse(&[]).is_err());
    }

    #[test]
    fn finds_text_section() {
        let info = parse(&sample()).unwrap();
        assert!(info.is_64);
        assert_eq!(info.text, words_to_bytes(&[ADDI_SP_SP_M16, SW_RA_12_SP, RET, ADD_A0_A0_A1, RET]));
    }

    #[test]
    fn finds_function_symbols_sorted_by_address() {
        let info = parse(&sample()).unwrap();
        let names: Vec<_> = info.functions.iter().map(|f| f.name.as_str()).collect();
        assert_eq!(names, ["foo", "bar"]);
        assert_eq!(info.functions[1].size, 8);
    }

    #[test]
    fn find_function_by_name() {
        let info = parse(&sample()).unwrap();
        assert_eq!(info.find_function("bar").unwrap().size, 8);
        assert!(info.find_function("nope").is_none());
    }

    #[test]
    fn function_bytes_slices_text() {
        let info = parse(&sample()).unwrap();
        let bar = info.find_function("bar").unwrap().clone();
        assert_eq!(info.function_bytes(&bar).unwrap(), words_to_bytes(&[ADD_A0_A0_A1, RET]));
    }

    #[test]
    fn function_outside_text_is_an_error() {
        let info = parse(&sample()).unwrap();
        let bogus = Function { name: "bogus".into(), addr: 0x9999, size: 4 };
        assert!(info.function_bytes(&bogus).is_err());
    }
}
