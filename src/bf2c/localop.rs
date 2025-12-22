pub mod localop {
    enum Prog {
        Vec(Stmt),
    }

    enum Stmt {
        Add(i32),
        Move(i32),
        Output(i32),
        Input(i32),
        Loop(Box<Prog>),
        ZeroLoop,
        ScanLoop(i32),
        MultiplicationLoop(u8, Vec<(i32, i32)>),
    }
}