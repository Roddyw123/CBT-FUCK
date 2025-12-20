pub mod ast {
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Type {
        Char,
        Int,
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Atom<'src> {
        Num(u32),
        Var(&'src str),
        Array(Box<Self>, Box<Expr<'src>>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Expr<'src> {
        Atom(Atom<'src>),
        Neg(Box<Self>),
        Add(Box<Self>, Box<Self>),
        Mul(Box<Self>, Box<Self>),
        Le(Box<Self>, Box<Self>),
        Ge(Box<Self>, Box<Self>),
        Eq(Box<Self>, Box<Self>),
        Inc(Box<Self>),
        Dec(Box<Self>),

        Call(&'src str, Vec<Self>),
        Assignment(Atom<'src>, Box<Self>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum LStmt<'src> {
        VarDec(
            Type,
            &'src str,
            Option<Option<Expr<'src>>>,
            Option<Expr<'src>>,
        ),
        While(Expr<'src>, Vec<Self>),
        Ifs(
            (Expr<'src>, Vec<Self>),
            Vec<(Expr<'src>, Vec<Self>)>,
            Option<Vec<Self>>,
        ),
        For(
            Option<Expr<'src>>,
            Option<Expr<'src>>,
            Option<Expr<'src>>,
            Vec<Self>,
        ),
        FuncDec(
            Type,
            &'src str,
            Vec<(Type, &'src str, Option<Option<Expr<'src>>>)>,
            Vec<Self>,
        ),
        Expr(Expr<'src>),
    }

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum GStmt<'src> {
        VarDec(
            Type,
            &'src str,
            Option<Option<Expr<'src>>>,
            Option<Expr<'src>>,
        ),
        FuncDec(
            Type,
            &'src str,
            Vec<(Type, &'src str, Option<Option<Expr<'src>>>)>,
            Vec<LStmt<'src>>,
        ),
    }
}
