// https://en.wikipedia.org/wiki/Static_single-assignment_form
use super::ir::{Prog, Stmt};
use std::collections::{HashMap, HashSet};

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
    pub dst: SsaVar, // The variable being assigned to
    pub incoming: Vec<SsaVar>, // Variables (versions)
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
                    let control_cell = self.current_cell();
                    let ptr_before = self.ptr_offset;

                    // Find all variables modified by the loop
                    let mutated_variables = self.analyze(&body, ptr_before);

                    // Create phi nodes at loop entry for all mutated variables
                    // Each phi merges the value before the loop with the value after loop body
                    let mut phi_nodes = Vec::new();

                    for &cell_offset in &mutated_variables {
                        let var_before = self.get_offset_var(cell_offset);

                        // Create new version for the phi node destination
                        let phi_dst = self.new_var(cell_offset);

                        // Placeholder phi node - we'll update incoming values later
                        phi_nodes.push(PhiNode {
                            dst: phi_dst,
                            incoming: vec![var_before], // Will add after-body version later
                        });
                    }

                    // Convert the loop body with phi destinations as current versions
                    let ssa_body = self.convert_ssa(body);

                    // Now collect the versions after the loop body for phi nodes
                    for phi in phi_nodes.iter_mut() {
                        let cell_offset = phi.dst.0;
                        let var_after = self.get_offset_var(cell_offset);
                        phi.incoming.push(var_after);
                    }

                    // The control variable for the loop condition
                    let control_var = if mutated_variables.contains(&control_cell) {
                        // If control cell is modified, use its phi node
                        phi_nodes.iter()
                            .find(|p| p.dst.0 == control_cell)
                            .map(|p| p.dst)
                            .unwrap_or_else(|| self.get_offset_var(control_cell))
                    } else {
                        self.get_offset_var(control_cell)
                    };

                    // Restore pointer offset (loop body might have moved it)
                    // self.ptr_offset = ptr_before;

                    output.push(SsaStmt::Loop(control_var, ssa_body, phi_nodes));
                }
                Stmt::ZeroLoop => { 
                    let new = self.new_var(self.ptr_offset);
                    output.push(SsaStmt::ZeroLoop(new));
                }
                Stmt::ScanLoop(dir) => {
                    output.push(SsaStmt::ScanLoop(dir));
                }
                Stmt::MultiplicationLoop(decrement, effects) => {
                    let control_cell = self.current_cell();
                    let control_var = self.get_offset_var(control_cell);

                    let mut ssa_effects = Vec::new();
                    for (cell_offset, factor) in effects {
                        let src = self.get_offset_var(self.ptr_offset + cell_offset);
                        let dst = self.new_var(self.ptr_offset + cell_offset);
                        ssa_effects.push((dst, src, factor));
                    }

                    // The control cell is also modified (decremented to 0)
                    // We need to create a new version for it
                    let _new_control = self.new_var(control_cell);

                    output.push(SsaStmt::MultiplicationLoop(decrement, control_var, ssa_effects));
                }
                Stmt::Set(val) => {
                    let new = self.new_var(self.ptr_offset);
                    output.push(SsaStmt::Set(new, val));
                }
            }
           
        }
        output
    }

    // Takes a program and offset. Returns a set containing all variables/offsets that have been
    // modified by the program. Handles inner loops via recursion.
    fn analyze(&mut self, prog: &Prog, start_offset: i32) -> HashSet<i32> {
        let mut modified: HashSet<i32> = HashSet::new();
        let mut offset = start_offset;
        for stmt in prog {
            match stmt {
                Stmt::Add(_) => {
                    modified.insert(offset);
                }
                Stmt::Move(delta) => {
                    offset = offset + delta;
                }
                Stmt::Output(_) => {
                    // Output doesn't modify anything
                }
                Stmt::Input(inp) => {
                    modified.insert(offset + inp);
                }
                Stmt::Loop(body) => {
                    let inner_loop_modified = self.analyze(body, offset);
                    modified.extend(inner_loop_modified);
                }
                Stmt::ZeroLoop => {
                    modified.insert(offset);
                }
                Stmt::ScanLoop(_) => {
                    // ScanLoop doesn't modify the current cell, just moves pointer
                }
                Stmt::MultiplicationLoop(_, effects) => {
                    // MultiplicationLoop modifies the control cell and effect cells
                    modified.insert(offset);
                    for (cell_offset, _) in effects {
                        modified.insert(offset + cell_offset);
                    }
                }
                Stmt::Set(_) => {
                    modified.insert(offset);
                }
            }
        }
        modified
    }

}
fn ssa(ir: Prog) -> SsaProg {
    let mut builder = SsaBuilder{ versions: HashMap::new(), ptr_offset: 0 };
    builder.ssa(ir)
}


// [+?]
// v1. v