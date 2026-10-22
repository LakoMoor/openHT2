//! Reader for the game's script syntax: parenthesised lists of atoms and quoted strings,
//! with `;` comments running to the end of the line.

use crate::{Error, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Sexp {
    /// An unquoted word or number, kept as text.
    Atom(String),
    /// A double-quoted string without the quotes.
    Str(String),
    List(Vec<Sexp>),
}

impl Sexp {
    pub fn as_list(&self) -> Option<&[Sexp]> {
        match self {
            Sexp::List(l) => Some(l),
            _ => None,
        }
    }

    /// Atom or string contents.
    pub fn as_text(&self) -> Option<&str> {
        match self {
            Sexp::Atom(s) | Sexp::Str(s) => Some(s),
            Sexp::List(_) => None,
        }
    }

    pub fn as_f32(&self) -> Option<f32> {
        self.as_text()?.parse().ok()
    }

    /// For a list whose first element is the atom `name`, the remaining elements.
    pub fn form(&self, name: &str) -> Option<&[Sexp]> {
        match self.as_list()? {
            [Sexp::Atom(head), rest @ ..] if head == name => Some(rest),
            _ => None,
        }
    }

    /// First child list that is a form called `name`.
    pub fn find_form(&self, name: &str) -> Option<&[Sexp]> {
        self.as_list()?.iter().find_map(|c| c.form(name))
    }
}

/// Parse all top-level forms of `text`.
pub fn parse(text: &str) -> Result<Vec<Sexp>> {
    let mut stack: Vec<Vec<Sexp>> = vec![Vec::new()];
    let mut chars = text.char_indices().peekable();
    while let Some((at, c)) = chars.next() {
        match c {
            ';' => {
                for (_, c) in chars.by_ref() {
                    if c == '\n' {
                        break;
                    }
                }
            }
            '(' => stack.push(Vec::new()),
            ')' => {
                let done = stack.pop().filter(|_| !stack.is_empty()).ok_or_else(|| Error::Other(format!("unbalanced ')' at {at}")))?;
                stack.last_mut().unwrap().push(Sexp::List(done));
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match chars.next() {
                        Some((_, '"')) => break,
                        // \" and \\ stand for a quote and a backslash ("Телевизор 14\"" in
                        // merchandise.sch); other backslashes are kept
                        Some((_, '\\')) => match chars.peek() {
                            Some(&(_, q @ ('"' | '\\'))) => {
                                s.push(q);
                                chars.next();
                            }
                            _ => s.push('\\'),
                        },
                        Some((_, c)) => s.push(c),
                        None => return Err(Error::Other(format!("unterminated string at {at}"))),
                    }
                }
                stack.last_mut().unwrap().push(Sexp::Str(s));
            }
            c if c.is_whitespace() => {}
            c => {
                let mut s = String::from(c);
                while let Some(&(_, n)) = chars.peek() {
                    if n.is_whitespace() || matches!(n, '(' | ')' | '"' | ';') {
                        break;
                    }
                    s.push(n);
                    chars.next();
                }
                stack.last_mut().unwrap().push(Sexp::Atom(s));
            }
        }
    }
    if stack.len() != 1 {
        return Err(Error::Other("unclosed '('".into()));
    }
    Ok(stack.pop().unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_game_config() {
        let t = "(\n (playertypes (live 1 \"ZilR\" 1) ; comment\n (police 15))\n (startposition -3020 -4625 20))";
        let top = parse(t).unwrap();
        let root = &top[0];
        let pos = root.find_form("startposition").unwrap();
        assert_eq!(pos.iter().filter_map(Sexp::as_f32).collect::<Vec<_>>(), [-3020.0, -4625.0, 20.0]);
        let live = root.find_form("playertypes").unwrap()[0].form("live").unwrap();
        assert_eq!(live[1].as_text(), Some("ZilR"));
    }

    #[test]
    fn rejects_unbalanced() {
        assert!(parse("(a (b)").is_err());
        assert!(parse("a)").is_err());
        assert!(parse("(\"x)").is_err());
    }
}
