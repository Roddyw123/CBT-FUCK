// https://en.wikipedia.org/wiki/Static_single-assignment_form
use super::ir::{Prog, Stmt};
use std::collections::HashMap;

pub type SsaVar = (i32, u32);

pub type SsaProg = Vec<SsaStmt>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SsaStmt {
    /// Add: dst = src + delta (mod 256)
    Add(SsaVar, SsaVar, i32),
    /// Set: dst = value
    Set(SsaVar, u8),
    /// Copy: dst = src
    Copy(SsaVar, SsaVar),
    /// Move: ptr = ptr + distance
    Move(i32),
    /// Output: print value of variable
    Output(SsaVar),
    /// Input: dst = getchar()
    Input(SsaVar),
    /// Loop: while src != 0 { body }
    Loop(SsaVar, SsaProg, Vec<PhiNode>),
    /// ZeroLoop: dst = 0
    ZeroLoop(SsaVar),
    /// ScanLoop: move pointer in direction until cell is 0
    ScanLoop(i32),
    /// MultiplicationLoop: optimized linear loop
    /// (decrement, control_var, effects: Vec<(dst, src, factor)>)
    MultiplicationLoop(u8, SsaVar, Vec<(SsaVar, SsaVar, i32)>),
    /// Phi: dst = phi(incoming_vars)
    /// Used at loop headers and control flow join points
    Phi(PhiNode),
}

/// Phi node for merging values at control flow join points
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhiNode {
    pub dst: SsaVar,
    pub incoming: Vec<SsaVar>,
}

/// SSA Builder - transforms IR to SSA form
pub struct SsaBuilder {
    /// Current version for each cell offset
    versions: HashMap<i32, u32>,
    /// Current pointer offset
    ptr_offset: i32,
}