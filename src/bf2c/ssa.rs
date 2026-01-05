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
    versions: HashMap<i32, u32>, // Maps cell->offset
    ptr_offset: i32, // Tracks current poiner offset
}


// SSA Builder Methods: Wrap important IR to SSA IR conversions
impl SsaBuilder {
    // Wrapper around convert_ssa
    fn ssa(&mut self, r: Prog) -> SsaProg {
        self.convert_ssa(r)
    }

    // Returns the current SSA Variable (offset, version) at the cell at cell_offset. Returns 0
    // if there is no ssa variable at the cell_offset return version=0
    fn get_offset_var(&self, cell_offset: i32) -> SsaVar {
        (cell_offset, self.versions.get(&cell_offset).copied().unwrap_or(0))
    }

    // Returns the current pointer offset
    fn current_cell(&self) -> i32 {
        self.ptr_offset
    }

    // Increments version counter for a cell and returns new SSA variable
    fn new_var(&mut self, cell_offset: i32) -> SsaVar{
        let n = self.versions.get(&cell_offset).copied().unwrap_or(0) + 1;
        self.versions.insert(cell_offset, n);
        (cell_offset, n)
    }

    // Returns an SsaProg by processing a program (vector of IR) in Haskell, case by case.
    fn convert_ssa(&mut self, r: Prog) -> SsaProg {
        let mut output: SsaProg = Vec::new();
        for ir in r { 
             match ir {
                Stmt::Add(delta) => {
                    let new = self.new_var(self.ptr_offset);
                    let src = self.get_offset_var(self.ptr_offset);
                    output.push(SsaStmt::Add(new, src, delta));
                }
                Stmt::Move(delta) => {
                    self.ptr_offset += delta;
                    output.push(SsaStmt::Move(delta));
                }
                Stmt::Output(outp) => {
                    let var = self.get_offset_var(self.ptr_offset + outp);
                    output.push(SsaStmt::Output(var));
                }
                Stmt::Input(inp) => {
                    let cell_offset = self.ptr_offset + inp;
                    let new = self.new_var(cell_offset);
                    output.push(SsaStmt::Input(new));
                }
                Stmt::Loop(body) => {
                    todo!();
                }
                Stmt::ZeroLoop => { 
                    let new = self.new_var(self.ptr_offset);
                    output.push(SsaStmt::ZeroLoop(new));
                }
                Stmt::ScanLoop(dir) => {
                    output.push(SsaStmt::ScanLoop(dir));
                }
                Stmt::MultiplicationLoop(decrement, effects) => {
                    todo!()
                }
                Stmt::Set(val) => {
                    let new = self.new_var(self.ptr_offset);
                    output.push(SsaStmt::Set(new, val));
                }
            }
           
        }
        output
    }
}
fn ssa(ir: Prog) -> SsaProg {
    let mut builder = SsaBuilder{ versions: HashMap::new(), ptr_offset: 0 };
    builder.ssa(ir)
}


// [+?]
// v1. v