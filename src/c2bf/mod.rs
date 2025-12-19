pub mod c2bf {

    use chumsky::{IterParser, Parser, error::Cheap, extra::{Err, ParserExtra}, input::Input, prelude::{choice, just, recursive}, text::{self, ascii::keyword}};

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Type {
        Char,
        Int
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Atom<'src> {
        Num(u32),
        Var(&'src str),
        Array(Box<Atom<'src>>, Box<Expr<'src>>),
    }
    
    #[derive(Debug, Clone, PartialEq, Eq)]
    enum Expr<'src> {
        Atom(Atom<'src>),
        Neg(Box<Expr<'src>>),
        Add(Box<Expr<'src>>, Box<Expr<'src>>),
        Mul(Box<Expr<'src>>, Box<Expr<'src>>),
        Le(Box<Expr<'src>>, Box<Expr<'src>>),
        Ge(Box<Expr<'src>>, Box<Expr<'src>>),
        Eq(Box<Expr<'src>>, Box<Expr<'src>>),
        Inc(Box<Expr<'src>>),
        Dec(Box<Expr<'src>>),
        
        Call(&'src str, Vec<Expr<'src>>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum LStmt<'src> {
        Dec(Type, &'src str, Option<Option<Expr<'src>>>, Option<Expr<'src>>),
        While(Expr<'src>, Vec<LStmt<'src>>),
        Ifs((Expr<'src>, Vec<LStmt<'src>>), Vec<(Expr<'src>, Vec<LStmt<'src>>)>, Option<Vec<LStmt<'src>>>),
        For(Option<Box<LStmt<'src>>>, Option<Box<LStmt<'src>>>, Option<Box<LStmt<'src>>>, Vec<LStmt<'src>>),
        FuncDec(Type, &'src str, Vec<(Type, &'src str, Option<Option<Expr<'src>>>)>, Vec<LStmt<'src>>),
        Expr(Expr<'src>),
        Assignment(&'src str, Expr<'src>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    enum GStmt<'src> {
        VarDec(Type, &'src str, Option<Option<Expr<'src>>>, Option<Expr<'src>>),
        FuncDec(Type, &'src str, Vec<(Type, &'src str, Option<Option<Expr<'src>>>)>, Vec<LStmt<'src>>),
    }

    fn parser<'src, I: Input<'src>, E: ParserExtra<'src, I>>() -> impl Parser<'src, &'src str, Vec<GStmt<'src>>, Err<Cheap>> {
        // not mapped to Var immediatly as it can be a function as well
        let ident = text::ascii::ident()
            .padded();

        let num = text::int(10)
            .padded();

        let sep = just(';').padded();

        let open_bracket = just('(').padded();
        let close_bracket = just(')').padded();
        let open_curly_bracket = just('{').padded();
        let close_curly_bracket = just('}').padded();
        let open_square_bracket = just('[').padded();
        let close_square_bracket = just(']').padded();

        let expr = || recursive(|expr| {

            let atom = recursive(|atom| {
                let array = atom.map(Box::new)
                        .then(
                            expr.clone()
                            .delimited_by(open_square_bracket, close_square_bracket)
                            .map(Box::new)
                        );
                choice((
                    ident
                        .map(Atom::Var),
                    num
                        .map(|s: &str|
                            Atom::Num(s.parse().unwrap())),
                    // TODO: solve left-recursion issue
                    // array
                    //     .map(|(name, id)|
                    //         Atom::Array(name, id)),
                    // TODO: struct inline definition { ... }
                ))
            });

            choice((
                atom
                    .map(Expr::Atom),
                expr.clone()
                    .delimited_by(open_bracket, close_bracket),
            ))
        });

        let types = || choice((
            keyword("char").to(Type::Char),
            keyword("int").to(Type::Int)
        ));

        let typed_variable = || types()
            .then(ident)
            .then(
                // hard coded 1D array
                expr()
                .or_not()
                .delimited_by(open_square_bracket, close_square_bracket)
                .or_not()
            );
        
        let declaration = || typed_variable()
            .then(
                just('=').padded()
                .ignore_then(expr())
                .or_not()
            )
            .then_ignore(sep);

        fn block_help<'src, I: Input<'src>, E: ParserExtra<'src, I>, S, O, C>(stmt: S, open: O, close: C) -> impl Parser<'src, I, Vec<LStmt<'src>>, E> + Clone
        where
            S: Parser<'src, I, LStmt<'src>, E> + Clone,
            O: Parser<'src, I, char, E> + Clone,
            C: Parser<'src, I, char, E> + Clone,
        {
            stmt.clone()
                .repeated()
                .collect::<Vec<LStmt<'src>>>()
                .delimited_by(open, close)
        }

        let func_dec_help = |stmt| types()
                .then(ident)
                .then(
                    typed_variable()
                    .separated_by(just(',').padded())
                    .allow_trailing()
                    .collect::<Vec<_>>()
                    .delimited_by(open_bracket, close_bracket)
                )
                .then(block_help(stmt, open_curly_bracket, close_curly_bracket).clone());
        
        let local_stmt =  || recursive(|stmt| {
            let block = block_help(stmt.clone(), open_curly_bracket, close_curly_bracket);

            let x_statment = |name| keyword(name).padded()
                .ignore_then(
                    expr()
                    .delimited_by(open_bracket, close_bracket)
                )
                .then(block.clone());
            let if_statment = x_statment("if").clone()
                .then(
                    keyword("else").padded()
                    .ignore_then(x_statment("if").clone())
                    .repeated().collect::<Vec<(Expr, Vec<LStmt>)>>()
                )
                .then(
                    keyword("else").padded()
                    .ignore_then(block.clone())
                    .or_not()
                );
            let for_loop = keyword("for").padded()
                .ignore_then(
                    stmt.clone().or_not()
                    .then_ignore(sep)
                    .then(stmt.clone().or_not())
                    .then_ignore(sep)
                    .then(stmt.clone().or_not())
                    .delimited_by(open_bracket, close_bracket)
                )
                .then(block.clone());
            let func_dec = func_dec_help.clone()(stmt);
            let assignment = ident.clone()
                .then(
                    just('=').padded()
                    .ignore_then(expr())
                )
                .then_ignore(sep);
            choice((
                declaration()
                    .map(|(((ty, name), arr), exp)|
                        LStmt::Dec(ty, name, arr, exp)),
                x_statment("while")
                    .map(|(cond, body)|
                        LStmt::While(cond, body)),
                if_statment
                    .map(|((e1, e2), else_tail)|
                        LStmt::Ifs(e1, e2, else_tail)),
                for_loop
                    .map(|(((e1, e2), e3),body)|
                        LStmt::For(e1.map(Box::new), e2.map(Box::new), e3.map(Box::new), body)),
                func_dec.clone()
                    .map(|(((ty,name ), params), body)|
                        LStmt::FuncDec(ty, name, params.into_iter().map(|((ty, name), arr)| (ty, name, arr)).collect(), body)),
                expr()
                    .map(LStmt::Expr),
                assignment
                    .map(|(name, exp)|
                        LStmt::Assignment(name, exp)),
            ))
        });
        let global_stmts = || choice((
            declaration()
                .map(|(((ty, name), arr), exp)|
                    GStmt::VarDec(ty, name, arr, exp)),
            func_dec_help.clone()(local_stmt())
                .map(|(((ty,name ), params), body)|
                    GStmt::FuncDec(ty, name, params.into_iter().map(|((ty, name), arr)| (ty, name, arr)).collect(), body)),
        ))
        .repeated()
        .collect::<Vec<GStmt>>();
        global_stmts()
    }

    #[cfg(test)]
    mod tests {
        use chumsky::{Parser, error::EmptyErr, extra::Err};

        use crate::c2bf::c2bf::{Expr, GStmt, LStmt, Atom, Type};

        use super::parser;
        #[test]
        fn empty_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        // Variable Declaration Tests
        #[test]
        fn global_char_variable_declaration_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e;").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::VarDec(Type::Char, "e", None, None)]));
        }

        #[test]
        fn global_char_variable_declaration_missing_semicolon_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_empty_array_variable_declaration_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e[];").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::VarDec(Type::Char, "e", Some(None), None)]));
        }

        #[test]
        fn global_char_sized_array_variable_declaration_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e[v];").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::VarDec(Type::Char, "e", Some(Some(Expr::Atom(Atom::Var("v")))), None)]));
        }

        #[test]
        fn global_char_variable_declaration_missing_expression_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e =;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_extra_identifier_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e e =;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_unmatched_right_bracket_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e [=;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_unmatched_left_bracket_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e ]=;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_untyped_variable_assignment_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("e = v;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_variable_declaration_assignment_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e = v;").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::VarDec(Type::Char, "e", None, Some(Expr::Atom(Atom::Var("v"))))]));
        }

        #[test]
        fn global_char_variable_declaration_assignment_missing_semicolon_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char e = v").into_result();
            assert!(stmts.is_err());
        }

        // Function Declaration Tests
        #[test]
        fn global_function_declaration_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(){}").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::FuncDec(Type::Char, "foo", Vec::new(), Vec::new())]));
        }

        #[test]
        fn global_function_declaration_missing_return_type_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("foo(){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_name_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char (){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_open_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_closed_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo({}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_extra_open_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo((){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_extra_closed_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo()){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_open_curly_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo()}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_closed_curly_bracket_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(){").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_1_parameter_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(char a){}").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::FuncDec(Type::Char, "foo", vec![(Type::Char, "a", None)], Vec::new())]));
        }

        #[test]
        fn global_function_declaration_1_parameter_trailing_comma_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(char a,){}").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::FuncDec(Type::Char, "foo", vec![(Type::Char, "a", None)], Vec::new())]));
        }

        // failing parameters
        #[test]
        fn global_function_declaration_parameter_missing_type_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(a){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_parameter_missing_name_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(char){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_single_comma_fail_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(,){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_2_parameters_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(char a, char b){}").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::FuncDec(Type::Char, "foo", vec![(Type::Char, "a", None), (Type::Char, "b", None)], Vec::new())]));
        }

        #[test]
        fn global_function_declaration_2_parameters_trailing_comma_test() {
            let stmts = parser::<&str, Err<EmptyErr>>().parse("char foo(char a, char b, ){}").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::FuncDec(Type::Char, "foo", vec![(Type::Char, "a", None), (Type::Char, "b", None)], Vec::new())]));
        }
    }
}