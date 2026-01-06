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
                    let src = self.get_offset_var(self.ptr_offset);  // Get old version FIRST
                    let new = self.new_var(self.ptr_offset);          // Then create new version
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
                    let net_movement = self.calculate_net_movement(&body);
                    let mutated_variables = self.analyze(&body, self.ptr_offset);
                    // if no net pointer movement, keep all versions
                    // otherwise discard all versions
                    if net_movement != 0 {
                        // new_var will overwrite the rest
                        let tmp: Vec<_> = self.versions.keys().cloned().collect();
                        tmp.into_iter().filter(|offset| !mutated_variables.contains(offset))
                            .for_each(|offset|{
                                self.new_var(offset);
                            });
                    }
                    
                    
                    // Create phi nodes at loop entry for all mutated variables
                    // Each phi merges the value before the loop with the value after loop body
                    let mut phi_nodes = Vec::new();
                    let mut phi_dsts = HashMap::new();  // Track phi destinations

                    for &cell_offset in &mutated_variables {
                        let var_before = self.get_offset_var(cell_offset);

                        // Create new version for the phi node destination
                        let phi_dst = self.new_var(cell_offset);
                        phi_dsts.insert(cell_offset, phi_dst);

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
                        let var_after = self.get_offset_var(cell_offset+net_movement);
                        phi.incoming.push(var_after);
                    }

                    // The control variable for the loop condition
                    let control_var = self.get_offset_var(net_movement);

                    // Restore pointer offset - reset if imbalanced loop
                    self.ptr_offset = if net_movement == 0 {
                        self.ptr_offset
                    } else {
                        0
                    };
                    let mut true_body = phi_nodes.into_iter().map(|phi| SsaStmt::Phi(phi)).collect::<Vec<_>>();
                    true_body.extend(ssa_body);
                    output.push(SsaStmt::Loop(control_var, true_body, Vec::new()));
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

    // Calculates the net pointer movement of a program.
    // Returns 0 for balanced programs (pointer returns to starting position).
    // Panics if nested loops are unbalanced.
    fn calculate_net_movement(&self, prog: &Prog) -> i32 {
        let mut net_movement = 0;
        for stmt in prog {
            match stmt {
                Stmt::Move(delta) => {
                    net_movement += delta;
                }
                Stmt::Loop(body) => {
                    let inner_net = self.calculate_net_movement(body);
                    if inner_net != 0 {
                        panic!(
                            "Nested loop has unbalanced pointer movement (net movement: {}). \
                            Only balanced loops (net movement = 0) are supported in SSA.",
                            inner_net
                        );
                    }
                }
                _ => {}
            }
        }
        net_movement
    }

}

pub fn ssa(ir: Prog) -> SsaProg {
    let mut builder = SsaBuilder{ versions: HashMap::new(), ptr_offset: 0 };
    builder.ssa(ir)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::bf2c::ir::Stmt;

    #[test]
    fn test_simple_add() {
        // Test: single Add instruction
        let ir = vec![Stmt::Add(5)];
        let ssa = ssa(ir);

        println!("Simple Add SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 1);
        match &ssa[0] {
            SsaStmt::Add(dst, src, delta) => {
                assert_eq!(dst, &(0, 1));  // First version of cell 0
                assert_eq!(src, &(0, 0));  // Initial version (0)
                assert_eq!(delta, &5);
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_set() {
        // Test: Set instruction
        let ir = vec![Stmt::Set(42)];
        let ssa = ssa(ir);

        println!("Set SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 1);
        match &ssa[0] {
            SsaStmt::Set(dst, val) => {
                assert_eq!(dst, &(0, 1));
                assert_eq!(val, &42);
            }
            _ => panic!("Expected Set"),
        }
    }

    #[test]
    fn test_move_and_add() {
        // Test: Move then Add
        let ir = vec![
            Stmt::Move(1),
            Stmt::Add(3),
        ];
        let ssa = ssa(ir);

        println!("Move+Add SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 2);
        match &ssa[0] {
            SsaStmt::Move(delta) => assert_eq!(delta, &1),
            _ => panic!("Expected Move"),
        }
        match &ssa[1] {
            SsaStmt::Add(dst, src, delta) => {
                assert_eq!(dst, &(1, 1));  // Cell 1, version 1
                assert_eq!(src, &(1, 0));  // Cell 1, version 0
                assert_eq!(delta, &3);
            }
            _ => panic!("Expected Add"),
        }
    }

    #[test]
    fn test_simple_loop() {
        // Test: [-] (decrement until zero)
        let ir = vec![
            Stmt::Loop(vec![Stmt::Add(-1)])
        ];
        let ssa = ssa(ir);

        println!("Simple Loop [-] SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 1);
        match &ssa[0] {
            SsaStmt::Loop(control_var, body, phi_nodes) => {
                // Control variable should be the phi node version
                assert_eq!(control_var, &(0, 1));

                // Body should have one Add
                assert_eq!(body.len(), 1);
                match &body[0] {
                    SsaStmt::Add(dst, src, delta) => {
                        assert_eq!(dst, &(0, 2));  // v2 = ...
                        assert_eq!(src, &(0, 1));  // ... v1 + (-1)
                        assert_eq!(delta, &-1);
                    }
                    _ => panic!("Expected Add in loop body"),
                }

                // Should have one phi node for cell 0
                assert_eq!(phi_nodes.len(), 1);
                assert_eq!(phi_nodes[0].dst, (0, 1));
                assert_eq!(phi_nodes[0].incoming.len(), 2);
                assert_eq!(phi_nodes[0].incoming[0], (0, 0));  // Before loop
                assert_eq!(phi_nodes[0].incoming[1], (0, 2));  // After body
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_loop_with_multiple_cells() {
        // Test: [+>-<] (increment cell[0], decrement cell[1])
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Add(1),     // +
                Stmt::Move(1),    // >
                Stmt::Add(-1),    // -
                Stmt::Move(-1),   // <
            ])
        ];
        let ssa = ssa(ir);

        println!("Loop [+>-<] SSA: {:#?}", ssa);

        match &ssa[0] {
            SsaStmt::Loop(control_var, body, phi_nodes) => {
                // Should have phi nodes for cells 0 and 1
                assert_eq!(phi_nodes.len(), 2);

                // Both cells should have phi nodes
                let cell_offsets: Vec<i32> = phi_nodes.iter().map(|p| p.dst.0).collect();
                assert!(cell_offsets.contains(&0));
                assert!(cell_offsets.contains(&1));
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_input_output() {
        // Test: ,. (input then output)
        let ir = vec![
            Stmt::Input(0),
            Stmt::Output(0),
        ];
        let ssa = ssa(ir);

        println!("Input/Output SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 2);
        match &ssa[0] {
            SsaStmt::Input(dst) => assert_eq!(dst, &(0, 1)),
            _ => panic!("Expected Input"),
        }
        match &ssa[1] {
            SsaStmt::Output(src) => assert_eq!(src, &(0, 1)),
            _ => panic!("Expected Output"),
        }
    }

    #[test]
    fn test_multiplication_loop() {
        // Test: MultiplicationLoop with effects at offsets 1 and 2
        let ir = vec![
            Stmt::MultiplicationLoop(1, vec![(1, 2), (2, 3)])
        ];
        let ssa = ssa(ir);

        println!("MultiplicationLoop SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 1);
        match &ssa[0] {
            SsaStmt::MultiplicationLoop(decr, ctrl, effects) => {
                assert_eq!(decr, &1);
                assert_eq!(ctrl, &(0, 0));  // Control cell before modification
                assert_eq!(effects.len(), 2);
                // Each effect should have dst and src versions
                assert_eq!(effects[0].0, (1, 1));  // dst
                assert_eq!(effects[0].1, (1, 0));  // src
                assert_eq!(effects[0].2, 2);       // factor
            }
            _ => panic!("Expected MultiplicationLoop"),
        }
    }

    // ==================== COMPLEX LOOP TESTS ====================

    #[test]
    fn test_nested_loops_simple() {
        // Test: [[+]] - nested loop that increments cell[0]
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Loop(vec![
                    Stmt::Add(1)
                ])
            ])
        ];
        let ssa = ssa(ir);

        println!("Nested Loop [[+]] SSA: {:#?}", ssa);

        // Outer loop
        match &ssa[0] {
            SsaStmt::Loop(outer_control, outer_body, outer_phi) => {
                // Outer control var should be v1 (phi node)
                assert_eq!(outer_control, &(0, 1));

                // Outer phi: cell[0] gets phi node
                assert_eq!(outer_phi.len(), 1);
                assert_eq!(outer_phi[0].dst, (0, 1));
                assert_eq!(outer_phi[0].incoming[0], (0, 0));  // Before outer loop

                // Inner loop
                assert_eq!(outer_body.len(), 1);
                match &outer_body[0] {
                    SsaStmt::Loop(inner_control, inner_body, inner_phi) => {
                        // Inner control should be v2 (inner phi)
                        assert_eq!(inner_control, &(0, 2));

                        // Inner phi: cell[0] gets phi node
                        assert_eq!(inner_phi.len(), 1);
                        assert_eq!(inner_phi[0].dst, (0, 2));
                        assert_eq!(inner_phi[0].incoming[0], (0, 1));  // v1 from outer phi

                        // Add inside inner loop
                        match &inner_body[0] {
                            SsaStmt::Add(dst, src, delta) => {
                                assert_eq!(dst, &(0, 3));   // v3 =
                                assert_eq!(src, &(0, 2));   // v2 + 1
                                assert_eq!(delta, &1);
                            }
                            _ => panic!("Expected Add"),
                        }

                        // Inner phi incoming should have v3 from body
                        assert_eq!(inner_phi[0].incoming[1], (0, 3));
                    }
                    _ => panic!("Expected inner Loop"),
                }

                // Outer phi incoming should have final version from inner loop (v3)
                assert_eq!(outer_phi[0].incoming[1], (0, 3));
            }
            _ => panic!("Expected outer Loop"),
        }
    }

    #[test]
    fn test_nested_loops_different_cells() {
        // Test: [+[->+<]] - outer increments cell[0], inner transfers cell[1] to cell[2]
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Add(1),         // +
                Stmt::Loop(vec![      // [
                    Stmt::Add(-1),    // -
                    Stmt::Move(1),    // >
                    Stmt::Add(1),     // +
                    Stmt::Move(-1),   // <
                ]),                   // ]
            ])
        ];
        let ssa = ssa(ir);

        println!("Nested Loop [+[->+<]] SSA: {:#?}", ssa);

        match &ssa[0] {
            SsaStmt::Loop(outer_control, outer_body, outer_phi) => {
                // Outer loop modifies cell[0] and cell[1] (via inner loop)
                assert_eq!(outer_phi.len(), 2);

                // Find phi nodes for each cell
                let cell0_phi = outer_phi.iter().find(|p| p.dst.0 == 0).unwrap();
                let cell1_phi = outer_phi.iter().find(|p| p.dst.0 == 1).unwrap();

                // Verify phi nodes have correct structure
                assert_eq!(cell0_phi.incoming.len(), 2);
                assert_eq!(cell1_phi.incoming.len(), 2);

                // cell[0] phi: incoming[0] should be (0, 0)
                assert_eq!(cell0_phi.incoming[0], (0, 0));

                // cell[1] phi: incoming[0] should be (1, 0)
                assert_eq!(cell1_phi.incoming[0], (1, 0));
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_loop_not_modifying_control_cell() {
        // Test: [>+<] - loop that increments cell[1] but not cell[0]
        // This tests that control variable is NOT a phi node
        let ir = vec![
            Stmt::Add(5),        // Set cell[0] to 5
            Stmt::Loop(vec![     // While cell[0] != 0
                Stmt::Move(1),   // >
                Stmt::Add(1),    // +
                Stmt::Move(-1),  // <
            ])
        ];
        let ssa = ssa(ir);

        println!("Loop [>+<] not modifying control cell SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 2);

        // First: Add
        match &ssa[0] {
            SsaStmt::Add(dst, src, delta) => {
                assert_eq!(dst, &(0, 1));
                assert_eq!(src, &(0, 0));
                assert_eq!(delta, &5);
            }
            _ => panic!("Expected Add"),
        }

        // Second: Loop
        match &ssa[1] {
            SsaStmt::Loop(control_var, body, phi_nodes) => {
                // IMPORTANT: Control variable should be (0, 1), NOT a phi node
                // because cell[0] is not modified in the loop
                assert_eq!(control_var, &(0, 1));

                // Only cell[1] should have a phi node
                assert_eq!(phi_nodes.len(), 1);
                assert_eq!(phi_nodes[0].dst.0, 1);  // cell[1]

                // Verify cell[1] phi
                assert_eq!(phi_nodes[0].incoming[0], (1, 0));  // Before loop
                assert_eq!(phi_nodes[0].incoming.len(), 2);

                // Body should have: Move(1), Add, Move(-1)
                assert_eq!(body.len(), 3);
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_sequential_loops() {
        // Test: [+][+] - two sequential loops, both increment cell[0]
        let ir = vec![
            Stmt::Loop(vec![Stmt::Add(1)]),   // First loop
            Stmt::Loop(vec![Stmt::Add(1)]),   // Second loop
        ];
        let ssa = ssa(ir);

        println!("Sequential Loops [+][+] SSA: {:#?}", ssa);

        assert_eq!(ssa.len(), 2);

        // First loop
        match &ssa[0] {
            SsaStmt::Loop(control1, body1, phi1) => {
                assert_eq!(control1, &(0, 1));  // First phi
                assert_eq!(phi1.len(), 1);
                assert_eq!(phi1[0].dst, (0, 1));
                assert_eq!(phi1[0].incoming[0], (0, 0));  // Initial
                assert_eq!(phi1[0].incoming[1], (0, 2));  // After body

                match &body1[0] {
                    SsaStmt::Add(dst, src, _) => {
                        assert_eq!(dst, &(0, 2));
                        assert_eq!(src, &(0, 1));
                    }
                    _ => panic!("Expected Add"),
                }
            }
            _ => panic!("Expected first Loop"),
        }

        // Second loop
        match &ssa[1] {
            SsaStmt::Loop(control2, body2, phi2) => {
                assert_eq!(control2, &(0, 3));  // Second phi
                assert_eq!(phi2.len(), 1);
                assert_eq!(phi2[0].dst, (0, 3));
                assert_eq!(phi2[0].incoming[0], (0, 2));  // From first loop!
                assert_eq!(phi2[0].incoming[1], (0, 4));  // After body

                match &body2[0] {
                    SsaStmt::Add(dst, src, _) => {
                        assert_eq!(dst, &(0, 4));
                        assert_eq!(src, &(0, 3));
                    }
                    _ => panic!("Expected Add"),
                }
            }
            _ => panic!("Expected second Loop"),
        }
    }

    #[test]
    fn test_loop_with_moves_balanced() {
        // Test: [>+>+<<-] - loop that moves right, does work, then returns
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(1),    // >
                Stmt::Add(1),     // +
                Stmt::Move(1),    // >
                Stmt::Add(1),     // +
                Stmt::Move(-2),   // <<
                Stmt::Add(-1),    // -
            ])
        ];
        let ssa = ssa(ir);

        println!("Loop with moves [>+>+<<-] SSA: {:#?}", ssa);

        match &ssa[0] {
            SsaStmt::Loop(control_var, body, phi_nodes) => {
                // Should have phi nodes for cells 0, 1, and 2
                assert_eq!(phi_nodes.len(), 3);

                let cells: Vec<i32> = phi_nodes.iter().map(|p| p.dst.0).collect();
                assert!(cells.contains(&0));
                assert!(cells.contains(&1));
                assert!(cells.contains(&2));

                // Each phi should have 2 incoming values
                for phi in phi_nodes {
                    assert_eq!(phi.incoming.len(), 2,
                        "Phi for cell {} should have 2 incoming values", phi.dst.0);

                    // Verify incoming[0] has version 0 (before loop)
                    assert_eq!(phi.incoming[0].1, 0,
                        "Phi for cell {} should have version 0 as incoming[0]", phi.dst.0);
                }

                // Control variable should be phi for cell[0]
                let cell0_phi = phi_nodes.iter().find(|p| p.dst.0 == 0).unwrap();
                assert_eq!(control_var, &cell0_phi.dst);
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_complex_nested_with_multiple_cells() {
        // Test: [>+[<+>-]<-] - Complex nested loop with multiple cells
        // Outer: moves right, inner loop, moves left, decrements
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(1),         // > (at cell 1)
                Stmt::Add(1),          // +
                Stmt::Loop(vec![       // [
                    Stmt::Move(-1),    //   < (at cell 0)
                    Stmt::Add(1),      //   +
                    Stmt::Move(1),     //   > (at cell 1)
                    Stmt::Add(-1),     //   -
                ]),                    // ]
                Stmt::Move(-1),        // < (at cell 0)
                Stmt::Add(-1),         // -
            ])
        ];
        let ssa = ssa(ir);

        println!("Complex nested [>+[<+>-]<-] SSA: {:#?}", ssa);

        match &ssa[0] {
            SsaStmt::Loop(outer_control, outer_body, outer_phi) => {
                // Outer loop modifies both cell[0] and cell[1]
                assert_eq!(outer_phi.len(), 2);

                let cell0_phi = outer_phi.iter().find(|p| p.dst.0 == 0).unwrap();
                let cell1_phi = outer_phi.iter().find(|p| p.dst.0 == 1).unwrap();

                // Verify phi structure
                assert_eq!(cell0_phi.incoming.len(), 2);
                assert_eq!(cell1_phi.incoming.len(), 2);

                // Check that outer control uses cell[0] phi
                assert_eq!(outer_control, &cell0_phi.dst);

                // Find inner loop in body
                let inner_loop_pos = outer_body.iter().position(|stmt| {
                    matches!(stmt, SsaStmt::Loop(_, _, _))
                }).expect("Should have inner loop");

                match &outer_body[inner_loop_pos] {
                    SsaStmt::Loop(inner_control, inner_body, inner_phi) => {
                        // Inner loop also modifies both cells
                        assert_eq!(inner_phi.len(), 2);

                        // Verify each inner phi has 2 incoming values
                        for phi in inner_phi {
                            assert_eq!(phi.incoming.len(), 2);
                        }

                        // Inner body should have operations
                        assert!(inner_body.len() > 0);
                    }
                    _ => panic!("Expected inner Loop"),
                }
            }
            _ => panic!("Expected outer Loop"),
        }
    }

    // ==================== UNBALANCED LOOP TESTS ====================

    #[test]
    #[should_panic(expected = "Unbalanced loop detected with net pointer movement of 1")]
    fn test_unbalanced_loop_move_right() {
        // Test: [>-] - moves right and decrements (unbalanced)
        // This should panic because net movement is +1
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(1),    // >
                Stmt::Add(-1),    // -
            ])
        ];
        let _ssa = ssa(ir);  // Should panic
    }

    #[test]
    #[should_panic(expected = "Unbalanced loop detected with net pointer movement of -1")]
    fn test_unbalanced_loop_move_left() {
        // Test: [<+] - moves left and increments (unbalanced)
        // This should panic because net movement is -1
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(-1),   // <
                Stmt::Add(1),     // +
            ])
        ];
        let _ssa = ssa(ir);  // Should panic
    }

    #[test]
    #[should_panic(expected = "Unbalanced loop detected with net pointer movement of 2")]
    fn test_unbalanced_loop_large_movement() {
        // Test: [>>+] - moves right by 2 and increments (unbalanced)
        // This should panic because net movement is +2
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(2),    // >>
                Stmt::Add(1),     // +
            ])
        ];
        let _ssa = ssa(ir);  // Should panic
    }

    #[test]
    #[should_panic(expected = "Nested loop has unbalanced pointer movement")]
    fn test_nested_unbalanced_loop() {
        // Test: [[>+]] - nested loop with unbalanced inner loop
        // This should panic when processing the inner loop
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Loop(vec![
                    Stmt::Move(1),    // >
                    Stmt::Add(1),     // +
                ])
            ])
        ];
        let _ssa = ssa(ir);  // Should panic
    }

    #[test]
    #[should_panic(expected = "Nested loop has unbalanced pointer movement")]
    fn test_balanced_outer_unbalanced_inner() {
        // Test: [>[-<]<] - balanced outer loop, but unbalanced inner loop
        // The outer loop is balanced (net movement = 0), but inner is not
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(1),       // >
                Stmt::Loop(vec![     // Inner loop is unbalanced
                    Stmt::Add(-1),   // -
                    Stmt::Move(-1),  // <
                ]),
                Stmt::Move(-1),      // <
            ])
        ];
        let _ssa = ssa(ir);  // Should panic when processing inner loop
    }

    #[test]
    fn test_balanced_loop_with_complex_movement() {
        // Test: [>>>+<<<-] - balanced loop with complex movement pattern
        // Net movement = 0, so should succeed
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Move(3),    // >>>
                Stmt::Add(1),     // +
                Stmt::Move(-3),   // <<<
                Stmt::Add(-1),    // -
            ])
        ];
        let ssa = ssa(ir);

        println!("Balanced complex movement [>>>+<<<-] SSA: {:#?}", ssa);

        // Should successfully convert with phi nodes for cells 0 and 3
        match &ssa[0] {
            SsaStmt::Loop(_control_var, _body, phi_nodes) => {
                assert_eq!(phi_nodes.len(), 2);
                let cells: Vec<i32> = phi_nodes.iter().map(|p| p.dst.0).collect();
                assert!(cells.contains(&0));
                assert!(cells.contains(&3));
            }
            _ => panic!("Expected Loop"),
        }
    }

    #[test]
    fn test_pointer_offset_restoration() {
        // Test that pointer offset is properly restored after a balanced loop
        let ir = vec![
            Stmt::Move(2),        // >> (move to cell 2)
            Stmt::Loop(vec![      // Balanced loop
                Stmt::Move(1),    // >
                Stmt::Add(1),     // +
                Stmt::Move(-1),   // <
                Stmt::Add(-1),    // -
            ]),
            Stmt::Add(5),         // + (should apply to cell 2, not cell 3!)
        ];
        let ssa = ssa(ir);

        println!("Pointer offset restoration test SSA: {:#?}", ssa);

        // The last Add should be at cell 2, not cell 3
        match &ssa[2] {
            SsaStmt::Add(dst, src, delta) => {
                assert_eq!(dst.0, 2, "After loop, pointer should be back at cell 2");
                assert_eq!(src.0, 2);
                assert_eq!(delta, &5);
            }
            _ => panic!("Expected Add as third statement"),
        }
    }

    #[test]
    fn test_loop_modifying_many_cells() {
        // Test: [+>+>+>+<<<<-] - loop that modifies 4 cells
        let ir = vec![
            Stmt::Loop(vec![
                Stmt::Add(1),     // cell[0] +
                Stmt::Move(1),    // >
                Stmt::Add(1),     // cell[1] +
                Stmt::Move(1),    // >
                Stmt::Add(1),     // cell[2] +
                Stmt::Move(1),    // >
                Stmt::Add(1),     // cell[3] +
                Stmt::Move(-3),   // <<<
                Stmt::Add(-1),    // cell[0] -
            ])
        ];
        let ssa = ssa(ir);

        println!("Loop modifying many cells [+>+>+>+<<<<-] SSA: {:#?}", ssa);

        match &ssa[0] {
            SsaStmt::Loop(control_var, _body, phi_nodes) => {
                // Should have phi nodes for cells 0, 1, 2, 3
                assert_eq!(phi_nodes.len(), 4, "Should have 4 phi nodes");

                let cells: Vec<i32> = phi_nodes.iter().map(|p| p.dst.0).collect();
                assert!(cells.contains(&0), "Should have phi for cell 0");
                assert!(cells.contains(&1), "Should have phi for cell 1");
                assert!(cells.contains(&2), "Should have phi for cell 2");
                assert!(cells.contains(&3), "Should have phi for cell 3");

                // All phis should have exactly 2 incoming values
                for phi in phi_nodes {
                    assert_eq!(phi.incoming.len(), 2,
                        "Phi for cell {} should have exactly 2 incoming", phi.dst.0);

                    // First incoming should be version 0 (initial state)
                    assert_eq!(phi.incoming[0], (phi.dst.0, 0),
                        "First incoming for cell {} should be initial version", phi.dst.0);

                    // Second incoming should be from the loop body (version > 0)
                    assert!(phi.incoming[1].1 > 0,
                        "Second incoming for cell {} should be from loop body", phi.dst.0);
                }

                // Control variable should be phi for cell[0]
                let cell0_phi = phi_nodes.iter().find(|p| p.dst.0 == 0).unwrap();
                assert_eq!(control_var, &cell0_phi.dst);
            }
            _ => panic!("Expected Loop"),
        }
    }
}