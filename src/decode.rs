//! Step 2: machine code -> `Instruction`. Pure bit fiddling, no ELF needed.
//!
//! Test:  cargo test --lib decode
//!
//! All base RV32I/RV64I instructions are 32 bits wide (we assume NO compressed
//! "C" extension - compile with -march=rv64g, see samples/build.sh).
//!
//! Encoding cheat sheet:
//!
//!   R-type  | funct7 | rs2 | rs1 | funct3 | rd | opcode |
//!   I-type  | imm[11:0]    | rs1 | funct3 | rd | opcode |
//!   S-type  | imm[11:5] | rs2 | rs1 | funct3 | imm[4:0] | opcode |
//!   B-type  | imm[12|10:5] | rs2 | rs1 | funct3 | imm[4:1|11] | opcode |
//!   U-type  | imm[31:12]                   | rd | opcode |
//!   J-type  | imm[20|10:1|11|19:12]        | rd | opcode |
//!
//! Opcodes needed:  0x33 OP (add)   0x13 OP-IMM (addi)   0x03 LOAD (lw)
//!                    0x23 STORE (sw) 0x63 BRANCH (beq/bne) 0x6f JAL
//!                    0x67 JALR       0x37 LUI

use crate::isa::{Instruction, Reg};

pub fn opcode(w: u32) -> u32 {
    w & 0x7f
}

pub fn rd(w: u32) -> Reg {
    ((w >> 7) & 0x1f) as Reg
}

pub fn rs1(w: u32) -> Reg {
    ((w >> 15) & 0x1f) as Reg
}

pub fn rs2(w: u32) -> Reg {
    ((w >> 20) & 0x1f) as Reg
}

pub fn funct3(w: u32) -> u32 {
    (w >> 12) & 0x7
}

pub fn funct7(w: u32) -> u32 {
    (w >> 25) & 0x7f
}

/// I-type: imm[11:0] = w[31:20].
pub fn imm_i(w: u32) -> i32 {
    (w as i32) >> 20
}
/// S-type: imm[11:5] = w[31:25], imm[4:0] = w[11:7].
pub fn imm_s(w: u32) -> i32 {
    let imm11_5 = (w >> 25) & 0x7f;
    let imm4_0 = (w >> 7) & 0x1f;
    let imm = (imm11_5 << 5) | imm4_0;
    ((imm as i32) << 20) >> 20 // sign-extend
}

pub fn imm_b(w: u32) -> i32 {
    let imm12 = (w >> 31) & 1;
    let imm10_5 = (w >> 25) & 0x3f;
    let imm4_1 = (w >> 8) & 0xf;
    let imm11 = (w >> 7) & 1;
    let imm = (imm12 << 12) | (imm10_5 << 5) | (imm4_1 << 1) | (imm11 << 11);
    ((imm as i32) << 19) >> 19
}

pub fn imm_u(w: u32) -> i32 {
    ((w >> 12) & 0xfffff) as i32
}

pub fn imm_j(w: u32) -> i32 {
    let imm20 = (w >> 31) & 1;
    let imm10_1 = (w >> 21) & 0x3ff;
    let imm11 = (w >> 20) & 1;
    let imm19_12 = (w >> 12) & 0xff;
    let imm = (imm20 << 20) | (imm19_12 << 12) | (imm11 << 11) | (imm10_1 << 1);
    ((imm as i32) << 11) >> 11
}

pub fn decode(w: u32) -> Instruction {
    let op = opcode(w);
    match op{
        0x33 => {
            let funct3 = funct3(w);
            let funct7 = funct7(w);
            match (funct3, funct7) {
                (0, 0) => Instruction::Add { rd: rd(w), rs1: rs1(w), rs2: rs2(w) },
                // (0, 0x20) => Instruction::Sub { rd: rd(w), rs1: rs1(w), rs2: rs2(w) }, // we don't have sub yet
                _ => Instruction::Unknown(w),
            }
        }
        0x13 => {
            let funct3 = funct3(w);
            match funct3 {
                0 => Instruction::Addi { rd: rd(w), rs1: rs1(w), imm: imm_i(w) },
                _ => Instruction::Unknown(w),
            }
        }
        0x03 => {
            let funct3 = funct3(w);
            match funct3 {
                2 => Instruction::Lw { rd: rd(w), rs1: rs1(w), imm: imm_i(w) },
                _ => Instruction::Unknown(w),
            }
        }
        0x23 => {
            let funct3 = funct3(w);
            match funct3 {
                2 => Instruction::Sw { rs2: rs2(w), rs1: rs1(w), imm: imm_s(w) },
                _ => Instruction::Unknown(w),
            }
        }
        0x63 => {
            let funct3 = funct3(w);
            match funct3 {
                0 => Instruction::Beq { rs1: rs1(w), rs2: rs2(w), imm: imm_b(w) },
                1 => Instruction::Bne { rs1: rs1(w), rs2: rs2(w), imm: imm_b(w) },
                _ => Instruction::Unknown(w),
            }
        }
        0x6f => Instruction::Jal { rd: rd(w), imm: imm_j(w) },
        0x67 => Instruction::Jalr { rd: rd(w), rs1: rs1(w), imm: imm_i(w) },
        0x37 => Instruction::Lui { rd: rd(w), imm: imm_u(w) },
        _ => Instruction::Unknown(w),
    }
}

/// Decode a byte slice (little-endian) into `(address, instruction)` pairs.
/// `base_addr` is the address of `bytes[0]`. Ignore a trailing partial word.
pub fn decode_all(bytes: &[u8], base_addr: u64) -> Vec<(u64, Instruction)> {
    // Hint: bytes.chunks_exact(4), u32::from_le_bytes, enumerate.
    bytes.chunks_exact(4)
        .enumerate()
        .map(|(i, chunk)| {
            let addr = base_addr + (i as u64) * 4;
            let word = u32::from_le_bytes(chunk.try_into().unwrap());
            (addr, decode(word))
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::Instruction::*;
    use crate::testutil::{words::*, words_to_bytes};

    // -- fields --
    #[test]
    fn fields_of_add() {
        // add a0, a0, a1
        let w = ADD_A0_A0_A1;
        assert_eq!(opcode(w), 0x33);
        assert_eq!(rd(w), 10);
        assert_eq!(rs1(w), 10);
        assert_eq!(rs2(w), 11);
        assert_eq!(funct3(w), 0);
        assert_eq!(funct7(w), 0);
    }

    // -- immediates --
    #[test]
    fn immediates() {
        assert_eq!(imm_i(ADDI_SP_SP_M16), -16);
        assert_eq!(imm_i(LW_A5_M20_S0), -20);
        assert_eq!(imm_s(SW_RA_12_SP), 12);
        assert_eq!(imm_b(BEQ_A0_ZERO_P8), 8);
        assert_eq!(imm_b(BEQ_ZERO_ZERO_M4), -4);
        assert_eq!(imm_u(LUI_A0_12345), 0x12345);
        assert_eq!(imm_j(JAL_RA_P8), 8);
    }

    // -- full decode --
    #[test]
    fn decodes_each_supported_instruction() {
        assert_eq!(decode(ADD_A0_A0_A1), Add { rd: 10, rs1: 10, rs2: 11 });
        assert_eq!(decode(ADDI_SP_SP_M16), Addi { rd: 2, rs1: 2, imm: -16 });
        assert_eq!(decode(LW_A5_M20_S0), Lw { rd: 15, rs1: 8, imm: -20 });
        assert_eq!(decode(SW_RA_12_SP), Sw { rs2: 1, rs1: 2, imm: 12 });
        assert_eq!(decode(BEQ_A0_ZERO_P8), Beq { rs1: 10, rs2: 0, imm: 8 });
        assert_eq!(decode(BEQ_ZERO_ZERO_M4), Beq { rs1: 0, rs2: 0, imm: -4 });
        assert_eq!(decode(JAL_RA_P8), Jal { rd: 1, imm: 8 });
        assert_eq!(decode(RET), Jalr { rd: 0, rs1: 1, imm: 0 });
        assert_eq!(decode(LUI_A0_12345), Lui { rd: 10, imm: 0x12345 });
    }

    #[test]
    fn bne_is_beq_with_funct3_1() {
        // TODO(exercise): add your own test with a hand-encoded `bne a5, a0, -4`.
        // Here, the same trick: take BEQ_A0_ZERO_P8 and set funct3 (bit 12) to 1.
        let w = BEQ_A0_ZERO_P8 | (1 << 12);
        assert_eq!(decode(w), Bne { rs1: 10, rs2: 0, imm: 8 });
    }

    #[test]
    fn unsupported_becomes_unknown_not_panic() {
        // `ecall` = 0x00000073, and an all-zero word is illegal in RISC-V.
        assert_eq!(decode(0x0000_0073), Unknown(0x0000_0073));
        assert_eq!(decode(0), Unknown(0));
        assert_eq!(decode(0xffff_ffff), Unknown(0xffff_ffff));
    }

    // -- decode_all --
    #[test]
    fn decode_all_assigns_addresses() {
        let bytes = words_to_bytes(&[ADDI_SP_SP_M16, SW_RA_12_SP, RET]);
        let out = decode_all(&bytes, 0x1000);
        assert_eq!(
            out,
            vec![
                (0x1000, Addi { rd: 2, rs1: 2, imm: -16 }),
                (0x1004, Sw { rs2: 1, rs1: 2, imm: 12 }),
                (0x1008, Jalr { rd: 0, rs1: 1, imm: 0 }),
            ]
        );
    }

    #[test]
    fn decode_all_ignores_trailing_bytes() {
        let mut bytes = words_to_bytes(&[RET]);
        bytes.extend_from_slice(&[0xaa, 0xbb]);
        assert_eq!(decode_all(&bytes, 0).len(), 1);
    }
}
