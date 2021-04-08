use crate::vm_program::VmProgram;
use std::error::Error;

const VIRTUAL_REGISTER_START: usize = 0;
const STATIC_MEMORY_START: usize = 16;
const STACK_MEMORY_START: usize = 256;
const HEAP_MEMORY_START: usize = 2048;
const MEMORY_MAPPED_IO_START: usize = 16384;

const STACK_POINTER_ADDR: &str = "SP";
const LOCAL_POINTER_ADDR: &str = "LCL";
const ARGUMENT_POINTER_ADDR: &str = "ARG";
const THIS_POINTER_ADDR: &str = "THIS";
const THAT_POINTER_ADDR: &str = "THAT";
const TEMP_SEGMENT_START: usize = VIRTUAL_REGISTER_START + 5;
const TEMP_SEGMENT_LENGTH: usize = 4;
const GENERAL_PURPOSE_ADDRS: [&str; 3] = ["R13", "R14", "R15"];

pub fn translate(program: VmProgram) -> Result<String, Box<dyn Error>> {
    Ok(format!("Code."))
}
