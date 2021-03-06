use std::{
    io::{Read, Write},
    num::Wrapping,
};

use crate::lir::{Lir, Stmt};

const MEM_SIZE: usize = 32_000;

type Memory = [Wrapping<u8>; MEM_SIZE];

// `repr(C)` to make sure rustc never reorders the fields weirdly
// maybe useless, but seems to give tiny wins
#[repr(C)]
struct Interpreter<'lir, W, R, P> {
    code: &'lir Lir<'lir>,
    profile_collector: P,
    ip: usize,
    ptr: usize,
    mem: Memory,
    stdout: W,
    stdin: R,
}

pub fn run<W, R, P>(code: &Lir<'_>, stdout: W, stdin: R, profile_collector: P)
where
    W: Write,
    R: Read,
    P: FnMut(usize),
{
    let mut interpreter = Interpreter {
        code,
        ip: 0,
        ptr: 0,
        stdout,
        stdin,
        mem: [Wrapping(0u8); MEM_SIZE],
        profile_collector,
    };

    // SAFETY: `Lir` can only be produced by the `crate::lir` module, which is trusted to not
    // produce out of bounds jumps and put the `End` at the end
    unsafe {
        interpreter.execute();
    }
}

impl<'c, W: Write, R: Read, P> Interpreter<'c, W, R, P>
where
    P: FnMut(usize),
{
    unsafe fn execute(&mut self) {
        let stmts = self.code.stmts();
        loop {
            // SAFETY: If the code ends with an `End` and there are no out of bounds jumps,
            // `self.ip` will never be out of bounds
            // Removing this bounds check speeds up execution by about 40%
            debug_assert!(self.ip < stmts.len());
            let instr = unsafe { *stmts.get_unchecked(self.ip) };
            self.ip += 1;
            match instr {
                Stmt::Add(n) => {
                    *self.elem_mut() += n;
                }
                Stmt::Sub(n) => {
                    *self.elem_mut() -= n;
                }
                Stmt::AddOffset { offset, n } => *self.elem_mut_offset(offset) += n,
                Stmt::SubOffset { offset, n } => *self.elem_mut_offset(offset) -= n,
                Stmt::MoveAddTo { offset } => {
                    let value = self.elem();
                    *self.elem_mut() = Wrapping(0);
                    *self.elem_mut_offset(offset) += value;
                }
                Stmt::Right(n) => {
                    self.ptr += n as usize;
                    if self.ptr >= MEM_SIZE {
                        self.ptr = 0;
                    }
                }
                Stmt::Left(n) => {
                    if self.ptr < n as usize {
                        let diff = n as usize - self.ptr;
                        self.ptr = MEM_SIZE - 1 - diff;
                    } else {
                        self.ptr -= n as usize;
                    }
                }
                Stmt::Out => {
                    let char = self.elem() as char;
                    write!(self.stdout, "{char}").unwrap();
                    self.stdout.flush().unwrap();
                }
                Stmt::In => {
                    let mut buf = [0; 1];
                    self.stdin.read_exact(&mut buf).unwrap();
                    *self.elem_mut() = Wrapping(buf[0]);
                }
                Stmt::SetN(n) => {
                    *self.elem_mut() = Wrapping(n);
                }
                Stmt::JmpIfZero(pos) => {
                    if self.elem() == 0 {
                        self.ip = pos as usize;
                    }
                }
                Stmt::JmpIfNonZero(pos) => {
                    if self.elem() != 0 {
                        self.ip = pos as usize;
                    }
                }
                Stmt::End => break,
            }

            // this should be a no-op if `profile_collector` is does nothing
            (self.profile_collector)(self.ip);
        }
    }

    fn elem_mut_offset(&mut self, offset: i32) -> &mut Wrapping<u8> {
        let ptr = self.ptr as isize;
        let offset = offset as isize;
        // SAFETY: `self.ptr` is never out of bounds
        debug_assert!(self.ptr < self.mem.len());
        unsafe { self.mem.get_unchecked_mut((ptr + offset) as usize) }
    }

    fn elem_mut(&mut self) -> &mut Wrapping<u8> {
        // SAFETY: `self.ptr` is never out of bounds
        debug_assert!(self.ptr < self.mem.len());
        unsafe { self.mem.get_unchecked_mut(self.ptr) }
    }

    fn elem(&self) -> u8 {
        // SAFETY: `self.ptr` is never out of bounds
        debug_assert!(self.ptr < self.mem.len());
        unsafe { self.mem.get_unchecked(self.ptr).0 }
    }
}
