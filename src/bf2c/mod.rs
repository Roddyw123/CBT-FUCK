mod localop;

pub mod bf2c {
    use std::fmt::Display;

    use indoc::indoc;

    #[derive(Clone)]
    pub enum Expr {
        Num(i32),
        Var(String),
        Assignment(
            Box<Self>, // lhs
            Box<Self>, // rhs
        ),
        PlusEq(
            Box<Self>, // lhs
            Box<Self>, // rhs
        ),
        MinEq(
            Box<Self>, // lhs
            Box<Self>, // rhs
        ),
        Deref(Box<Self>),
        Inc(Box<Self>),
        Dec(Box<Self>),
        Call(
            Box<Self>, // funcion
            Vec<Self>, // args
        ),
        Array(
            Box<Self>, // array
            Box<Self>, // indedx
        )
    }

    #[derive(Clone)]
    pub enum Stmt {
        While(
            Expr, // condition
            Stmts, // body
        ),
        Expr(Expr),
    }

    #[derive(Clone)]
    pub struct Stmts {
        pub(crate) stmts: Vec<Stmt>
    }

    impl Display for Expr {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Expr::Num(n) => write!(f, "({})", n),
                Expr::Var(v) => write!(f, "({})", v),
                Expr::Assignment(expr, expr1) => write!(f, "({}={})", expr, expr1),
                Expr::Deref(expr) => write!(f, "(*{})", expr),
                Expr::Inc(expr) => write!(f, "({}++)", expr),
                Expr::Dec(expr) => write!(f, "({}--)", expr),
                Expr::Call(expr, exprs) => write!(f, "({}({})", expr, exprs.into_iter().map(|exp| exp.to_string()).collect::<Vec<_>>().join(", ")),
                Expr::Array(expr, expr1) => write!(f, "({}[{}])", expr, expr1),
                Expr::PlusEq(expr, expr1) => write!(f, "({}+={})", expr, expr1),
                Expr::MinEq(expr, expr1) => write!(f, "({}-={})", expr, expr1),
            }
        }
    }

    impl Display for Stmt {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            match self {
                Stmt::While(expr, stmts) => write!(f, "while({}) {{{}}}", expr.to_string(), stmts.to_string().lines().map(|line| "  ".to_string()+line).collect::<Vec<_>>().join("\n")),
                Stmt::Expr(expr) => write!(f, "{};", expr.to_string()),
            }
        }
    }
    impl Display for Stmts {
        fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
            write!(f, "{}", self.stmts.clone().into_iter().map(|sym| sym.to_string()).collect::<Vec<String>>().join("\n"))
        }
    }

    pub trait Emmitable<S> {
        fn emit(&self) -> S;
    }

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    pub enum BfSymbol {
        Left,
        Right,
        Plus,
        Minus,
        Period,
        Comma,
        OpenBracket,
        CloseBracket,
    }
    fn parse_without_verification(buf: &str) -> Vec<BfSymbol> {
        parse(buf, false).unwrap()
    }
    fn parse(buf: &str, verify: bool) -> Result<Vec<BfSymbol>, &'static str> {
        let mut out = Vec::new();
        let mut bracket_depth = 0;
        for c in buf.trim().chars() {
            match c {
                '<' => out.push(BfSymbol::Left),
                '>' => out.push(BfSymbol::Right),
                '+' => out.push(BfSymbol::Plus),
                '-' => out.push(BfSymbol::Minus),
                '.' => out.push(BfSymbol::Period),
                ',' => out.push(BfSymbol::Comma),
                '[' => {out.push(BfSymbol::OpenBracket);
                    if verify {
                        bracket_depth += 1;
                    }
                },
                ']' => {out.push(BfSymbol::CloseBracket);
                    if verify {
                        if bracket_depth == 0 {
                            return Err("missing open bracket");
                        }
                        bracket_depth -= 1;
                    }
                },
                _ => {} // ignore non-BF characters
            }
        }
        if bracket_depth != 0 {
            return Err("Brainfuck code is not well-formed (Brackets do not match)");
        }
        Ok(out)
    }

    struct BfSymbols {
        symbols: Vec<BfSymbol>
    }

    impl Emmitable<Expr> for BfSymbol {
        fn emit(&self) -> Expr {
            match self {
                BfSymbol::Left => Expr::Dec(Box::new(Expr::Var("ptr".to_string()))),
                BfSymbol::Right => Expr::Inc(Box::new(Expr::Var("ptr".to_string()))),
                BfSymbol::Plus => Expr::Inc(Box::new(Expr::Deref(Box::new(Expr::Var("ptr".to_string()))))),
                BfSymbol::Minus => Expr::Dec(Box::new(Expr::Deref(Box::new(Expr::Var("ptr".to_string()))))),
                BfSymbol::Period => Expr::Call(Box::new(Expr::Var("putchar".to_string())), vec![Expr::Deref(Box::new(Expr::Var("ptr".to_string())))]),
                BfSymbol::Comma => Expr::Assignment(Box::new(Expr::Deref(Box::new(Expr::Var("ptr".to_string())))), Box::new(Expr::Call(Box::new(Expr::Var("getchar".to_string())), Vec::new()))),
                _ => panic!("Impossible")
            }
        }
    }

    impl Emmitable<Stmts> for BfSymbols {
        fn emit(&self) -> Stmts {
            let mut result = Vec::new();
            let mut stack = Vec::new();
            for symbol in self.symbols.as_slice() {
                if let sym @ (BfSymbol::OpenBracket | BfSymbol::CloseBracket) = symbol {
                    match sym {
                        BfSymbol::OpenBracket => {
                            stack.push(result);
                            result = Vec::new();
                            continue;
                        },
                        BfSymbol::CloseBracket => {
                            let tmp = Stmt::While(Expr::Deref(Box::new(Expr::Var("ptr".to_string()))), Stmts { stmts: result });
                            result = stack.pop().unwrap();
                            result.push(tmp);
                            continue;
                        },
                        _ => panic!("Impossible")
                    }
                }
                result.push(Stmt::Expr(symbol.emit()));
            }
            Stmts { stmts: result }
        }
    }

    fn wrap_boilerplate(code: String) -> String {
        let boilerplate = String::from(indoc! {
            "#include <stdio.h>
             int main() {
                char tape[200000];
                for (int i = 0; i < 200000; i++) tape[i] = 0;
                char *ptr = tape;
            "
        });

        let boilerplate_end = String::from(indoc! {
            "   return 0;
             }
            "
        });
        format!("{}{}{}", boilerplate, code, boilerplate_end)
    }

    fn emit(tokens: &Vec<BfSymbol>) -> String {
        wrap_boilerplate(emit_without_boilerplate(tokens))
    }

    fn emit_without_boilerplate(tokens: &Vec<BfSymbol>) -> String {
        use std::fmt::Write;
        let mut out = String::new();
        let indent = " ".repeat(4);
        let mut indent_depth = 1; // core code is inside int main()

        for token in tokens {
            match token {
                BfSymbol::Left => {
                    writeln!(&mut out, "{}ptr++;", indent.repeat(indent_depth), ).unwrap();
                }
                BfSymbol::Right => {
                    writeln!(&mut out, "{}ptr--;", indent.repeat(indent_depth), ).unwrap();
                }
                BfSymbol::Plus => {
                    writeln!(&mut out, "{}(*ptr)++;", indent.repeat(indent_depth)).unwrap();
                }
                BfSymbol::Minus => {
                    writeln!(&mut out, "{}(*ptr)--;", indent.repeat(indent_depth)).unwrap();
                }
                BfSymbol::Period => {
                    writeln!(&mut out, "{}putchar(*ptr);", indent.repeat(indent_depth)).unwrap();
                }
                BfSymbol::Comma => {
                    writeln!(&mut out, "{}*ptr = getchar();", indent.repeat(indent_depth)).unwrap();
                }
                BfSymbol::OpenBracket => {
                    writeln!(&mut out, "{}while (*ptr) {{", indent.repeat(indent_depth)).unwrap();
                    indent_depth += 1;
                }
                BfSymbol::CloseBracket => {
                    indent_depth -= 1;
                    writeln!(&mut out, "{}}}", indent.repeat(indent_depth)).unwrap();
                }
            }
        }
        out
    }

    pub fn bf2cify(input: String) -> Result<String, String>{
        let parsed = parse(input.as_str(), true)?;
        Ok(emit(&parsed))
    }


    #[cfg(test)]
    mod tests {
        use indoc::indoc;
        use super::{BfSymbol, parse_without_verification, parse, emit, emit_without_boilerplate};
        #[test]
        fn parse_empty() {
            assert!(parse_without_verification("").is_empty());
        }

        #[test]
        fn parse_only_bf() {
            let tokens = parse_without_verification("<>+-.,[]");
            assert_eq!(tokens.len(), 8);
            assert_eq!(tokens[0], BfSymbol::Left);
            assert_eq!(tokens[1], BfSymbol::Right);
            assert_eq!(tokens[2], BfSymbol::Plus);
            assert_eq!(tokens[3], BfSymbol::Minus);
            assert_eq!(tokens[4], BfSymbol::Period);
            assert_eq!(tokens[5], BfSymbol::Comma);
            assert_eq!(tokens[6], BfSymbol::OpenBracket);
            assert_eq!(tokens[7], BfSymbol::CloseBracket);
        }

        #[test]
        fn parse_non_bf() {
            let tokens = parse_without_verification("abcdefg[]");
            assert_eq!(tokens.len(), 2);
            assert_eq!(tokens[0], BfSymbol::OpenBracket);
            assert_eq!(tokens[1], BfSymbol::CloseBracket);
        }

        #[test]
        fn parse_missing_open_bracket() {
            let tokens = parse("]", true);
            assert!(tokens.is_err())
        }

        #[test]
        fn parse_missing_close_bracket() {
            let tokens = parse("[", true);
            assert!(tokens.is_err())
        }

        #[test]
        fn emit_empty_program() {
            let tokens: Vec<BfSymbol> = vec![];
            let expected= indoc ! {
                "#include <stdio.h>
                 int main() {
                    char tape[200000];
                    for (int i = 0; i < 200000; i++) tape[i] = 0;
                    char *ptr = tape;
                    return 0;
                 }
                 "
                };
            assert_eq!(emit(&tokens), expected);
        }

        fn trim_leading_spaces(s: String) -> String {
            s.lines().map(|l| l.trim_start()).collect::<Vec<_>>().join("\n") + "\n"
        }

        #[test]
        fn emit_symbols_correctly() {
            let tokens: Vec<BfSymbol> = vec![
                BfSymbol::Left,
                BfSymbol::Right,
                BfSymbol::Plus,
                BfSymbol::Minus,
                BfSymbol::Period,
                BfSymbol::Comma,
                BfSymbol::OpenBracket,
                BfSymbol::CloseBracket,
            ];
            let expected = indoc! {"
                 ptr++;
                 ptr--;
                 (*ptr)++;
                 (*ptr)--;
                 putchar(*ptr);
                 *ptr = getchar();
                 while (*ptr) {
                 }
            "
            };
            assert_eq!(trim_leading_spaces(emit_without_boilerplate(&tokens)), expected);
        }

        #[test]
        fn emit_nested_while_loops_indentation() {
            // BF: [ [ + ] - ]
            let tokens: Vec<BfSymbol> = vec![
                BfSymbol::OpenBracket,
                BfSymbol::OpenBracket,
                BfSymbol::Plus,
                BfSymbol::CloseBracket,
                BfSymbol::Minus,
                BfSymbol::CloseBracket,
            ];

            let expected =
"    while (*ptr) {
        while (*ptr) {
            (*ptr)++;
        }
        (*ptr)--;
    }
";
            assert_eq!(emit_without_boilerplate(&tokens), expected);
        }
    }
}
