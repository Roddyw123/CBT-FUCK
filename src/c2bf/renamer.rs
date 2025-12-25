use std::{
    collections::HashMap,
    fmt::{write, Debug, Display},
    hash::Hash,
};

use super::cast::ast::*;

#[derive(Debug, Clone)]
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

#[derive(PartialEq, Eq, Hash, Clone)]
pub struct QualifiedName {
    name: String,
    path: Vec<String>,
}

fn to_qualified_name(name: String, path: Vec<String>) -> QualifiedName {
    QualifiedName {
        name: name,
        path: path,
    }
}

impl Debug for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("QualifiedName")
            .field("parts", &self.path.join("/"))
            .finish()
    }
}

impl Display for QualifiedName {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.path.last().unwrap().to_string())
    }
}

#[derive(Debug)]
pub enum ScopedStmt<'src> {
    Global,
    FuncDec,
    Carrier, // Ifs, Elifs, and Elses
    If(
        Expr<'src>, // condition
    ),
    Elif(
        Expr<'src>, // condition
    ),
    Else,
    For(
        Option<Expr<'src>>, // intialiser
        Option<Expr<'src>>, // condition
        Option<Expr<'src>>, // updater
    ),
    While(
        Expr<'src>, // condition
    ),
}

#[derive(Debug)]
pub enum SStmt<'src> {
    ScopedStmt(
        Vec<String>,      // scope keys
        ScopedStmt<'src>, // Type of scoped stmt
        Vec<Self>,        // stmts inside the scope
                          // HashMap<QualifiedName, SemType>, // symbol table (or should a version be passed around?)
    ),
    Stmt(Expr<'src>), // stores expressions(the only case left)
}

#[derive(Debug)]
pub struct Trie {
    member: Option<SemType>,
    map: HashMap<String, Self>,
}

impl Trie {
    fn get_name(&mut self, scope: Vec<String>) -> Option<SemType> {
        // current scope
        self.member
            .clone()
            // child scopes
            .or(scope.first().and_then(|cd| {
                self.map
                    .get_mut(cd)
                    .map(|t| t.get_name(scope[1..].to_vec()))
                    .flatten()
            }))
    }

    fn insert(&mut self, path: Vec<String>, ty: SemType) {
        match path.first() {
            None => {
                self.member = Some(ty);
            }
            Some(cd) => {
                let entry = self
                    .map
                    .entry(cd.to_string())
                    .or_insert_with(|| Trie::empty());
                entry.insert(path[1..].to_vec(), ty);
            }
        }
    }

    fn empty() -> Self {
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
}

impl<E> RenamerCTX<E> {
    fn err(&mut self, err: E) {
        self.errs.push(err);
    }
}

impl RenamerCTX<String> {
    fn add_name(&mut self, name: QualifiedName, ty: SemType) {
        let trie = self
            .mapping
            .entry(name.name.clone())
            .or_insert(Trie::empty());

        if let Some(old_ty) = trie.get_name(name.path.clone()) {
            trie.insert(name.path, ty);
            self.err(format!(
                "{} is previously defined with type: {old_ty}",
                name.name
            ));
        } else {
            trie.insert(name.path, ty);
        }
    }
    fn get_next_ctx(&mut self) -> String {
        let tmp = self.counter.to_string();
        self.counter += 1;
        tmp
    }
}

fn new_renamer_ctx() -> RenamerCTX<String> {
    RenamerCTX {
        mapping: HashMap::new(),
        errs: Vec::new(),
        counter: 0,
    }
}

pub fn symbolify_lstmts<'a>(
    stmts: Vec<LStmt<'a>>,
    ctx: &mut RenamerCTX<String>,
    scope: Vec<String>,
) -> Vec<SStmt<'a>> {
    let mut v = Vec::new();
    for stmt in stmts {
        v.push(match stmt {
            LStmt::VarDec(ty, name, _arr_info, Some(expr1)) => {
                println!("local var dec");
                ctx.add_name(
                    to_qualified_name(name.to_string(), scope.clone()),
                    SemType::KnownType(ty),
                );
                // assign value
                Some(SStmt::Stmt(Expr::Assignment(
                    Box::new(Expr::Atom(Atom::Var(name))),
                    Box::new(expr1),
                )))
            }
            LStmt::VarDec(ty, name, _arr_info, None) => {
                ctx.add_name(
                    to_qualified_name(name.to_string(), scope.clone()),
                    SemType::KnownType(ty),
                );
                None
            }
            LStmt::FuncDec(_ty, name, items, lstmts) => {
                ctx.add_name(
                    to_qualified_name(name.to_string(), scope.clone()),
                    SemType::UnknownType,
                );
                let mut new_scope = scope.clone();
                new_scope.push(name.to_string());
                // add argument variables into function scope
                for (arg_ty, arg_name, _arg_arr) in items {
                    let arg_qname = to_qualified_name(arg_name.to_string(), new_scope.clone());
                    ctx.add_name(arg_qname.clone(), SemType::KnownType(arg_ty.clone()));
                }
                Some(SStmt::ScopedStmt(
                    new_scope.clone(),
                    ScopedStmt::FuncDec,
                    symbolify_lstmts(lstmts, ctx, new_scope),
                ))
            }
            LStmt::For(init, cond, step, body) => {
                // TODO: change to add everything inside new new scope if init is a declaration
                // ctx.add_name(
                //     to_qualified_name(init.name.to_string(), scope.clone()),
                //     SemType::KnownType(init.ty.clone()),
                // );
                let mut new_scope = scope.clone();
                new_scope.push(ctx.get_next_ctx());
                Some(SStmt::ScopedStmt(
                    new_scope.clone(),
                    ScopedStmt::For(init, cond, step),
                    symbolify_lstmts(body, ctx, new_scope),
                ))
            }
            LStmt::While(cond, body) => {
                let mut new_scope = scope.clone();
                new_scope.push(ctx.get_next_ctx());
                Some(SStmt::ScopedStmt(
                    new_scope.clone(),
                    ScopedStmt::While(cond),
                    symbolify_lstmts(body, ctx, new_scope),
                ))
            }
            LStmt::Ifs((if_cond, if_stmts), then_branch, else_branch) => {
                let mut new_scope = scope.clone();
                new_scope.push(ctx.get_next_ctx());

                // if case
                let mut if_scope = new_scope.clone();
                if_scope.push(ctx.get_next_ctx());
                let mut v = vec![SStmt::ScopedStmt(
                    if_scope.clone(),
                    ScopedStmt::If(if_cond),
                    symbolify_lstmts(if_stmts, ctx, if_scope),
                )];

                // elif cases
                v = v
                    .into_iter()
                    .chain(then_branch.into_iter().map(|(elif_cond, elif_stmts)| {
                        let mut elif_scope = new_scope.clone();
                        elif_scope.push(ctx.get_next_ctx());
                        SStmt::ScopedStmt(
                            elif_scope.clone(),
                            ScopedStmt::Elif(elif_cond),
                            symbolify_lstmts(elif_stmts, ctx, elif_scope),
                        )
                    }))
                    .collect();

                // else case
                v = v
                    .into_iter()
                    .chain(
                        else_branch
                            .map(|else_stmts| {
                                let mut else_scope = new_scope.clone();
                                else_scope.push(ctx.get_next_ctx());
                                SStmt::ScopedStmt(
                                    else_scope.clone(),
                                    ScopedStmt::Else,
                                    symbolify_lstmts(else_stmts, ctx, else_scope),
                                )
                            })
                            .into_iter(),
                    )
                    .collect();

                Some(SStmt::ScopedStmt(new_scope.clone(), ScopedStmt::Carrier, v))
            }
            LStmt::Expr(expr) => Some(SStmt::Stmt(expr)),
        });
    }
    v.into_iter().flat_map(|x| x).collect()
}

pub fn symbolify(stmts: Vec<GStmt>) -> (SStmt, RenamerCTX<String>) {
    let mut ctx = new_renamer_ctx();
    let mut v = Vec::new();
    // for loop to dodge closure taking mapping reference
    for stmt in stmts {
        v.push(match stmt {
            GStmt::VarDec(ty, name, _arr_info, Some(expr1)) => {
                ctx.add_name(
                    to_qualified_name(name.to_string(), vec![]),
                    SemType::KnownType(ty),
                );
                Some(SStmt::Stmt(Expr::Assignment(
                    Box::new(Expr::Atom(Atom::Var(name))),
                    Box::new(expr1),
                )))
            }
            GStmt::VarDec(ty, name, _arr_info, None) => {
                ctx.add_name(
                    to_qualified_name(name.to_string(), vec![]),
                    SemType::KnownType(ty),
                );
                None
            }
            GStmt::FuncDec(ty, name, items, lstmts) => {
                ctx.add_name(
                    to_qualified_name(name.to_string(), vec![]),
                    SemType::KnownType(ty.clone()),
                );
                let scope = vec![ctx.get_next_ctx()];
                // add argument variables into function scope
                for (arg_ty, arg_name, _arg_arr) in items {
                    let arg_qname = to_qualified_name(arg_name.to_string(), scope.clone());
                    ctx.add_name(arg_qname.clone(), SemType::KnownType(arg_ty.clone()));
                }
                Some(SStmt::ScopedStmt(
                    scope.clone(),
                    ScopedStmt::FuncDec,
                    symbolify_lstmts(lstmts, &mut ctx, scope),
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
    fn integration_test() {
        let (stmts, map) = symbolify(vec![
            GStmt::VarDec(Type::Int, "x", None, Some(Expr::Atom(Atom::Num(5)))),
            GStmt::VarDec(Type::Char, "x", None, None),
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
