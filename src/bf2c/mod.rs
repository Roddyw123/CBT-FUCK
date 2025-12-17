pub mod bf2c {
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
    pub fn tokenise(buf: &str) -> Vec<BfSymbol> {
        let mut out = Vec::new();
        for c in buf.trim().chars() {
            match c {
                '<' => out.push(BfSymbol::Left),
                '>' => out.push(BfSymbol::Right),
                '+' => out.push(BfSymbol::Plus),
                '-' => out.push(BfSymbol::Minus),
                '.' => out.push(BfSymbol::Period),
                ',' => out.push(BfSymbol::Comma),
                '[' => out.push(BfSymbol::OpenBracket),
                ']' => out.push(BfSymbol::CloseBracket),
                _ => {} // ignore non-BF characters
            }
        }
        out
    }
}

#[cfg(test)]
mod tests {
    use super::bf2c::{BfSymbol, tokenise};
    #[test]
    fn tokenise_empty() {
        assert!(tokenise("").is_empty());
    }

    #[test]
    fn tokenise_only_bf() {
        let tokens = tokenise("<>+-.,[]");
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
    fn tokenise_non_bf() {
        let tokens = tokenise("abcdefg[]");
        assert_eq!(tokens.len(), 2);
        assert_eq!(tokens[0], BfSymbol::OpenBracket);
        assert_eq!(tokens[1], BfSymbol::CloseBracket);
    }
}