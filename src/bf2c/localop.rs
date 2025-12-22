pub mod localop {
    use super::super::bf2c::*;

    #[derive(Debug, PartialEq, Eq)]
    pub enum Prog {
        Vec(Vec<Stmt>),
    }

    #[derive(Debug, PartialEq, Eq)]
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

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        #[ignore]
        fn loop_nesting_1_test() {
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::OpenBracket,
                BfSymbol::Plus,
                BfSymbol::CloseBracket,
                BfSymbol::Minus,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Loop(Prog::Vec(vec![Stmt::Loop(Prog::Vec(vec![Stmt::Add(1)])), Stmt::Add(-1)])),
                    
                ])
            );
        }

        #[test]
        #[ignore]
        fn loop_nesting_2_test() {
            let symbols = vec![
                BfSymbol::Period,
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::OpenBracket,
                BfSymbol::Plus,
                BfSymbol::CloseBracket,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Output(1),
                    Stmt::Loop(Prog::Vec(vec![
                        Stmt::Add(-1),
                        Stmt::Loop(Prog::Vec(vec![Stmt::Add(1)]))
                    ]))
                ])
            );
        }

        #[test]
        #[ignore]
        fn add_test() {
            let symbols = vec![BfSymbol::Plus, BfSymbol::Plus, BfSymbol::Plus];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Add(3)]));
        }

        #[test]
        #[ignore]
        fn subtract_test() {
            let symbols = vec![BfSymbol::Minus, BfSymbol::Minus, BfSymbol::Minus];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Add(-3)]));
        }

        #[test]
        #[ignore]
        fn add_zero_test() {
            let symbols = vec![
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Minus,
                BfSymbol::Minus,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Add(1)]));
        }

        #[test]
        #[ignore]
        fn move_right_test() {
            let symbols = vec![BfSymbol::Right, BfSymbol::Right, BfSymbol::Right];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(3)]));
        }

        #[test]
        #[ignore]
        fn move_left_test() {
            let symbols = vec![BfSymbol::Left, BfSymbol::Left, BfSymbol::Left];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(-3)]));
        }

        #[test]
        #[ignore]
        fn move_cancel_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Left,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(-1)]));
        }

        #[test]
        #[ignore]
        fn input_output_test() {
            let symbols = vec![
                BfSymbol::Comma,
                BfSymbol::Comma,
                BfSymbol::Period,
                BfSymbol::Period,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Input(2), Stmt::Output(2)]));
        }

        #[test]
        #[ignore]
        fn no_cancel_io_test() {
            let symbols = vec![
                BfSymbol::Comma,
                BfSymbol::Period,
                BfSymbol::Comma,
                BfSymbol::Period,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Input(1),
                    Stmt::Output(1),
                    Stmt::Input(1),
                    Stmt::Output(1)
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_add_move_test() {
            let symbols = vec![
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Left,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Add(2),
                    Stmt::Move(2),
                    Stmt::Add(2),
                    Stmt::Move(-3)
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_add_io_test() {
            let symbols = vec![
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Comma,
                BfSymbol::Comma,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Period,
                BfSymbol::Period,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Add(2),
                    Stmt::Input(2),
                    Stmt::Add(2),
                    Stmt::Output(2)
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_move_io_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Comma,
                BfSymbol::Comma,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Period,
                BfSymbol::Period,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Move(2),
                    Stmt::Input(2),
                    Stmt::Move(-2),
                    Stmt::Output(2)
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_add_loop_test() {
            let symbols = vec![
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::OpenBracket,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Add(2),
                    Stmt::Loop(Prog::Vec(vec![Stmt::Add(2)]))
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_move_loop_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::OpenBracket,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Move(2),
                    Stmt::Loop(Prog::Vec(vec![Stmt::Move(2)]))
                ])
            );
        }

        #[test]
        #[ignore]
        fn no_coalescing_io_loop_test() {
            let symbols = vec![
                BfSymbol::Comma,
                BfSymbol::Comma,
                BfSymbol::OpenBracket,
                BfSymbol::Comma,
                BfSymbol::Comma,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![
                    Stmt::Input(2),
                    Stmt::Loop(Prog::Vec(vec![Stmt::Input(2)]))
                ])
            );
        }
    }
}
