//! an experimental MIR (mid-level-ir)
//!
//! The MIR consists of two parts. First, there are instructions (`Stmt`). These instructions
//! can be seen as an extended version of the default brainfuck instruction set `+-<>,.[]`.
//! These instructions modify the classic tape. What MIR does is that it attaches an abstract
//! `MemoryState` to *each* statement. This state contains all facts known about the state of the
//! tape at the point of execution of the statement.
//!
//! For example, for the code `++.`, the `MemoryState` for the `.` instruction contains a single
//! fact: "The current cell was written to, by the instruction before and with the value 2". MIR
//! tracks as much of the reads/writes to determine their dependencies and eliminate as many
//! of them as possible.
//!
//! Note that MIR is always pessimized, so if it can't determine for sure that something is true,
//! it will not act on it.
#![allow(dead_code)]

mod opts;
mod state;

use std::fmt::{Debug, Formatter};

use bumpalo::Bump;

use crate::{
    hir::{Hir, StmtKind as HirStmtKind},
    mir::state::{MemoryState, Store},
    parse::Span,
    BumpVec,
};

#[derive(Debug, Clone)]
pub struct Mir<'mir> {
    stmts: BumpVec<'mir, Stmt<'mir>>,
}

#[derive(Clone)]
struct Stmt<'mir> {
    kind: StmtKind<'mir>,
    state: MemoryState<'mir>,
    span: Span,
}

impl Debug for Stmt<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Stmt")
            .field("kind", &self.kind)
            .field("state", &self.state)
            .finish()
    }
}

type Offset = i32;

#[derive(Debug, Clone)]
enum StmtKind<'mir> {
    /// Add or sub, the value has the valid range -255..=255
    AddSub {
        offset: Offset,
        n: i16,
        store: Store,
    },
    /// Sets the current cell to 0 and adds that value of the cell to another cell at `offset`
    MoveAddTo {
        offset: Offset,
        store_set_null: Store,
        store_move: Store,
    },
    /// Left or Right pointer move (`<>`)
    PointerMove(Offset),
    Loop(Mir<'mir>),
    Out,
    In(Store),
    SetN(u8, Store),
}

#[tracing::instrument(skip(alloc, hir))]
pub fn optimized_mir<'mir>(alloc: &'mir Bump, hir: &Hir<'_>) -> Mir<'mir> {
    let mut mir = hir_to_mir(alloc, hir);
    opts::passes(alloc, &mut mir);
    mir
}

/// compiles hir down to a minimal mir
fn hir_to_mir<'mir>(alloc: &'mir Bump, hir: &Hir<'_>) -> Mir<'mir> {
    let mut stmts = Vec::new_in(alloc);
    let iter = hir.stmts.iter().map(|hir_stmt| {
        let kind = match *hir_stmt.kind() {
            HirStmtKind::Add(offset, n) => StmtKind::AddSub {
                offset,
                n: i16::from(n),
                store: Store::dead(),
            },
            HirStmtKind::Sub(offset, n) => StmtKind::AddSub {
                offset,
                n: -i16::from(n),
                store: Store::dead(),
            },
            HirStmtKind::MoveAddTo { offset } => StmtKind::MoveAddTo {
                offset,
                store_set_null: Store::dead(),
                store_move: Store::dead(),
            },
            HirStmtKind::Right(n) => StmtKind::PointerMove(i32::try_from(n).unwrap()),
            HirStmtKind::Left(n) => StmtKind::PointerMove(-i32::try_from(n).unwrap()),
            HirStmtKind::Loop(ref body) => StmtKind::Loop(hir_to_mir(alloc, body)),
            HirStmtKind::Out => StmtKind::Out,
            HirStmtKind::In => StmtKind::In(Store::dead()),
            HirStmtKind::SetN(n) => StmtKind::SetN(n, Store::dead()),
        };
        Stmt {
            kind,
            span: hir_stmt.span,
            state: MemoryState::empty(alloc),
        }
    });
    stmts.extend(iter);

    Mir { stmts }
}
