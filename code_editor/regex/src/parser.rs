use {
    crate::{ast::Quant, Ast},
    std::str::Chars,
};

#[derive(Clone, Debug)]
pub struct Parser {
    asts: Vec<Ast>,
    groups: Vec<Group>,
}

impl Parser {
    pub(crate) fn new() -> Self {
        Self {
            asts: Vec::new(),
            groups: Vec::new(),
        }
    }

    pub(crate) fn parse(&mut self, string: &str) -> Ast {
        let mut chars = string.chars();
        ParseContext {
            cap_count: 1,
            ch_0: chars.next(),
            ch_1: chars.next(),
            chars,
            position: 0,
            asts: &mut self.asts,
            groups: &mut self.groups,
            group: Group::new(Some(0)),
        }
        .parse()
    }
}

#[derive(Debug)]
struct ParseContext<'a> {
    cap_count: usize,
    ch_0: Option<char>,
    ch_1: Option<char>,
    chars: Chars<'a>,
    position: usize,
    asts: &'a mut Vec<Ast>,
    groups: &'a mut Vec<Group>,
    group: Group,
}

impl<'a> ParseContext<'a> {
    fn parse(&mut self) -> Ast {
        loop {
            match self.peek_char() {
                Some('|') => {
                    self.skip_char();
                    self.maybe_push_cat();
                    self.pop_cats();
                    self.group.alt_count += 1;
                }
                Some('?') => {
                    self.skip_char();
                    let ast = self.asts.pop().unwrap();
                    self.asts.push(Ast::Rep(Box::new(ast), Quant::Quest));
                }
                Some('*') => {
                    self.skip_char();
                    let ast = self.asts.pop().unwrap();
                    self.asts.push(Ast::Rep(Box::new(ast), Quant::Star));
                }
                Some('+') => {
                    self.skip_char();
                    let ast = self.asts.pop().unwrap();
                    self.asts.push(Ast::Rep(Box::new(ast), Quant::Plus));
                }
                Some('(') => {
                    self.skip_char();
                    let cap = match self.peek_two_chars() {
                        (Some('?'), Some(':')) => {
                            self.skip_two_chars();
                            false
                        }
                        _ => true,
                    };
                    self.push_group(cap);
                }
                Some(')') => {
                    self.skip_char();
                    self.pop_group();
                }
                Some(ch) => {
                    self.skip_char();
                    self.maybe_push_cat();
                    self.asts.push(Ast::Char(ch));
                    self.group.ast_count += 1;
                }
                None => break,
            }
        }
        self.maybe_push_cat();
        self.pop_alts();
        Ast::Cap(Box::new(self.asts.pop().unwrap()), 0)
    }

    fn peek_char(&self) -> Option<char> {
        self.ch_0
    }

    fn peek_two_chars(&self) -> (Option<char>, Option<char>) {
        (self.ch_0, self.ch_1)
    }

    fn skip_char(&mut self) {
        self.position += self.ch_0.unwrap().len_utf8();
        self.ch_0 = self.ch_1;
        self.ch_1 = self.chars.next();
    }

    fn skip_two_chars(&mut self) {
        self.position += self.ch_1.unwrap().len_utf8();
        self.position += self.ch_1.unwrap().len_utf8();
        self.ch_0 = self.chars.next();
        self.ch_1 = self.chars.next();
    }

    fn push_group(&mut self, cap: bool) {
        use std::mem;

        self.maybe_push_cat();
        self.pop_cats();
        let cap_index = if cap {
            let cap_index = self.cap_count;
            self.cap_count += 1;
            Some(cap_index)
        } else {
            None
        };
        let group = mem::replace(&mut self.group, Group::new(cap_index));
        self.groups.push(group);
    }

    fn pop_group(&mut self) {
        self.maybe_push_cat();
        self.pop_alts();
        if let Some(index) = self.group.cap {
            let ast = self.asts.pop().unwrap();
            self.asts.push(Ast::Cap(Box::new(ast), index));
        }
        self.group = self.groups.pop().unwrap();
        self.group.ast_count += 1;
    }

    fn maybe_push_cat(&mut self) {
        if self.group.ast_count - self.group.alt_count - self.group.cat_count == 2 {
            self.group.cat_count += 1;
        }
    }

    fn pop_alts(&mut self) {
        self.pop_cats();
        if self.group.alt_count == 0 {
            return;
        }
        let asts = self
            .asts
            .split_off(self.asts.len() - (self.group.alt_count + 1));
        self.asts.push(Ast::Alt(asts));
        self.group.ast_count -= self.group.alt_count;
        self.group.alt_count = 0;
    }

    fn pop_cats(&mut self) {
        if self.group.cat_count == 0 {
            return;
        }
        let asts = self
            .asts
            .split_off(self.asts.len() - (self.group.cat_count + 1));
        self.asts.push(Ast::Cat(asts));
        self.group.ast_count -= self.group.cat_count;
        self.group.cat_count = 0;
    }
}

#[derive(Clone, Copy, Debug)]
struct Group {
    cap: Option<usize>,
    ast_count: usize,
    alt_count: usize,
    cat_count: usize,
}

impl Group {
    fn new(index: Option<usize>) -> Self {
        Self {
            cap: index,
            ast_count: 0,
            alt_count: 0,
            cat_count: 0,
        }
    }
}
