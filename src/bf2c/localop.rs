pub mod localop {
    use super::super::bf2c::*;
    pub enum Prog {
        Vec(Vec<Stmt>),
    }

    pub enum Stmt {
        Add(i32),
        Move(i32),
        Output(i32),
        Input(i32),
        Loop(Prog),
        ZeroLoop,
        ScanLoop(i32),
        MultiplicationLoop(u8, Vec<(i32, i32)>),
    }

    pub fn optimise_local(prog: Vec<BfSymbol>) -> Prog {
        todo!()
    }
}