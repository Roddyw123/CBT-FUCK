mod localop;

pub mod bf2c {
    use indoc::indoc;

    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum BfSymbol {
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
