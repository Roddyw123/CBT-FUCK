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

    pub fn parser<'src, I: Input<'src>, E: ParserExtra<'src, I>>() -> impl Parser<'src, &'src str, Vec<GStmt<'src>>, Err<Cheap>> {
       // not mapped to Var immediatly as it can be a function as well
        let ident = text::ascii::ident()
            .padded();

        let num = text::int(10)
            .padded();

        let sep = just(';').padded();

        let open_bracket = just('(').padded();
        let close_bracket = just(')').padded();
        let open_square_bracket = just('[').padded();
        let close_square_bracket = just(']').padded();
        
        let expr = recursive(|expr| {

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

        let types = choice((
            keyword("char").to(Type::Char),
            keyword("int").to(Type::Int)
        ));

        let typed_variable = types.clone()
            .then(ident)
            .then(
                // hard coded 1D array
                expr.clone()
                .or_not()
                .delimited_by(open_square_bracket, close_square_bracket)
                .or_not()
            );
        
        let declaration = typed_variable.clone()
            .then(
                just('=').padded()
                .ignore_then(expr.clone())
                .or_not()
            )
            .then_ignore(sep);

        let global_stmts = choice((
            declaration.clone()
                .map(|(((ty, name), arr), exp)|
                    GStmt::VarDec(ty, name, arr, exp)),
        ))
        .repeated()
        .collect::<Vec<GStmt>>();
        global_stmts
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
    }
}