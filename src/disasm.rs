use crate::isa::Instruction;

pub fn format(inst: &Instruction, addr: u64) -> String {
    match inst {
        Instruction::Add { rd, rs1, rs2 } => format!("add  {}, {}, {}", crate::isa::reg_name(*rd), crate::isa::reg_name(*rs1), crate::isa::reg_name(*rs2)),
        Instruction::Addi { rd, rs1, imm } => format!("addi {}, {}, {}", crate::isa::reg_name(*rd), crate::isa::reg_name(*rs1), imm),
        Instruction::Lw { rd, rs1, imm } => format!("lw   {}, {}({})", crate::isa::reg_name(*rd), imm, crate::isa::reg_name(*rs1)),
        Instruction::Sw { rs2, rs1, imm } => format!("sw   {}, {}({})", crate::isa::reg_name(*rs2), imm, crate::isa::reg_name(*rs1)),
        Instruction::Beq { rs1, rs2, imm } => format!("beq  {}, {}, 0x{:x}", crate::isa::reg_name(*rs1), crate::isa::reg_name(*rs2), addr.wrapping_add(*imm as i64 as u64)),
        Instruction::Bne { rs1, rs2, imm } => format!("bne  {}, {}, 0x{:x}", crate::isa::reg_name(*rs1), crate::isa::reg_name(*rs2), addr.wrapping_add(*imm as i64 as u64)),
        Instruction::Jal { rd, imm } => format!("jal  {}, 0x{:x}", crate::isa::reg_name(*rd), addr.wrapping_add(*imm as i64 as u64)),
        Instruction::Jalr { rd, rs1, imm } => format!("jalr {}, {}({})", crate::isa::reg_name(*rd), imm, crate::isa::reg_name(*rs1)),
        Instruction::Lui { rd, imm } => format!("lui  {}, 0x{:x}", crate::isa::reg_name(*rd), *imm),
        Instruction::Unknown(w) => format!(".word 0x{:08x}", w),
    }
}

/// Optional pseudo-instruction polish (TODO(stretch), test below is #[ignore]d):
///   addi rd, zero, imm -> "li rd, imm"      jalr zero, 0(ra) -> "ret"
///   jal zero, T -> "j T"                    addi rd, rs, 0   -> "mv rd, rs"
pub fn format_pretty(inst: &Instruction, addr: u64) -> String {
    todo!()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::Instruction::*;

    #[test]
    fn formats_alu_and_memory() {
        assert_eq!(format(&Add { rd: 10, rs1: 10, rs2: 11 }, 0), "add  a0, a0, a1");
        assert_eq!(format(&Addi { rd: 2, rs1: 2, imm: -16 }, 0), "addi sp, sp, -16");
        assert_eq!(format(&Lw { rd: 15, rs1: 8, imm: -20 }, 0), "lw   a5, -20(s0)");
        assert_eq!(format(&Sw { rs2: 1, rs1: 2, imm: 12 }, 0), "sw   ra, 12(sp)");
        assert_eq!(format(&Lui { rd: 10, imm: 0x12345 }, 0), "lui  a0, 0x12345");
    }

    #[test]
    fn formats_control_flow_with_absolute_targets() {
        assert_eq!(format(&Beq { rs1: 10, rs2: 0, imm: 8 }, 0x1008), "beq  a0, zero, 0x1010");
        assert_eq!(format(&Bne { rs1: 15, rs2: 10, imm: -4 }, 0x1008), "bne  a5, a0, 0x1004");
        assert_eq!(format(&Jal { rd: 1, imm: 0x18 }, 0x1008), "jal  ra, 0x1020");
        assert_eq!(format(&Jalr { rd: 0, rs1: 1, imm: 0 }, 0), "jalr zero, 0(ra)");
    }

    #[test]
    fn formats_unknown_as_raw_word() {
        assert_eq!(format(&Unknown(0xdead_beef), 0), ".word 0xdeadbeef");
    }

    #[test]
    #[ignore = "stretch goal: remove #[ignore] when you implement format_pretty"]
    fn pretty_pseudo_instructions() {
        assert_eq!(format_pretty(&Jalr { rd: 0, rs1: 1, imm: 0 }, 0), "ret");
        assert_eq!(format_pretty(&Addi { rd: 10, rs1: 0, imm: 5 }, 0), "li   a0, 5");
        assert_eq!(format_pretty(&Addi { rd: 10, rs1: 11, imm: 0 }, 0), "mv   a0, a1");
    }
}
