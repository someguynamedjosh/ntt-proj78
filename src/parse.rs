use std::error::Error;
use crate::vm_program::VmProgram;

#[derive(Clone, Copy)]
enum ParserState {
    Command,
    Argument {
        /// Indicates which argument the next symbol will provide.
        current_index: usize,
        /// Indicates how many more arguments need to be specified for this 
        remaining: usize,
    }
}

struct Parser<'a> {
    source: &'a str,
    current_line: usize,
    current_col: usize,
    current_state: ParserState,
}

impl<'a> Parser<'a> {
    pub fn new(source: &'a str) -> Self {
        Self {
            source,
            current_line: 1,
            current_col: 1,
            current_state: ParserState::Command,
        }
    }

    pub fn parse(mut self) -> VmProgram {
        VmProgram
    }
}

pub fn parse(src: &str) -> Result<VmProgram, Box<dyn Error>> {
    unimplemented!()
}