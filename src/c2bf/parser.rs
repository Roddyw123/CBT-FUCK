pub mod parser {

    use super::super::cast::ast::*;
    use chumsky::{
        container::OrderedSeq,
        error::Rich,
        extra::{Err, ParserExtra},
        input::Input,
        pratt::*,
        prelude::{choice, just, recursive},
        text::{self},
        IterParser, Parser,
    };

    pub fn parser<'src>() -> impl Parser<'src, &'src str, Vec<GStmt<'src>>, Err<Rich<'src, char>>> {
        // not mapped to Var immediatly as it can be a function as well
        // TODO: proper identifier parsing (regex probably)
        let ident = || text::ascii::ident().padded();

        let num = || text::int(10).padded();

        let sep = || just(';').padded();

        let open_bracket = || just('(').padded();
        let close_bracket = || just(')').padded();
        let open_curly_bracket = || just('{').padded();
        let close_curly_bracket = || just('}').padded();
        let open_square_bracket = || just('[').padded();
        let close_square_bracket = || just(']').padded();

        let expr = || {
            recursive(|expr| {
                let atom = || {
                    recursive(|atom| {
                        let array = atom.map(Box::new).then(
                            expr.clone()
                                .delimited_by(open_square_bracket(), close_square_bracket())
                                .map(Box::new),
                        );
                        choice((
                            ident().map(Atom::Var),
                            num().map(|s: &str| Atom::Num(s.parse().unwrap())),
                            // TODO: solve left-recursion issue
                            // array
                            //     .map(|(name, id)|
                            //         Atom::Array(name, id)),
                            // TODO: struct inline definition { ... }
                        ))
                    })
                };

                choice((
                    atom().map(Expr::Atom),
                    expr.clone().delimited_by(open_bracket(), close_bracket()),
                ))
                .pratt((
                    prefix(14, just('!').padded(), |e1, x, e| Expr::Neg(Box::new(x))),
                    infix(left(13), just('*').padded(), |x, e1, y, e| {
                        Expr::Mul(Box::new(x), Box::new(y))
                    }),
                    infix(left(12), just('+').padded(), |x, e1, y, e| {
                        Expr::Add(Box::new(x), Box::new(y))
                    }),
                    infix(left(10), just('<').padded(), |x, e1, y, e| {
                        Expr::Lt(Box::new(x), Box::new(y))
                    }),
                    infix(left(10), just('>').padded(), |x, e1, y, e| {
                        Expr::Gt(Box::new(x), Box::new(y))
                    }),
                    infix(left(9), just("==").padded(), |x, e1, y, e| {
                        Expr::Eq(Box::new(x), Box::new(y))
                    }),
                    infix(right(2), just("=").padded(), |x, e1, y, e| {
                        Expr::Assignment(Box::new(x), Box::new(y))
                    }),
                    postfix(
                        15,
                        expr.clone()
                            .separated_by(just(',').padded())
                            .allow_trailing()
                            .collect::<Vec<Expr>>()
                            .delimited_by(open_bracket(), close_bracket()),
                        |x, y, e| Expr::Call(Box::new(x), y),
                    ),
                ))
                //TODO: then fold with postfix operators
            })
        };

        let types =
            || choice((text::keyword("char").to(Type::Char), text::keyword("int").to(Type::Int))).padded();

        let typed_variable = || {
            types().then(ident()).then(
                // hard coded 1D array
                expr()
                    .or_not()
                    .delimited_by(open_square_bracket(), close_square_bracket())
                    .or_not(),
            )
        };

        let declaration = || {
            typed_variable()
                .then(just('=').padded().ignore_then(expr()).or_not())
                .then_ignore(sep())
        };

        fn block_help<'src, I: Input<'src>, E: ParserExtra<'src, I>, S, O, C>(
            stmt: S,
            open: O,
            close: C,
        ) -> impl Parser<'src, I, Vec<LStmt<'src>>, E> + Clone
        where
            S: Parser<'src, I, LStmt<'src>, E> + Clone,
            O: Parser<'src, I, char, E> + Clone,
            C: Parser<'src, I, char, E> + Clone,
            <I as Input<'src>>::Token: PartialEq,
            char: OrderedSeq<'src, <I as Input<'src>>::Token>,
        {
            stmt.separated_by(just(';').repeated())
                .allow_trailing()
                .allow_leading()
                .collect::<Vec<LStmt<'src>>>()
                .delimited_by(open, close)
        }

        // bad option?(don't know why)
        // fn func_dec_help<'src, I: Input<'src>+ ValueInput<'src>, E: ParserExtra<'src, I>, S>(stmt: S) -> impl Parser<'src, I, ((Type, &'src str), Vec<((Type, &'src str), Option<Option<Expr<'src>>>)>, Vec<LStmt<'src>>), E> + Clone
        // where
        //     S: Parser<'src, I, LStmt<'src>, E> + Clone,
        //     <I as Input<'src>>::Token: chumsky::text::Char,
        //     char: OrderedSeq<'src, <I as Input<'src>>::Token>
        // {
        let func_dec_help = |stmt| {
            types()
                .then(ident())
                .then(
                    typed_variable()
                        .separated_by(just(',').padded())
                        .allow_trailing()
                        .collect::<Vec<_>>()
                        .delimited_by(open_bracket(), close_bracket()),
                )
                .then(block_help(stmt, open_curly_bracket(), close_curly_bracket()))
        };

        let local_stmt = || {
            recursive(|stmt| {
                let block = || block_help(stmt.clone(), open_curly_bracket(), close_curly_bracket());

                let x_statment = |name| {
                    text::keyword(name)
                        .padded()
                        .ignore_then(expr().delimited_by(open_bracket(), close_bracket()))
                        .then(block())
                };
                let if_statment = x_statment("if")
                    .then(
                        text::keyword("else")
                            .padded()
                            .ignore_then(x_statment("if"))
                            .repeated()
                            .collect::<Vec<(Expr, Vec<LStmt>)>>(),
                    )
                    .then(text::keyword("else").padded().ignore_then(block()).or_not());
                let for_loop = text::keyword("for")
                    .padded()
                    .ignore_then(
                        // TODO: accept empty parts
                        // TODO: accept declarations in first part
                        expr()
                            .or_not()
                            .then_ignore(sep())
                            .then(expr().or_not())
                            .then_ignore(sep())
                            .then(expr().or_not())
                            .delimited_by(open_bracket(), close_bracket()),
                    )
                    .then(block());
                let func_dec = || func_dec_help(stmt.clone());
                choice((
                    declaration().map(|(((ty, name), arr), exp)| LStmt::VarDec(ty, name, arr, exp)),
                    x_statment("while").map(|(cond, body)| LStmt::While(cond, body)),
                    if_statment.map(|((e1, e2), else_tail)| LStmt::Ifs(e1, e2, else_tail)),
                    for_loop.map(|(((e1, e2), e3), body)| LStmt::For(e1, e2, e3, body)),
                    func_dec().map(|(((ty, name), params), body)| {
                        LStmt::FuncDec(
                            ty,
                            name,
                            params
                                .into_iter()
                                .map(|((ty, name), arr)| (ty, name, arr))
                                .collect(),
                            body,
                        )
                    }),
                    expr().then_ignore(sep()).map(LStmt::Expr),
                ))
            })
        };
        let global_stmts = || {
            choice((
                declaration().map(|(((ty, name), arr), exp)| GStmt::VarDec(ty, name, arr, exp)),
                func_dec_help(local_stmt()).map(|(((ty, name), params), body)| {
                    GStmt::FuncDec(
                        ty,
                        name,
                        params
                            .into_iter()
                            .map(|((ty, name), arr)| (ty, name, arr))
                            .collect(),
                        body,
                    )
                }),
            ))
            .separated_by(sep().repeated())
            .allow_trailing()
            .allow_leading()
            .collect::<Vec<GStmt>>()
        };
        global_stmts().padded()
    }

    #[cfg(test)]
    mod tests {
        use chumsky::{error::EmptyErr, extra::Err, Parser};

        use super::*;

        #[test]
        fn empty_test() {
            let stmts = parser().parse("").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        #[test]
        fn whitespace_test() {
            let stmts = parser().parse(" ").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        #[test]
        fn whitespace_preceeding_test() {
            let stmts = parser().parse(" ;").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        #[test]
        fn empty_line_test() {
            let stmts = parser().parse(";").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        #[test]
        fn empty_lines_test() {
            let stmts = parser().parse(";;").into_result();
            assert_eq!(stmts, Ok(Vec::new()));
        }

        // Variable Declaration Tests
        #[test]
        fn global_char_variable_declaration_test() {
            let stmts = parser().parse("char e;").into_result();
            assert_eq!(stmts, Ok(vec![GStmt::VarDec(Type::Char, "e", None, None)]));
        }

        #[test]
        fn global_char_variable_declaration_missing_semicolon_fail_test() {
            let stmts = parser().parse("char e").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_empty_array_variable_declaration_test() {
            let stmts = parser().parse("char e[];").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(Type::Char, "e", Some(None), None)])
            );
        }

        #[test]
        fn global_char_sized_array_variable_declaration_test() {
            let stmts = parser().parse("char e[v];").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    Some(Some(Expr::Atom(Atom::Var("v")))),
                    None
                )])
            );
        }

        #[test]
        fn global_char_variable_declaration_missing_expression_fail_test() {
            let stmts = parser().parse("char e =;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_extra_identifier_fail_test() {
            let stmts = parser().parse("char e e =;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_unmatched_right_bracket_fail_test() {
            let stmts = parser().parse("char e [=;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_sized_array_variable_declaration_unmatched_left_bracket_fail_test() {
            let stmts = parser().parse("char e ]=;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_untyped_variable_assignment_fail_test() {
            let stmts = parser().parse("e = v;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_char_variable_declaration_assignment_test() {
            let stmts = parser().parse("char e = v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Atom(Atom::Var("v")))
                )])
            );
        }

        #[test]
        fn global_char_variable_declaration_assignment_missing_semicolon_fail_test() {
            let stmts = parser().parse("char e = v").into_result();
            assert!(stmts.is_err());
        }

        // Function Declaration Tests
        #[test]
        fn global_function_declaration_test() {
            let stmts = parser().parse("char foo(){}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    Vec::new()
                )])
            );
        }

        #[test]
        fn global_function_declaration_missing_return_type_test() {
            let stmts = parser().parse("foo(){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_name_test() {
            let stmts = parser().parse("char (){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_open_bracket_test() {
            let stmts = parser().parse("char foo){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_closed_bracket_test() {
            let stmts = parser().parse("char foo({}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_extra_open_bracket_test() {
            let stmts = parser().parse("char foo((){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_extra_closed_bracket_test() {
            let stmts = parser().parse("char foo()){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_open_curly_bracket_test() {
            let stmts = parser().parse("char foo()}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_missing_closed_curly_bracket_test() {
            let stmts = parser().parse("char foo(){").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_1_parameter_test() {
            let stmts = parser().parse("char foo(char a){}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    vec![(Type::Char, "a", None)],
                    Vec::new()
                )])
            );
        }

        #[test]
        fn global_function_declaration_1_parameter_trailing_comma_test() {
            let stmts = parser().parse("char foo(char a,){}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    vec![(Type::Char, "a", None)],
                    Vec::new()
                )])
            );
        }

        // failing parameters
        #[test]
        fn global_function_declaration_parameter_missing_type_fail_test() {
            let stmts = parser().parse("char foo(a){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_parameter_missing_name_fail_test() {
            let stmts = parser().parse("char foo(char){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_single_comma_fail_test() {
            let stmts = parser().parse("char foo(,){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_function_declaration_2_parameters_test() {
            let stmts = parser().parse("char foo(char a, char b){}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    vec![(Type::Char, "a", None), (Type::Char, "b", None)],
                    Vec::new()
                )])
            );
        }

        #[test]
        fn global_function_declaration_2_parameters_trailing_comma_test() {
            let stmts = parser().parse("char foo(char a, char b, ){}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    vec![(Type::Char, "a", None), (Type::Char, "b", None)],
                    Vec::new()
                )])
            );
        }

        // Bad Global Statements

        #[test]
        fn global_expression_fail_test() {
            let stmts = parser().parse("e;").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_if_fail_test() {
            let stmts = parser().parse("if(e){}else{}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_while_fail_test() {
            let stmts = parser().parse("while(e){}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn global_for_fail_test() {
            let stmts = parser().parse("for(e;e;e){}").into_result();
            assert!(stmts.is_err());
        }

        // Local Statement Tests

        #[test]
        fn local_char_variable_declaration_test() {
            let stmts = parser().parse("char foo(){char e;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::VarDec(Type::Char, "e", None, None)]
                )])
            );
        }

        #[test]
        fn local_char_variable_declaration_missing_semicolon_fail_test() {
            let stmts = parser().parse("char foo(){char e}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_char_empty_array_variable_declaration_test() {
            let stmts = parser().parse("char foo(){char e[];}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::VarDec(Type::Char, "e", Some(None), None)]
                )])
            );
        }

        #[test]
        fn local_char_sized_array_variable_declaration_test() {
            let stmts = parser().parse("char foo(){char e[v];}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::VarDec(
                        Type::Char,
                        "e",
                        Some(Some(Expr::Atom(Atom::Var("v")))),
                        None
                    )]
                )])
            );
        }

        #[test]
        fn local_char_variable_declaration_missing_expression_fail_test() {
            let stmts = parser().parse("char foo(){char e =;}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_char_sized_array_variable_declaration_extra_identifier_fail_test() {
            let stmts = parser().parse("char foo(){char e e =;}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_char_sized_array_variable_declaration_unmatched_right_bracket_fail_test() {
            let stmts = parser().parse("char foo(){char e [=;}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_char_sized_array_variable_declaration_unmatched_left_bracket_fail_test() {
            let stmts = parser().parse("char foo(){char e ]=;}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_untyped_variable_assignment_test() {
            let stmts = parser().parse("char foo(){e = v;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::Expr(Expr::Assignment(
                        Box::new(Expr::Atom(Atom::Var("e"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))]
                )])
            );
        }

        #[test]
        fn local_char_variable_declaration_assignment_test() {
            let stmts = parser().parse("char foo(){char e = v;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::VarDec(
                        Type::Char,
                        "e",
                        None,
                        Some(Expr::Atom(Atom::Var("v")))
                    )]
                )])
            );
        }

        #[test]
        fn local_char_variable_declaration_assignment_missing_semicolon_fail_test() {
            let stmts = parser().parse("char foo(){char e = v}").into_result();
            assert!(stmts.is_err());
        }

        #[test]
        fn local_empty_line_test() {
            let stmts = parser().parse("char foo(){;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    Vec::new()
                )])
            );
        }

        #[test]
        fn local_empty_lines_test() {
            let stmts = parser().parse("char foo(){;;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    Vec::new()
                )])
            );
        }

        // Local Expression Tests
        #[test]
        fn local_expression_test() {
            let stmts = parser().parse("char foo(){e;}").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::Expr(Expr::Atom(Atom::Var("e")))]
                )])
            );
        }

        #[test]
        fn local_if_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    if (e) {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::Ifs(
                        (Expr::Atom(Atom::Var("e")), Vec::new()),
                        Vec::new(),
                        None
                    )]
                )])
            );
        }

        #[test]
        fn local_if_else_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    if (e) {
                    } else {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::Ifs(
                        (Expr::Atom(Atom::Var("e")), Vec::new()),
                        Vec::new(),
                        Some(Vec::new())
                    )]
                )])
            );
        }

        #[test]
        fn local_if_else_if_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    if (e) {
                    } else if (e) {
                    } else {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::Ifs(
                        (Expr::Atom(Atom::Var("e")), Vec::new()),
                        vec![(Expr::Atom(Atom::Var("e")), Vec::new())],
                        Some(Vec::new())
                    )]
                )])
            );
        }

        #[test]
        fn local_while_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    while (e) {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::While(Expr::Atom(Atom::Var("e")), Vec::new())]
                )])
            );
        }

        #[test]
        fn local_for_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    for (i = e; i; i = i) {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::For(
                        Some(Expr::Assignment(
                            Box::new(Expr::Atom(Atom::Var("i"))),
                            Box::new(Expr::Atom(Atom::Var("e")))
                        )),
                        Some(Expr::Atom(Atom::Var("i"))),
                        Some(Expr::Assignment(
                            Box::new(Expr::Atom(Atom::Var("i"))),
                            Box::new(Expr::Atom(Atom::Var("i")))
                        )),
                        Vec::new()
                    )]
                )])
            );
        }

        #[test]
        fn local_for_empty_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    for ( ; ;) {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::For(None, None, None, Vec::new())]
                )])
            );
        }

        #[test]
        fn local_for_partially_empty_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    for ( ;e ;) {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::For(
                        None,
                        Some(Expr::Atom(Atom::Var("e"))),
                        None,
                        Vec::new()
                    )]
                )])
            );
        }

        #[test]
        fn local_function_declaration_test() {
            let stmts = parser()
                .parse(
                    r#"
                char foo(){
                    char bar() {
                    }
                }
                "#,
                )
                .into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::FuncDec(
                    Type::Char,
                    "foo",
                    Vec::new(),
                    vec![LStmt::FuncDec(Type::Char, "bar", Vec::new(), Vec::new())]
                )])
            );
        }

        #[test]
        fn numeric_literal_expression_test() {
            let stmts = parser().parse("char e = 15;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Atom(Atom::Num(15)))
                )])
            );
        }

        #[test]
        fn parenthesized_expression_test() {
            let stmts = parser().parse("char e = (v);").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Atom(Atom::Var("v")))
                )])
            );
        }

        #[test]
        fn negate_expression_test() {
            let stmts = parser().parse("char e = !v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Neg(Box::new(Expr::Atom(Atom::Var("v")))))
                )])
            );
        }

        #[test]
        fn double_negate_expression_test() {
            let stmts = parser().parse("char e = !!v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Neg(Box::new(Expr::Neg(Box::new(Expr::Atom(
                        Atom::Var("v")
                    ))))))
                )])
            );
        }

        #[test]
        fn add_expressions_test() {
            let stmts = parser().parse("char e = u + v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Add(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn multiply_expressions_test() {
            let stmts = parser().parse("char e = u * v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn less_than_expressions_test() {
            let stmts = parser().parse("char e = u < v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Lt(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn greater_than_expressions_test() {
            let stmts = parser().parse("char e = u > v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Gt(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn equal_expressions_test() {
            let stmts = parser().parse("char e = u == v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Eq(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn assign_expressions_test() {
            let stmts = parser().parse("char e = u = v;").into_result();
            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Assignment(
                        Box::new(Expr::Atom(Atom::Var("u"))),
                        Box::new(Expr::Atom(Atom::Var("v")))
                    ))
                )])
            );
        }

        #[test]
        fn negation_binds_tighter_than_multiplication() {
            let stmts = parser().parse("char e = !a * b;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Neg(Box::new(Expr::Atom(Atom::Var("a"))))),
                        Box::new(Expr::Atom(Atom::Var("b")))
                    ))
                )])
            );
        }

        #[test]
        fn double_negation_with_comparison() {
            let stmts = parser().parse("char e = !!a < b;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Lt(
                        Box::new(Expr::Neg(Box::new(Expr::Neg(Box::new(Expr::Atom(
                            Atom::Var("a")
                        )))))),
                        Box::new(Expr::Atom(Atom::Var("b")))
                    ))
                )])
            );
        }

        #[test]
        fn brackets_override_negation_binding() {
            let stmts = parser().parse("char e = !(a + b) * c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Neg(Box::new(Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )))),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn mul_has_higher_precedence_than_add() {
            let stmts = parser().parse("char e = a + b * c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Add(
                        Box::new(Expr::Atom(Atom::Var("a"))),
                        Box::new(Expr::Mul(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn bracket_overrides_mul_precedence() {
            let stmts = parser().parse("char e = (a + b) * c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn add_has_higher_precedence_than_less_than() {
            let stmts = parser().parse("char e = a + b < c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Lt(
                        Box::new(Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn bracket_overrides_comparison_precedence() {
            let stmts = parser().parse("char e = a + (b < c);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Add(
                        Box::new(Expr::Atom(Atom::Var("a"))),
                        Box::new(Expr::Lt(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn comparison_has_higher_precedence_than_equality() {
            let stmts = parser().parse("char e = a < b == c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Eq(
                        Box::new(Expr::Lt(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn bracket_overrides_equality_precedence() {
            let stmts = parser().parse("char e = a < (b == c);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Lt(
                        Box::new(Expr::Atom(Atom::Var("a"))),
                        Box::new(Expr::Eq(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn equality_has_higher_precedence_than_assignment() {
            let stmts = parser().parse("char e = a == b = c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Assignment(
                        Box::new(Expr::Eq(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn bracket_overrides_assignment_precedence() {
            let stmts = parser().parse("char e = a = (b == c);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Assignment(
                        Box::new(Expr::Atom(Atom::Var("a"))),
                        Box::new(Expr::Eq(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn addition_is_left_associative() {
            let stmts = parser().parse("char e = a + b + c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Add(
                        Box::new(Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn assignment_is_right_associative() {
            let stmts = parser().parse("char e = a = b = c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Assignment(
                        Box::new(Expr::Atom(Atom::Var("a"))),
                        Box::new(Expr::Assignment(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn no_postfix_precedence_stress_test() {
            let stmts = parser()
                .parse("char r = a = !!b + c * !d < e == !(f = g + !h);")
                .into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "r",
                    None,
                    Some(
                        // a = ...
                        Expr::Assignment(
                            Box::new(Expr::Atom(Atom::Var("a"))),
                            Box::new(
                                // (...) == !(...)
                                Expr::Eq(
                                    Box::new(
                                        // (!!b + (c * !d)) < e
                                        Expr::Lt(
                                            Box::new(Expr::Add(
                                                Box::new(Expr::Neg(Box::new(Expr::Neg(Box::new(
                                                    Expr::Atom(Atom::Var("b"))
                                                ))))),
                                                Box::new(Expr::Mul(
                                                    Box::new(Expr::Atom(Atom::Var("c"))),
                                                    Box::new(Expr::Neg(Box::new(Expr::Atom(
                                                        Atom::Var("d")
                                                    ))))
                                                ))
                                            )),
                                            Box::new(Expr::Atom(Atom::Var("e")))
                                        )
                                    ),
                                    Box::new(
                                        // !(f = g + !h)
                                        Expr::Neg(Box::new(Expr::Assignment(
                                            Box::new(Expr::Atom(Atom::Var("f"))),
                                            Box::new(Expr::Add(
                                                Box::new(Expr::Atom(Atom::Var("g"))),
                                                Box::new(Expr::Neg(Box::new(Expr::Atom(
                                                    Atom::Var("h")
                                                ))))
                                            ))
                                        )))
                                    )
                                )
                            )
                        )
                    )
                )])
            );
        }

        #[test]
        fn empty_call_test() {
            let stmts = parser().parse("char e = f();").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(Box::new(Expr::Atom(Atom::Var("f"))), vec![]))
                )])
            );
        }

        #[test]
        fn call_with_arguments_test() {
            let stmts = parser().parse("char e = f(a, b, c);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(
                        Box::new(Expr::Atom(Atom::Var("f"))),
                        vec![
                            Expr::Atom(Atom::Var("a")),
                            Expr::Atom(Atom::Var("b")),
                            Expr::Atom(Atom::Var("c"))
                        ]
                    ))
                )])
            );
        }

        #[test]
        fn call_with_negation_argument_test() {
            let stmts = parser().parse("char e = f(!a, !!b);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(
                        Box::new(Expr::Atom(Atom::Var("f"))),
                        vec![
                            Expr::Neg(Box::new(Expr::Atom(Atom::Var("a")))),
                            Expr::Neg(Box::new(Expr::Neg(Box::new(Expr::Atom(Atom::Var("b"))))))
                        ]
                    ))
                )])
            );
        }

        #[test]
        fn call_with_binary_expression_test() {
            let stmts = parser().parse("char e = f(a) + b * c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Add(
                        Box::new(Expr::Call(
                            Box::new(Expr::Atom(Atom::Var("f"))),
                            vec![Expr::Atom(Atom::Var("a"))]
                        )),
                        Box::new(Expr::Mul(
                            Box::new(Expr::Atom(Atom::Var("b"))),
                            Box::new(Expr::Atom(Atom::Var("c")))
                        ))
                    ))
                )])
            );
        }

        #[test]
        fn nested_function_call_test() {
            let stmts = parser().parse("char e = f(g(a), h(b, c));").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(
                        Box::new(Expr::Atom(Atom::Var("f"))),
                        vec![
                            Expr::Call(
                                Box::new(Expr::Atom(Atom::Var("g"))),
                                vec![Expr::Atom(Atom::Var("a"))]
                            ),
                            Expr::Call(
                                Box::new(Expr::Atom(Atom::Var("h"))),
                                vec![Expr::Atom(Atom::Var("b")), Expr::Atom(Atom::Var("c"))]
                            )
                        ]
                    ))
                )])
            );
        }

        #[test]
        fn function_call_with_brackets_test() {
            let stmts = parser().parse("char e = !(f(a) + b) * c;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Neg(Box::new(Expr::Add(
                            Box::new(Expr::Call(
                                Box::new(Expr::Atom(Atom::Var("f"))),
                                vec![Expr::Atom(Atom::Var("a"))]
                            )),
                            Box::new(Expr::Atom(Atom::Var("b")))
                        )))),
                        Box::new(Expr::Atom(Atom::Var("c")))
                    ))
                )])
            );
        }

        #[test]
        fn complex_call_expression_test() {
            let stmts = parser().parse("char e = (fs + i)(r);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(
                        Box::new(Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("fs"))),
                            Box::new(Expr::Atom(Atom::Var("i")))
                        )),
                        vec![Expr::Atom(Atom::Var("r"))]
                    ))
                )])
            );
        }

        #[test]
        fn chained_calls_test() {
            let stmts = parser().parse("char e = f()(g)(h + i);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Call(
                        Box::new(Expr::Call(
                            Box::new(Expr::Call(Box::new(Expr::Atom(Atom::Var("f"))), vec![])),
                            vec![Expr::Atom(Atom::Var("g"))]
                        )),
                        vec![Expr::Add(
                            Box::new(Expr::Atom(Atom::Var("h"))),
                            Box::new(Expr::Atom(Atom::Var("i")))
                        )]
                    ))
                )])
            );
        }

        #[test]
        fn call_vs_mul_test() {
            let stmts = parser().parse("char e = f(a) * b;").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Mul(
                        Box::new(Expr::Call(
                            Box::new(Expr::Atom(Atom::Var("f"))),
                            vec![Expr::Atom(Atom::Var("a"))]
                        )),
                        Box::new(Expr::Atom(Atom::Var("b")))
                    ))
                )])
            );
        }

        #[test]
        fn call_vs_unary_test() {
            let stmts = parser().parse("char e = !f(a);").into_result();

            assert_eq!(
                stmts,
                Ok(vec![GStmt::VarDec(
                    Type::Char,
                    "e",
                    None,
                    Some(Expr::Neg(Box::new(Expr::Call(
                        Box::new(Expr::Atom(Atom::Var("f"))),
                        vec![Expr::Atom(Atom::Var("a"))]
                    ))))
                )])
            );
        }
    }
}
