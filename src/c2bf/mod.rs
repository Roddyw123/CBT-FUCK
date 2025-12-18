pub mod c2bf {

    use chumsky::{Parser, error::Cheap, extra::{Err, ParserExtra}, input::Input, prelude::{empty}};

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
        empty().to(Vec::new())
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
    }
}