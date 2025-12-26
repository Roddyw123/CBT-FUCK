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
        let mut stmts = Vec::new();
        let mut loop_stack = Vec::new();

        for symbol in prog {
            match symbol {
                BfSymbol::Plus => {
                    if let Some(Stmt::Add(n)) = stmts.last_mut() {
                        *n += 1;
                        if n == &0 {
                            stmts.pop();
                        }
                    } else {
                        stmts.push(Stmt::Add(1));
                    }
                }
                BfSymbol::Minus => {
                    if let Some(Stmt::Add(n)) = stmts.last_mut() {
                        *n -= 1;
                        if n == &0 {
                            stmts.pop();
                        }
                    } else {
                        stmts.push(Stmt::Add(-1));
                    }
                }
                BfSymbol::Right => {
                    if let Some(Stmt::Move(n)) = stmts.last_mut() {
                        *n += 1;
                        if n == &0 {
                            stmts.pop();
                        }
                    } else {
                        stmts.push(Stmt::Move(1));
                    }
                }
                BfSymbol::Left => {
                    if let Some(Stmt::Move(n)) = stmts.last_mut() {
                        *n -= 1;
                        if n == &0 {
                            stmts.pop();
                        }
                    } else {
                        stmts.push(Stmt::Move(-1));
                    }
                }
                BfSymbol::Period => {
                    if let Some(Stmt::Output(n)) = stmts.last_mut() {
                        *n += 1;
                    } else {
                        stmts.push(Stmt::Output(1));
                    }
                }
                BfSymbol::Comma => {
                    if let Some(Stmt::Input(n)) = stmts.last_mut() {
                        *n += 1;
                    } else {
                        stmts.push(Stmt::Input(1));
                    }
                }
                BfSymbol::OpenBracket => {
                    loop_stack.push(stmts);
                    stmts = Vec::new();
                }
                BfSymbol::CloseBracket => {
                    if let Some(mut start) = loop_stack.pop() {
                        let loop_body = stmts;
                        start.push(match loop_body[..] {
                            // TODO: zero loop detection
                            // scan loop
                            [Stmt::Move(dir @ (1 | -1))] => Stmt::ScanLoop(dir),
                            _ => Stmt::Loop(Prog::Vec(loop_body)),
                        });
                        stmts = start;
                    }
                }
            }
        }

        Prog::Vec(stmts)
    }

    #[cfg(test)]
    mod tests {
        use super::*;

        #[test]
        fn loop_nesting_1_test() {
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::OpenBracket,
                BfSymbol::Comma,
                BfSymbol::CloseBracket,
                BfSymbol::Minus,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Loop(Prog::Vec(vec![
                    Stmt::Loop(Prog::Vec(vec![Stmt::Input(1)])),
                    Stmt::Add(-1)
                ])),])
            );
        }

        #[test]
        fn loop_nesting_2_test() {
            let symbols = vec![
                BfSymbol::Period,
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::OpenBracket,
                BfSymbol::Period,
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
                        Stmt::Loop(Prog::Vec(vec![Stmt::Output(1)]))
                    ]))
                ])
            );
        }

        #[test]
        fn add_test() {
            let symbols = vec![BfSymbol::Plus, BfSymbol::Plus, BfSymbol::Plus];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Add(3)]));
        }

        #[test]
        fn subtract_test() {
            let symbols = vec![BfSymbol::Minus, BfSymbol::Minus, BfSymbol::Minus];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Add(-3)]));
        }

        #[test]
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
        fn move_right_test() {
            let symbols = vec![BfSymbol::Right, BfSymbol::Right, BfSymbol::Right];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(3)]));
        }

        #[test]
        fn move_left_test() {
            let symbols = vec![BfSymbol::Left, BfSymbol::Left, BfSymbol::Left];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(-3)]));
        }

        #[test]
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

        #[test]
        fn scan_loop_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::OpenBracket,
                BfSymbol::Right,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(optimized, Prog::Vec(vec![Stmt::Move(1), Stmt::ScanLoop(1)]));
        }

        #[test]
        fn scan_loop_negative_test() {
            let symbols = vec![
                BfSymbol::Left,
                BfSymbol::OpenBracket,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Move(-1), Stmt::ScanLoop(-1)])
            );
        }

        #[test]
        fn not_scan_loop_test() {
            let symbols = vec![
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
                    Stmt::Move(1),
                    Stmt::Loop(Prog::Vec(vec![Stmt::Move(2)]))
                ])
            );
        }

        #[test]
        fn subtle_true_scan_loop_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::OpenBracket,
                BfSymbol::Right,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Move(1), Stmt::ScanLoop(-1),])
            );
        }

        #[test]
        fn ignore_cancelled_operations_in_scan_loop_test() {
            let symbols = vec![
                BfSymbol::Right,
                BfSymbol::OpenBracket,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Minus,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Move(1), Stmt::ScanLoop(1),])
            );
        }

        #[test]
        fn simple_multiplication_loop_test() {
            // [->+<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::MultiplicationLoop(1, vec![(1, 1)])])
            );
        }

        #[test]
        fn multiplication_loop_with_larger_offset_test() {
            // [->>++<<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::MultiplicationLoop(1, vec![(2, 2)])])
            );
        }

        #[test]
        fn multiplication_loop_multiple_targets_test() {
            // [-<+>>+++<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Left,
                BfSymbol::Plus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::MultiplicationLoop(1, vec![(-1, 1), (1, 3)])])
            );
        }

        #[test]
        fn multiplication_loop_odd_decrement_test() {
            // [--->>++>>>+++++<<<<<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::MultiplicationLoop(3, vec![(2, 2), (5, 5)])])
            );
        }

        #[test]
        fn not_multiplication_loop_even_decrement_test() {
            // [-->>+<<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Loop(Prog::Vec(vec![
                    Stmt::Add(-2),
                    Stmt::Move(2),
                    Stmt::Add(1),
                    Stmt::Move(-2),
                ]))])
            );
        }

        #[test]
        fn not_multiplication_loop_even_decrement_large_test() {
            // [---->>+<<]
            let symbols = vec![
                BfSymbol::OpenBracket,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Minus,
                BfSymbol::Right,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Left,
                BfSymbol::Left,
                BfSymbol::CloseBracket,
            ];
            let optimized = optimise_local(symbols);
            assert_eq!(
                optimized,
                Prog::Vec(vec![Stmt::Loop(Prog::Vec(vec![
                    Stmt::Add(-4),
                    Stmt::Move(2),
                    Stmt::Add(1),
                    Stmt::Move(-2),
                ]))])
            );
        }
    }
}
