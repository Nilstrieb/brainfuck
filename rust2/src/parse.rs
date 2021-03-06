use std::{
    cmp,
    fmt::{Debug, Formatter},
};

use bumpalo::Bump;

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Span {
    start: u32,
    len: u32,
}

impl Span {
    fn single(idx: usize) -> Self {
        Self {
            start: idx.try_into().unwrap(),
            len: 1,
        }
    }

    /// start..end
    fn start_end(start: usize, end: usize) -> Span {
        Self {
            start: start.try_into().unwrap(),
            len: (end - start).try_into().unwrap(),
        }
    }

    // start..=end
    fn start_end_incl(start: usize, end: usize) -> Span {
        Self {
            start: start.try_into().unwrap(),
            len: (end - start + 1).try_into().unwrap(),
        }
    }

    #[must_use]
    pub fn until(&self, other: Self) -> Self {
        Self {
            start: self.start,
            len: (other.start + other.len) - self.len,
        }
    }

    #[must_use]
    pub fn merge(&self, other: Self) -> Self {
        Self::start_end(
            cmp::min(self.start(), other.start()),
            cmp::max(self.end(), other.end()),
        )
    }

    pub fn start(&self) -> usize {
        self.start.try_into().unwrap()
    }

    pub fn len(&self) -> usize {
        self.len.try_into().unwrap()
    }

    /// ..end
    pub fn end(&self) -> usize {
        self.start() + self.len()
    }
}

impl Debug for Span {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        Debug::fmt(&(self.start..(self.start + self.len)), f)
    }
}

pub type Ast<'ast> = Vec<(Instr<'ast>, Span), &'ast Bump>;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Instr<'ast> {
    Add,
    Sub,
    Right,
    Left,
    Out,
    In,
    Loop(Ast<'ast>),
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParseError;

pub fn parse<I>(alloc: &Bump, mut src: I) -> Result<Ast<'_>, ParseError>
where
    I: Iterator<Item = (usize, u8)>,
{
    let mut instrs = Vec::new_in(alloc);

    loop {
        match src.next() {
            Some((idx, b'+')) => instrs.push((Instr::Add, Span::single(idx))),
            Some((idx, b'-')) => instrs.push((Instr::Sub, Span::single(idx))),
            Some((idx, b'>')) => instrs.push((Instr::Right, Span::single(idx))),
            Some((idx, b'<')) => instrs.push((Instr::Left, Span::single(idx))),
            Some((idx, b'.')) => instrs.push((Instr::Out, Span::single(idx))),
            Some((idx, b',')) => instrs.push((Instr::In, Span::single(idx))),
            Some((idx, b'[')) => {
                let (loop_instrs, span) = parse_loop(alloc, &mut src, 0, idx)?;
                instrs.push((Instr::Loop(loop_instrs), span));
            }
            Some((_, b']')) => return Err(ParseError),
            Some(_) => {} // comment
            None => break,
        }
    }

    Ok(instrs)
}

fn parse_loop<'ast, I>(
    alloc: &'ast Bump,
    src: &mut I,
    depth: u16,
    start_idx: usize,
) -> Result<(Ast<'ast>, Span), ParseError>
where
    I: Iterator<Item = (usize, u8)>,
{
    const MAX_DEPTH: u16 = 1000;

    if depth > MAX_DEPTH {
        return Err(ParseError);
    }

    let mut instrs = Vec::new_in(alloc);

    let end_idx = loop {
        match src.next() {
            Some((idx, b'+')) => instrs.push((Instr::Add, Span::single(idx))),
            Some((idx, b'-')) => instrs.push((Instr::Sub, Span::single(idx))),
            Some((idx, b'>')) => instrs.push((Instr::Right, Span::single(idx))),
            Some((idx, b'<')) => instrs.push((Instr::Left, Span::single(idx))),
            Some((idx, b'.')) => instrs.push((Instr::Out, Span::single(idx))),
            Some((idx, b',')) => instrs.push((Instr::In, Span::single(idx))),
            Some((idx, b'[')) => {
                let (loop_instrs, span) = parse_loop(alloc, src, depth + 1, idx)?;
                instrs.push((Instr::Loop(loop_instrs), span));
            }
            Some((idx, b']')) => break idx,
            Some(_) => {} // comment
            None => return Err(ParseError),
        }
    };

    Ok((instrs, Span::start_end_incl(start_idx, end_idx)))
}

#[cfg(test)]
mod tests {
    use bumpalo::Bump;

    #[test]
    fn simple() {
        let alloc = Bump::new();

        let bf = ">+<++[-].";
        let instrs = super::parse(&alloc, bf.bytes().enumerate());
        insta::assert_debug_snapshot!(instrs);
    }

    #[test]
    fn nested_loop() {
        let alloc = Bump::new();

        let bf = "+[-[-[-]]+>>>]";
        let instrs = super::parse(&alloc, bf.bytes().enumerate());
        insta::assert_debug_snapshot!(instrs);
    }
}
