
//! Test:  cargo test --lib cfg

use crate::isa::{Flow, Instruction};
use std::collections::{BTreeSet, HashMap, HashSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BasicBlock {
    pub id: usize,
    pub start: u64,
    pub insts: Vec<(u64, Instruction)>,
    pub succs: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Cfg {
    pub name: String,
    pub blocks: Vec<BasicBlock>,
}

pub fn find_leaders(insts: &[(u64, Instruction)]) -> BTreeSet<u64> {
    let mut leaders = BTreeSet::new();
    let addrs: HashSet<u64> = insts.iter().map(|(addr, _)| *addr).collect();
    if let Some((first, _)) = insts.first() {
        leaders.insert(*first);
    }

    for (addr, inst) in insts {
        let next = addr + 4;
        match inst.flow(*addr) {
            Flow::Fallthrough => {}
            Flow::CondBranch { target } | Flow::Jump { target } => {
                if addrs.contains(&target) {
                    leaders.insert(target);
                }
                if addrs.contains(&next) {
                    leaders.insert(next);
                }
            }
            Flow::Indirect => {
                if addrs.contains(&next) {
                    leaders.insert(next);
                }
            }
        }
    }

    leaders
}


pub fn build_cfg(name: &str, insts: &[(u64, Instruction)]) -> Cfg {
    let leaders = find_leaders(insts);
    let mut control_flow_graph = Cfg {
        name: name.to_string(),
        blocks: Vec::new(),
    };
    for(address, instruction) in insts {
        if leaders.contains(address) {
            let block_id = control_flow_graph.blocks.len();
            let mut block_insts = Vec::new();
            block_insts.push((*address, *instruction));
            control_flow_graph.blocks.push(BasicBlock {
                id: block_id,
                start: *address,
                insts: block_insts,
                succs: Vec::new(),
            });
        } else if let Some(last_block) = control_flow_graph.blocks.last_mut() {
            last_block.insts.push((*address, *instruction));
        }
    }

    let start_to_id: HashMap<u64, usize> = control_flow_graph
        .blocks
        .iter()
        .map(|b| (b.start, b.id))
        .collect();
    let block_count = control_flow_graph.blocks.len();

    for block in &mut control_flow_graph.blocks {
        let Some(&(addr, inst)) = block.insts.last() else {
            continue;
        };
        let next = if block.id + 1 < block_count { Some(block.id + 1) } else { None };
        let lookup = |target: u64| start_to_id.get(&target).copied();

        let mut succs = Vec::new();
        match inst.flow(addr) {
            Flow::Fallthrough => succs.extend(next),
            Flow::CondBranch { target } => {
                succs.extend(next);
                succs.extend(lookup(target));
            }
            Flow::Jump { target } => succs.extend(lookup(target)),
            Flow::Indirect => {}
        }
        block.succs = succs;
    }

    control_flow_graph
}

impl Cfg {
    /// TODO(stretch): predecessor lists (preds[i] = ids of blocks that jump to i).
    pub fn predecessors(&self) -> Vec<Vec<usize>> {
        todo!()
    }

    /// TODO(stretch, v2): ids of blocks NOT reachable from BB0 (DFS/BFS over succs).
    /// At -O0 there should be none; can you find any in an -O2 binary?
    pub fn unreachable_blocks(&self) -> Vec<usize> {
        todo!()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::isa::Instruction::*;

    const RA: u8 = 1;
    const A0: u8 = 10;
    const ZERO: u8 = 0;

    fn at(start: u64, insts: &[Instruction]) -> Vec<(u64, Instruction)> {
        insts.iter().enumerate().map(|(i, x)| (start + 4 * i as u64, *x)).collect()
    }
    fn nop() -> Instruction {
        Addi { rd: A0, rs1: A0, imm: 1 }
    }
    fn ret() -> Instruction {
        Jalr { rd: ZERO, rs1: RA, imm: 0 }
    }
    fn succs(cfg: &Cfg) -> Vec<Vec<usize>> {
        cfg.blocks.iter().map(|b| b.succs.clone()).collect()
    }

    /// 0: beq a0,zero,+12 ; 4: addi ; 8: j +8 ; 12: addi ; 16: ret      (if/else diamond)
    fn diamond() -> Vec<(u64, Instruction)> {
        at(0, &[Beq { rs1: A0, rs2: ZERO, imm: 12 }, nop(), Jal { rd: ZERO, imm: 8 }, nop(), ret()])
    }

    /// 0: addi ; 4: addi (loop head) ; 8: bne a0,a0,-4 ; 12: ret       (loop)
    fn looping() -> Vec<(u64, Instruction)> {
        at(0, &[nop(), nop(), Bne { rs1: A0, rs2: A0, imm: -4 }, ret()])
    }

    #[test]
    fn leaders_of_diamond() {
        let leaders: Vec<u64> = find_leaders(&diamond()).into_iter().collect();
        assert_eq!(leaders, [0, 4, 12, 16]);
    }

    #[test]
    fn leaders_of_loop() {
        let leaders: Vec<u64> = find_leaders(&looping()).into_iter().collect();
        assert_eq!(leaders, [0, 4, 12]);
    }

    #[test]
    fn straight_line_code_is_one_block_even_with_a_call() {
        let insts = at(0, &[nop(), Jal { rd: RA, imm: 0x100 }, nop(), ret()]);
        let cfg = build_cfg("f", &insts);
        assert_eq!(cfg.blocks.len(), 1);
        assert_eq!(cfg.blocks[0].insts.len(), 4);
        assert!(cfg.blocks[0].succs.is_empty());
    }

    #[test]
    fn diamond_blocks_and_edges() {
        let cfg = build_cfg("foo", &diamond());
        assert_eq!(cfg.name, "foo");
        let starts: Vec<u64> = cfg.blocks.iter().map(|b| b.start).collect();
        assert_eq!(starts, [0, 4, 12, 16]);
        // BB0 -> BB1 (fallthrough), BB2 (taken) ; BB1 -> BB3 ; BB2 -> BB3 ; BB3 = ret
        assert_eq!(succs(&cfg), vec![vec![1, 2], vec![3], vec![3], vec![]]);
        assert_eq!(cfg.blocks[1].insts.len(), 2);
    }

    #[test]
    fn loop_has_a_back_edge() {
        let cfg = build_cfg("loop", &looping());
        // BB0 -> BB1 ; BB1 -> BB2 (exit, fallthrough), BB1 (back edge) ; BB2 = ret
        assert_eq!(succs(&cfg), vec![vec![1], vec![2, 1], vec![]]);
    }

    #[test]
    fn branch_outside_function_is_ignored() {
        let insts = at(0, &[Beq { rs1: A0, rs2: ZERO, imm: 0x100 }, ret()]);
        let cfg = build_cfg("f", &insts);
        assert_eq!(succs(&cfg), vec![vec![1], vec![]]);
    }

    #[test]
    fn empty_function() {
        assert!(build_cfg("empty", &[]).blocks.is_empty());
    }

    #[test]
    fn block_ids_match_positions() {
        let cfg = build_cfg("foo", &diamond());
        for (i, b) in cfg.blocks.iter().enumerate() {
            assert_eq!(b.id, i);
        }
    }

    #[test]
    #[ignore = "stretch goal"]
    fn predecessors_of_diamond() {
        let cfg = build_cfg("foo", &diamond());
        assert_eq!(cfg.predecessors(), vec![vec![], vec![0], vec![0], vec![1, 2]]);
    }

    #[test]
    #[ignore = "stretch goal"]
    fn code_after_an_unconditional_jump_can_be_unreachable() {
        // 0: j +8 ; 4: addi (dead) ; 8: ret
        let insts = at(0, &[Jal { rd: ZERO, imm: 8 }, nop(), ret()]);
        let cfg = build_cfg("f", &insts);
        assert_eq!(cfg.unreachable_blocks(), vec![1]);
    }
}
