use std::collections::HashMap;
pub type Prog = Vec<Stmt>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Stmt {
    Add(i32),
    Move(i32),
    Output(i32),
    Input(i32),
    Loop(Prog),
    ZeroLoop,
    ScanLoop(i32),
    MultiplicationLoop(u8, Vec<(i32, i32)>),
    Set(u8),
}