pub mod bf2c {
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

    // fn emit(tokens: &Vec<BfSymbol>, format: ) -> String {
    //     let mut out = String::new();
    //     if tokens.contains(&BfSymbol::Period) || tokens.contains(&BfSymbol::Comma) {
    //         out.push_str("#include <stdio.h>\n");
    //     }
    //
    //     out.push_str("int main() {\n");
    //
    //     // for token in tokens {
    //     //
    //     // }
    //
    //     out.push_str("}\n");
    //     out
    //
    // }


    #[cfg(test)]
    mod tests {
        use super::{BfSymbol, parse_without_verification, parse};
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

        // #[test]
        // fn emit_empty_program() {
        //     let tokens: Vec<BfSymbol> = vec![];
        //     assert_eq!(emit(&tokens), String::new());
        // }
    }
}
