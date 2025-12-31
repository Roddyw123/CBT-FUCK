use std::{
    collections::HashMap,
    fmt::{Debug, Display},
};

use super::cast::ast::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SemType {
    UnknownType,
    KnownType(Type),
}

impl Display for SemType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SemType::UnknownType => write!(f, "Unknown type"),
            SemType::KnownType(ty) => write!(f, "{}", ty),
        }
    }
}

#[derive(PartialEq, Eq, Clone)]
pub struct QualifiedName {
    name: String,
    old_name: String,
    ty: SemType,
}

fn to_qualified_name(name: String, old_name: String, ty: SemType) -> QualifiedName {
    QualifiedName {
        name: name,
        old_name: old_name,
        ty: ty,
    }
}

impl Debug for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QualifiedName")
            .field("old name", &self.old_name)
            .field("new name", &self.name)
            .field("type", &self.ty)
            .finish()
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.old_name)
    }
}

#[derive(Debug)]
pub enum ScopedStmt {
    Global,
    FuncDec,
    Carrier, // Ifs, Elifs, and Elses
    If(
        Expr<QualifiedName>, // condition
    ),
    Elif(
        Expr<QualifiedName>, // condition
    ),
    Else,
    For(
        Option<Expr<QualifiedName>>, // intialiser
        Option<Expr<QualifiedName>>, // condition
        Option<Expr<QualifiedName>>, // updater
    ),
    While(
        Expr<QualifiedName>, // condition
    ),
}

#[derive(Debug)]
pub enum SStmt {
    ScopedStmt(
        Vec<String>, // scope keys (delays true flattening of Trie into Hashmap)
        ScopedStmt,  // Type of scoped stmt
        Vec<Self>,   // stmts inside the scope
    ),
    Stmt(Expr<QualifiedName>), // stores expressions(the only case left) // consider duplicating AST to catch undeclared variables
}

#[derive(Debug)]
pub struct Trie {
    member: Option<SemType>,
    map: HashMap<String, Self>,
}

impl Trie {
    fn get_name(&self, scope: Vec<String>) -> Option<SemType> {
        // current scope
        scope
            .first()
            .and_then(|cd| {
                self.map
                    .get(cd)
                    .map(|t| t.get_name(scope[1..].to_vec()))
                    .flatten()
            })
            .or(self.member.clone())
    }

    fn insert(&mut self, path: Vec<String>, ty: SemType) -> Option<SemType> {
        match path.first() {
            None => self.member.replace(ty),
            Some(cd) => {
                let entry = self
                    .map
                    .entry(cd.to_string())
                    .or_insert_with(|| Trie::new());
                entry.insert(path[1..].to_vec(), ty)
            }
        }
    }

    fn new() -> Self {
        Trie {
            member: None,
            map: HashMap::new(),
        }
    }
}

#[derive(Debug)]
pub struct RenamerCTX<E> {
    mapping: HashMap<String, Trie>,
    errs: Vec<E>,
    counter: u64,
    scope: Vec<String>,
}

impl<E> RenamerCTX<E> {
    fn err(&mut self, err: E) {
        self.errs.push(err);
    }
    fn get_next_ctx(&mut self) -> String {
        let tmp = self.counter.to_string();
        self.counter += 1;
        tmp
    }
    fn enter_scope(&mut self) -> &mut Self {
        let new_scope = self.get_next_ctx();
        self.scope.push(new_scope);
        self
    }
    fn exit_scope(&mut self) -> &mut Self {
        if self.scope.is_empty() {
            panic!("Exiting scope when none exist");
        }
        self.scope.pop();
        self
    }
}

impl RenamerCTX<String> {
    fn add_name(&mut self, name: String, ty: SemType) {
        let trie = self.mapping.entry(name.clone()).or_insert(Trie::new());

        if let Some(old_ty) = trie.insert(self.scope.clone(), ty) {
            self.err(format!(
                "{} is previously defined with type: {old_ty}",
                name
            ));
        }
    }

    fn get_name(&self, name: &str) -> QualifiedName {
        (|(new_name, ty)| {
            to_qualified_name(
                self.scope.clone().join("/") + new_name,
                name.to_string(),
                ty,
            )
        })(
            self.mapping
                .get(name)
                .and_then(|trie| trie.get_name(self.scope.clone()))
                .map_or(("?", SemType::UnknownType), |ty| (name, ty)),
        )
    }
}

fn new_renamer_ctx() -> RenamerCTX<String> {
    RenamerCTX {
        mapping: HashMap::new(),
        errs: Vec::new(),
        counter: 0,
        scope: Vec::new(),
    }
}

pub fn qualify_atom(atom: Atom<&str>, ctx: &RenamerCTX<String>) -> Atom<QualifiedName> {
    match atom {
        Atom::Var(name) => Atom::Var(ctx.get_name(name)),
        Atom::Array(atom, expr) => Atom::Array(
            Box::new(qualify_atom(*atom, ctx)),
            Box::new(qualify_expressions(*expr, ctx)),
        ),
        Atom::Num(n) => Atom::Num(n),
    }
}

pub fn qualify_expressions<'src>(
    stmt: Expr<&'src str>,
    ctx: &RenamerCTX<String>,
) -> Expr<QualifiedName> {
    match stmt {
        Expr::Atom(atom) => Expr::Atom(qualify_atom(atom, ctx)),
        Expr::Neg(expr) => Expr::Neg(Box::new(qualify_expressions(*expr, ctx))),
        Expr::Add(expr, expr1) => Expr::Add(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Mul(expr, expr1) => Expr::Mul(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Lt(expr, expr1) => Expr::Lt(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Gt(expr, expr1) => Expr::Gt(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Eq(expr, expr1) => Expr::Eq(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Inc(expr) => Expr::Inc(Box::new(qualify_expressions(*expr, ctx))),
        Expr::Dec(expr) => Expr::Dec(Box::new(qualify_expressions(*expr, ctx))),
        Expr::Call(expr, exprs) => Expr::Call(
            Box::new(qualify_expressions(*expr, ctx)),
            exprs
                .into_iter()
                .map(|arg| qualify_expressions(arg, ctx))
                .collect(),
        ),
        Expr::Array(expr, expr1) => Expr::Array(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
        Expr::Assignment(expr, expr1) => Expr::Assignment(
            Box::new(qualify_expressions(*expr, ctx)),
            Box::new(qualify_expressions(*expr1, ctx)),
        ),
    }
}

pub fn symbolify_lstmts<'a>(
    stmts: Vec<LStmt<&'a str>>,
    ctx: &mut RenamerCTX<String>,
) -> Vec<SStmt> {
    let mut v = Vec::new();
    for stmt in stmts {
        v.push(match stmt {
            LStmt::VarDec(ty, name, _arr_info, Some(expr1)) => {
                let expr = qualify_expressions(expr1, ctx);
                ctx.add_name(name.to_string(), SemType::KnownType(ty));
                // assign value
                Some(SStmt::Stmt(Expr::Assignment(
                    Box::new(qualify_expressions(Expr::Atom(Atom::Var(name)), ctx)),
                    Box::new(expr),
                )))
            }
            LStmt::VarDec(ty, name, _arr_info, None) => {
                ctx.add_name(name.to_string(), SemType::KnownType(ty));
                None
            }
            LStmt::FuncDec(_ty, name, items, lstmts) => {
                ctx.add_name(name.to_string(), SemType::UnknownType);
                let new_scope = ctx.enter_scope().scope.clone();
                // add argument variables into function scope
                for (arg_ty, arg_name, _arg_arr) in items {
                    ctx.add_name(arg_name.to_string(), SemType::KnownType(arg_ty));
                }
                let renamed_lstmts = symbolify_lstmts(lstmts, ctx);
                ctx.exit_scope();
                Some(SStmt::ScopedStmt(
                    new_scope,
                    ScopedStmt::FuncDec,
                    renamed_lstmts,
                ))
            }
            LStmt::For(init, cond, step, body) => {
                // TODO: change to add everything inside new new scope if init is a declaration
                // ctx.add_name(
                //     to_qualified_name(init.name.to_string(), scope.clone()),
                //     SemType::KnownType(init.ty.clone()),
                // );
                let new_scope = ctx.enter_scope().scope.clone();
                let for_stmt = ScopedStmt::For(
                    init.map(|expr| qualify_expressions(expr, ctx)),
                    cond.map(|expr| qualify_expressions(expr, ctx)),
                    step.map(|expr| qualify_expressions(expr, ctx)),
                );
                let renamed_body = symbolify_lstmts(body, ctx);
                ctx.exit_scope();
                Some(SStmt::ScopedStmt(new_scope, for_stmt, renamed_body))
            }
            LStmt::While(cond, body) => {
                let while_stmt = ScopedStmt::While(qualify_expressions(cond, ctx));
                let new_scope = ctx.enter_scope().scope.clone();
                let renamed_body = symbolify_lstmts(body, ctx);
                ctx.exit_scope();
                Some(SStmt::ScopedStmt(new_scope, while_stmt, renamed_body))
            }
            LStmt::Ifs((if_cond, if_stmts), then_branch, else_branch) => {
                let new_scope = ctx.enter_scope().scope.clone();

                // if case
                let if_stmt = ScopedStmt::If(qualify_expressions(if_cond, ctx));
                let if_scope = ctx.enter_scope().scope.clone();
                let mut v = vec![SStmt::ScopedStmt(
                    if_scope.clone(),
                    if_stmt,
                    symbolify_lstmts(if_stmts, ctx),
                )];
                ctx.exit_scope();

                // elif cases
                v = v
                    .into_iter()
                    .chain(then_branch.into_iter().map(|(elif_cond, elif_stmts)| {
                        let elif_stmt = ScopedStmt::Elif(qualify_expressions(elif_cond, ctx));
                        let elif_scope = ctx.enter_scope().scope.clone();
                        let renamed_elif_stmts = symbolify_lstmts(elif_stmts, ctx);
                        ctx.exit_scope();
                        SStmt::ScopedStmt(elif_scope, elif_stmt, renamed_elif_stmts)
                    }))
                    .collect();

                // else case
                v = v
                    .into_iter()
                    .chain(
                        else_branch
                            .map(|else_stmts| {
                                let else_scope = ctx.enter_scope().scope.clone();
                                let renamed_else_stmts = symbolify_lstmts(else_stmts, ctx);
                                ctx.exit_scope();
                                SStmt::ScopedStmt(else_scope, ScopedStmt::Else, renamed_else_stmts)
                            })
                            .into_iter(),
                    )
                    .collect();

                ctx.exit_scope();
                Some(SStmt::ScopedStmt(new_scope.clone(), ScopedStmt::Carrier, v))
            }
            LStmt::Expr(expr) => Some(SStmt::Stmt(qualify_expressions(expr, ctx))),
        });
    }
    v.into_iter().flat_map(|x| x).collect()
}

pub fn symbolify<'src>(stmts: Vec<GStmt<&'src str>>) -> (SStmt, RenamerCTX<String>) {
    let mut ctx = new_renamer_ctx();
    let mut v = Vec::new();
    // for loop to dodge closure taking mapping reference
    for stmt in stmts {
        v.push(match stmt {
            GStmt::VarDec(ty, name, _arr_info, Some(expr1)) => {
                let expr = qualify_expressions(expr1, &ctx);
                ctx.add_name(name.to_string(), SemType::KnownType(ty));
                Some(SStmt::Stmt(Expr::Assignment(
                    Box::new(qualify_expressions(Expr::Atom(Atom::Var(name)), &ctx)),
                    Box::new(expr),
                )))
            }
            GStmt::VarDec(ty, name, _arr_info, None) => {
                ctx.add_name(name.to_string(), SemType::KnownType(ty));
                None
            }
            GStmt::FuncDec(ty, name, items, lstmts) => {
                ctx.add_name(name.to_string(), SemType::KnownType(ty.clone()));
                ctx.enter_scope();
                let scope = ctx.scope.clone();
                // add argument variables into function scope
                for (arg_ty, arg_name, _arg_arr) in items {
                    ctx.add_name(arg_name.to_string(), SemType::KnownType(arg_ty.clone()));
                }
                let renamed_lstmts = symbolify_lstmts(lstmts, &mut ctx);
                ctx.exit_scope();
                Some(SStmt::ScopedStmt(
                    scope,
                    ScopedStmt::FuncDec,
                    renamed_lstmts,
                ))
            }
        });
    }
    (
        SStmt::ScopedStmt(
            Vec::new(),
            ScopedStmt::Global,
            v.into_iter().flat_map(|x| x).collect(),
        ),
        ctx,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn same_variable_global_scope_test() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::VarDec(Type::Char, "x", None, None),
        ]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_ne!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn allow_shadowing_test() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::FuncDec(
                Type::Fn(Box::new(Type::Char), vec![Type::Int]),
                "foo",
                vec![(Type::Int, "x", None)],
                vec![],
            ),
        ]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_eq!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn disallow_shadowing_function_parameters() {
        let (stmts, map) = symbolify(vec![GStmt::FuncDec(
            Type::Fn(Box::new(Type::Char), vec![Type::Int]),
            "foo",
            vec![(Type::Int, "x", None)],
            vec![LStmt::VarDec(
                Type::Int,
                "x",
                None,
                Some(Expr::Atom(Atom::Num(5))),
            )],
        )]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_ne!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn allow_shadowing_from_function_body_test() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::FuncDec(
                Type::Fn(Box::new(Type::Char), vec![Type::Int]),
                "foo",
                vec![],
                vec![LStmt::VarDec(
                    Type::Int,
                    "x",
                    None,
                    Some(Expr::Atom(Atom::Num(5))),
                )],
            ),
        ]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_eq!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn allow_shadowing_from_if_expression_test() {
        let (stmts, map) = symbolify(vec![GStmt::FuncDec(
            Type::Fn(Box::new(Type::Char), vec![Type::Int]),
            "foo",
            vec![],
            vec![
                LStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
                LStmt::Ifs(
                    (
                        Expr::Atom(Atom::Num(1)),
                        vec![LStmt::VarDec(
                            Type::Int,
                            "x",
                            None,
                            Some(Expr::Atom(Atom::Num(5))),
                        )],
                    ),
                    vec![],
                    None,
                ),
            ],
        )]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_eq!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn allow_shadowing_from_while_loop_test() {
        let (stmts, map) = symbolify(vec![GStmt::FuncDec(
            Type::Fn(Box::new(Type::Char), vec![Type::Int]),
            "foo",
            vec![],
            vec![
                LStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
                LStmt::While(
                    Expr::Atom(Atom::Num(1)),
                    vec![LStmt::VarDec(
                        Type::Int,
                        "x",
                        None,
                        Some(Expr::Atom(Atom::Num(5))),
                    )],
                ),
            ],
        )]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_eq!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn allow_shadowing_from_for_loop_test() {
        let (stmts, map) = symbolify(vec![GStmt::FuncDec(
            Type::Fn(Box::new(Type::Char), vec![Type::Int]),
            "foo",
            vec![],
            vec![
                LStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
                LStmt::For(
                    Some(Expr::Atom(Atom::Num(1))),
                    Some(Expr::Atom(Atom::Num(2))),
                    None,
                    vec![LStmt::VarDec(
                        Type::Int,
                        "x",
                        None,
                        Some(Expr::Atom(Atom::Num(5))),
                    )],
                ),
            ],
        )]);
        // println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_eq!(map.errs, Vec::<String>::new());
    }

    #[test]
    fn integration_test() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::VarDec(Type::Char, "x", None, Some(Expr::Atom(Atom::Num(1)))),
            GStmt::FuncDec(
                Type::Fn(Box::new(Type::Char), vec![Type::Int]),
                "foo",
                vec![(Type::Int, "a", None)],
                vec![
                    LStmt::Expr(Expr::Atom(Atom::Var("a"))),
                    LStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(10)))),
                ],
            ),
            GStmt::VarDec(Type::Char, "y", None, None),
        ]);
        println!("{:#?}", stmts);
        println!("{:#?}", map);
        assert_ne!(map.errs, Vec::<String>::new())
    }

    #[test]
    fn integration_test_1() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::VarDec(Type::Char, "x", None, None),
            GStmt::FuncDec(
                Type::Fn(Box::new(Type::Char), vec![Type::Int]),
                "foo",
                vec![(Type::Int, "a", None)],
                vec![
                    LStmt::Expr(Expr::Atom(Atom::Var("a"))),
                    LStmt::VarDec(Type::Int, "x", None, None),
                    LStmt::Ifs(
                        (
                            Expr::Atom(Atom::Var("x")),
                            vec![LStmt::Expr(Expr::Atom(Atom::Num(10)))],
                        ),
                        vec![],
                        None,
                    ),
                    LStmt::Ifs(
                        (Expr::Atom(Atom::Var("x")), vec![]),
                        vec![(
                            Expr::Atom(Atom::Num(20)),
                            vec![LStmt::Expr(Expr::Atom(Atom::Num(30)))],
                        )],
                        None,
                    ),
                ],
            ),
            GStmt::VarDec(Type::Char, "y", None, None),
        ]);
        println!("{:#?}", stmts);
        println!("{:#?}", map);
    }
}
