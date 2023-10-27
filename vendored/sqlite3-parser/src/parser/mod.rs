//! SQLite parser
use log::error;

pub mod ast;
pub mod parse {
    #![allow(unused_braces)]
    #![allow(unused_comparisons)] // FIXME
    #![allow(clippy::collapsible_if)]
    #![allow(clippy::if_same_then_else)]
    #![allow(clippy::absurd_extreme_comparisons)] // FIXME
    #![allow(clippy::needless_return)]
    #![allow(clippy::upper_case_acronyms)]
    #![allow(clippy::manual_range_patterns)]

    include!(concat!(env!("OUT_DIR"), "/parse.rs"));
}

use crate::dialect::Token;
use ast::{Cmd, ExplainKind, Name, Stmt};

/// Parser error
#[derive(Debug)]
pub enum ParserError {
    StackOverflow,
    SyntaxError {
        token_type: &'static str,
        found: Option<String>,
    },
    UnexpectedEof,
    Custom(String),
}

impl std::fmt::Display for ParserError {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            ParserError::StackOverflow => f.write_str("parser overflowed its stack"),
            ParserError::SyntaxError { token_type, found } => {
                write!(f, "near {}, \"{:?}\": syntax error", token_type, found)
            }
            ParserError::UnexpectedEof => f.write_str("unexpected end of input"),
            ParserError::Custom(s) => f.write_str(s),
        }
    }
}

impl std::error::Error for ParserError {}

/// Parser context
pub struct Context<'input> {
    input: &'input [u8],
    explain: Option<ExplainKind>,
    stmt: Option<Stmt>,
    constraint_name: Option<Name>,      // transient
    module_arg: Option<(usize, usize)>, // Complete text of a module argument
    module_args: Option<Vec<String>>,   // CREATE VIRTUAL TABLE args
    done: bool,
    error: Option<ParserError>,
}

impl<'input> Context<'input> {
    pub fn new(input: &'input [u8]) -> Context<'input> {
        Context {
            input,
            explain: None,
            stmt: None,
            constraint_name: None,
            module_arg: None,
            module_args: None,
            done: false,
            error: None,
        }
    }

    /// Consume parsed command
    pub fn cmd(&mut self) -> Option<Cmd> {
        if let Some(stmt) = self.stmt.take() {
            match self.explain.take() {
                Some(ExplainKind::Explain) => Some(Cmd::Explain(stmt)),
                Some(ExplainKind::QueryPlan) => Some(Cmd::ExplainQueryPlan(stmt)),
                None => Some(Cmd::Stmt(stmt)),
            }
        } else {
            None
        }
    }

    fn constraint_name(&mut self) -> Option<Name> {
        self.constraint_name.take()
    }
    fn no_constraint_name(&self) -> bool {
        self.constraint_name.is_none()
    }

    fn vtab_arg_init(&mut self) {
        self.add_module_arg();
        self.module_arg = None;
    }
    fn vtab_arg_extend(&mut self, any: Token) {
        if let Some((_, ref mut n)) = self.module_arg {
            *n = any.2
        } else {
            self.module_arg = Some((any.0, any.2))
        }
    }
    fn add_module_arg(&mut self) {
        if let Some((start, end)) = self.module_arg.take() {
            if let Ok(arg) = std::str::from_utf8(&self.input[start..end]) {
                self.module_args.get_or_insert(vec![]).push(arg.to_owned());
            } // FIXME error handling
        }
    }
    fn module_args(&mut self) -> Option<Vec<String>> {
        self.add_module_arg();
        self.module_args.take()
    }

    fn sqlite3_error_msg(&mut self, msg: &str) {
        error!("parser error: {}", msg);
    }

    /// This routine is called after a single SQL statement has been parsed.
    fn sqlite3_finish_coding(&mut self) {
        self.done = true;
    }

    /// Return `true` if parser completes either successfully or with an error.
    pub fn done(&self) -> bool {
        self.done || self.error.is_some()
    }

    pub fn is_ok(&self) -> bool {
        self.error.is_none()
    }

    /// Consume error generated by parser
    pub fn error(&mut self) -> Option<ParserError> {
        self.error.take()
    }

    pub fn reset(&mut self) {
        self.explain = None;
        self.stmt = None;
        self.constraint_name = None;
        self.module_arg = None;
        self.module_args = None;
        self.done = false;
        self.error = None;
    }
}