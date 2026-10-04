use object::write::{Object, Symbol, SymbolSection};
use object::{Architecture, BinaryFormat, Endianness, SectionKind, SymbolFlags, SymbolKind, SymbolScope};

/// Encode 32-bit instruction words as little-endian bytes.
pub fn words_to_bytes(words: &[u32]) -> Vec<u8> {
    words.iter().flat_map(|w| w.to_le_bytes()).collect()
}

/// Build an ELF64 RISC-V file. `funcs` is `(name, first_word_index, word_count)`.
pub fn build_elf(words: &[u32], funcs: &[(&str, usize, usize)]) -> Vec<u8> {
    let mut obj = Object::new(BinaryFormat::Elf, Architecture::Riscv64, Endianness::Little);
    let text = obj.add_section(vec![], b".text".to_vec(), SectionKind::Text);
    obj.append_section_data(text, &words_to_bytes(words), 4);
    for (name, first, count) in funcs {
        obj.add_symbol(Symbol {
            name: name.as_bytes().to_vec(),
            value: (*first * 4) as u64,
            size: (*count * 4) as u64,
            kind: SymbolKind::Text,
            scope: SymbolScope::Linkage,
            weak: false,
            section: SymbolSection::Section(text),
            flags: SymbolFlags::None,
        });
    }
    obj.write().expect("failed to build test ELF")
}

/// Hand-assembled words used across tests (verified encodings).
pub mod words {
    pub const ADDI_SP_SP_M16: u32 = 0xff01_0113; // addi sp, sp, -16
    pub const SW_RA_12_SP: u32 = 0x0011_2623; // sw   ra, 12(sp)
    pub const LW_A5_M20_S0: u32 = 0xfec4_2783; // lw   a5, -20(s0)
    pub const ADD_A0_A0_A1: u32 = 0x00b5_0533; // add  a0, a0, a1
    pub const LUI_A0_12345: u32 = 0x1234_5537; // lui  a0, 0x12345
    pub const BEQ_A0_ZERO_P8: u32 = 0x0005_0463; // beq  a0, zero, +8
    pub const BEQ_ZERO_ZERO_M4: u32 = 0xfe00_0ee3; // beq  zero, zero, -4
    pub const JAL_RA_P8: u32 = 0x0080_00ef; // jal  ra, +8
    pub const RET: u32 = 0x0000_8067; // jalr zero, 0(ra)
}

#[cfg(test)]
mod tests {
    use super::*;
    use object::{Object as _, ObjectSection};

    #[test]
    fn build_elf_roundtrips() {
        let bytes = build_elf(&[words::RET], &[("f", 0, 1)]);
        let file = object::File::parse(&*bytes).unwrap();
        assert_eq!(file.section_by_name(".text").unwrap().data().unwrap(), &words::RET.to_le_bytes());
    }
}
