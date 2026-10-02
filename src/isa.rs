//! Test:  cargo test --lib isa
//!
//! RISC-V reference: "RISC-V Unprivileged ISA", chapter "RV32I Base Integer ISA".
//! Register ABI names: the "RISC-V ELF psABI / calling convention" document.

pub type Reg = u8;

/// The small subset we decode. Everything else becomes `Unknown(raw_word)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Instruction {
    Add { rd: Reg, rs1: Reg, rs2: Reg },
    Addi { rd: Reg, rs1: Reg, imm: i32 },
    Lw { rd: Reg, rs1: Reg, imm: i32 },
    Sw { rs2: Reg, rs1: Reg, imm: i32 },
    Beq { rs1: Reg, rs2: Reg, imm: i32 },
    Bne { rs1: Reg, rs2: Reg, imm: i32 },
    Jal { rd: Reg, imm: i32 },
    Jalr { rd: Reg, rs1: Reg, imm: i32 },
    Lui { rd: Reg, imm: i32 },
    Unknown(u32),
    // TODO(stretch): real gcc output for abs_sum also needs blt/bge, sub, slli,
    // addw/addiw, ld/sd (RV64)... Add them here + in decode.rs + disasm.rs
    // once the basic ~10 work. Blt/Bge/Bltu/Bgeu MUST be added for a correct CFG.
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    Fallthrough,
    CondBranch { target: u64 },
    Jump { target: u64 },
    Indirect,
}

pub fn reg_name(r: Reg) -> &'static str {
    match r {
        0 => "zero",
        1 => "ra",
        2 => "sp",
        3 => "gp",
        4 => "tp",
        5 => "t0",
        6 => "t1",
        7 => "t2",
        8 => "s0",
        9 => "s1",
        10 => "a0",
        11 => "a1",
        12 => "a2",
        13 => "a3",
        14 => "a4",
        15 => "a5",
        16 => "a6",
        17 => "a7",
        18 => "s2",
        19 => "s3",
        20 => "s4",
        21 => "s5",
        22 => "s6",
        23 => "s7",
        24 => "s8",
        25 => "s9",
        26 => "s10",
        27 => "s11",
        28 => "t3",
        29 => "t4",
        30 => "t5",
        31 => "t6",
        _ => "invalid",
    }

}

impl Instruction {
    pub fn flow(&self, addr: u64) -> Flow {
        match self {
            Instruction::Beq {imm,..} | Instruction:: Bne {imm, ..} =>
                Flow::CondBranch {target: addr.wrapping_add(*imm as i64 as u64)},
            Instruction::Jal {rd, imm} =>
                if *rd ==0 {
                    Flow::Jump {target: addr.wrapping_add(*imm as i64 as u64)}
                } else {
                    Flow::Fallthrough
                }
            Instruction::Jalr {rd, ..} =>
                if *rd == 0 {
                    Flow::Indirect
                } else {
                    Flow::Fallthrough
                }
            _ => Flow::Fallthrough,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn register_names() {
        assert_eq!(reg_name(0), "zero");
        assert_eq!(reg_name(1), "ra");
        assert_eq!(reg_name(2), "sp");
        assert_eq!(reg_name(8), "s0");
        assert_eq!(reg_name(10), "a0");
        assert_eq!(reg_name(15), "a5");
        assert_eq!(reg_name(17), "a7");
        assert_eq!(reg_name(18), "s2");
        assert_eq!(reg_name(31), "t6");
    }

    #[test]
    fn plain_instruction_falls_through() {
        let i = Instruction::Addi { rd: 2, rs1: 2, imm: -16 };
        assert_eq!(i.flow(0x100), Flow::Fallthrough);
    }

    #[test]
    fn branches_have_absolute_targets() {
        let fwd = Instruction::Beq { rs1: 10, rs2: 0, imm: 8 };
        assert_eq!(fwd.flow(0x100), Flow::CondBranch { target: 0x108 });
        let back = Instruction::Bne { rs1: 15, rs2: 10, imm: -4 };
        assert_eq!(back.flow(0x100), Flow::CondBranch { target: 0xfc });
    }

    #[test]
    fn jal_zero_is_jump_but_jal_ra_is_call() {
        assert_eq!(Instruction::Jal { rd: 0, imm: 16 }.flow(0x100), Flow::Jump { target: 0x110 });
        assert_eq!(Instruction::Jal { rd: 1, imm: 16 }.flow(0x100), Flow::Fallthrough);
    }

    #[test]
    fn jalr_zero_is_indirect_but_jalr_ra_is_call() {
        assert_eq!(Instruction::Jalr { rd: 0, rs1: 1, imm: 0 }.flow(0), Flow::Indirect); // ret
        assert_eq!(Instruction::Jalr { rd: 1, rs1: 5, imm: 0 }.flow(0), Flow::Fallthrough);
    }
}
