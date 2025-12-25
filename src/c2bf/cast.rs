pub mod ast {
    use std::fmt::Display;

    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum Type {
        Char,
        Int,
        Fn(Box<Type>, Vec<Type>),
    }

    impl Display for Type {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Type::Char => write!(f, "char"),
                Type::Int => write!(f, "int"),
                Type::Fn(ret, args) => {
                    write!(f, "fn(")?;
                    for (i, arg) in args.iter().enumerate() {
                        write!(f, "{}", arg)?;
                        if i != args.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }
                    write!(f, ") -> {}", ret)
                }
            }
        }
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
        Lt(Box<Self>, Box<Self>),
        Gt(Box<Self>, Box<Self>),
        Eq(Box<Self>, Box<Self>),
        Inc(Box<Self>),
        Dec(Box<Self>),

        Call(Box<Self>, Vec<Self>),
        Array(Box<Self>, Box<Self>),
        Assignment(Box<Self>, Box<Self>),
    }

    impl Display for Expr<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Expr::Atom(atom) => write!(f, "{:?}", atom),
                Expr::Neg(expr) => write!(f, "-({})", expr),
                Expr::Add(lhs, rhs) => write!(f, "({}) + ({})", lhs, rhs),
                Expr::Mul(lhs, rhs) => write!(f, "({}) * ({})", lhs, rhs),
                Expr::Lt(lhs, rhs) => write!(f, "({}) < ({})", lhs, rhs),
                Expr::Gt(lhs, rhs) => write!(f, "({}) > ({})", lhs, rhs),
                Expr::Eq(lhs, rhs) => write!(f, "({}) == ({})", lhs, rhs),
                Expr::Inc(expr) => write!(f, "++({})", expr),
                Expr::Dec(expr) => write!(f, "--({})", expr),
                Expr::Call(func, args) => {
                    write!(f, "{}(", func)?;
                    for (i, arg) in args.iter().enumerate() {
                        write!(f, "{}", arg)?;
                        if i != args.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }
                    write!(f, ")")
                }
                Expr::Array(array, index) => write!(f, "{}[{}]", array, index),
                Expr::Assignment(lhs, rhs) => write!(f, "{} = {}", lhs, rhs),
            }
        }
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

    impl Display for LStmt<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                LStmt::VarDec(ty, name, arr, exp) => {
                    write!(f, "{:?} {}", ty, name)?;
                    if let Some(arr) = arr {
                        if let Some(size) = arr {
                            write!(f, "[{}]", size)?;
                        } else {
                            write!(f, "[]")?;
                        }
                    }
                    if let Some(exp) = exp {
                        write!(f, " = {}", exp)?;
                    }
                    write!(f, ";")
                }
                LStmt::While(cond, body) => {
                    write!(f, "while ({}) {{\n", cond)?;
                    for stmt in body {
                        write!(f, "    {}\n", stmt)?;
                    }
                    write!(f, "}}")
                }
                LStmt::Expr(expr) => write!(f, "{};", expr),
                _ => write!(f, "// Unsupported statement for display"),
            }
        }
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

    impl Display for GStmt<'_> {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                GStmt::VarDec(ty, name, arr, exp) => {
                    write!(f, "{:?} {}", ty, name)?;
                    if let Some(arr) = arr {
                        if let Some(size) = arr {
                            write!(f, "[{}]", size)?;
                        } else {
                            write!(f, "[]")?;
                        }
                    }
                    if let Some(exp) = exp {
                        write!(f, " = {}", exp)?;
                    }
                    write!(f, ";")
                }
                GStmt::FuncDec(ty, name, params, body) => {
                    write!(f, "{:?} {}(", ty, name)?;
                    for (i, (pty, pname, parr)) in params.iter().enumerate() {
                        write!(f, "{:?} {}", pty, pname)?;
                        if let Some(parr) = parr {
                            if let Some(size) = parr {
                                write!(f, "[{}]", size)?;
                            } else {
                                write!(f, "[]")?;
                            }
                        }
                        if i != params.len() - 1 {
                            write!(f, ", ")?;
                        }
                    }
                    write!(f, ") {{\n")?;
                    for stmt in body {
                        write!(f, "    {}\n", stmt)?;
                    }
                    write!(f, "}}")
                }
            }
        }
    }
}
