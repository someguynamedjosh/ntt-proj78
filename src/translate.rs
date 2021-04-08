use crate::vm_program::{ArithmeticOpcode, CommandName, MemorySegment, VmCommand, VmProgram};
use std::{
    error::Error,
    fmt::{Display, Formatter},
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Register {
    A,
    D,
    M,
}

impl Display for Register {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let c = match self {
            A => 'A',
            D => 'D',
            M => 'M',
        };
        write!(f, "{}", c)
    }
}

use Register::*;

struct Translator {
    /// The VM creates its own labels for some commands, this keeps track of a counter that
    /// ensures the label names are unique.
    next_unnamed_label_id: usize,
    result: String,
}

impl Translator {
    fn make_label(&mut self) -> String {
        let label = format!("__VM_IMPL_LABEL_{}", self.next_unnamed_label_id);
        self.next_unnamed_label_id += 1;
        label
    }

    fn push(&mut self, from: Register) {
        self.result.push_str("\n// action: push\n");
        if from != D {
            self.result.push_str(&format!("D={}\n", from));
        }
        self.result.push_str(
            r"@SP      // Copy stack pointer address into A
A=M      // Copy *spa into A.
M=D      // Copy D into **spa
D=A+1    // Copy *(*spa + 1) into D
@SP      // Copy spa into A
M=D      // Copy D (==*(*spa + 1)) into *spa

",
        );
    }

    fn pop(&mut self, into: Register) {
        self.result.push_str("// action: pop\n");
        self.result.push_str(
            r"@SP      // Copy stack pointer address into A
A=M-1    // Copy *spa-1 into A
D=M      // Copy *(*spa-1) into D
@R13     // Copy r13addr into A
M=D      // Copy *(*spa-1) into *r13addr
@SP      // Copy spa into A
M=M-1    // Copy *spa-1 into *spa
@R13     // Copy r13addr into A
",
        );
        self.result.push_str(&format!(
            "{0}=M      // Copy *r13addr (==**spa) into {0}\n\n",
            into
        ));
    }

    fn translate_arithmetic_opcode(&mut self, opcode: ArithmeticOpcode) {
        self.result.push_str("// command: arithmetic\n");
        use ArithmeticOpcode::*;
        let mut pop_second = true;
        let op = match opcode {
            Add => "M=M+D",
            Sub => "M=M-D",
            Neg => {
                pop_second = false;
                "M=-M"
            }
            Eq | Gt | Lt => {
                self.pop(D);
                let skip_set_false = self.make_label();
                self.result.push_str(&format!(
                    r"@SP      // Load spa into A
A=M-1    // Load *spa-1 into A
D=M-D    // Perform comparison between D and *(*spa-1)
M=-1     // Load true into *(*spa-1)
@{0}
D;{1}    // Skip setting value to false if condition is true
@SP      // Load spa into A
A=M-1    // Load *spa-1 into A
M=0      // Load false into *(*spa-1)
({0})
// end command: arithmetic

",
                    skip_set_false,
                    match opcode {
                        Eq => "JEQ",
                        Gt => "JGT",
                        Lt => "JLT",
                        _ => unreachable!(),
                    }
                ));
                return;
            }
            And => "M=M&D",
            Or => "M=M|D",
            Not => {
                pop_second = false;
                "M=!M"
            }
        };
        if pop_second {
            self.pop(D);
        }
        self.result.push_str(&format!(
            r"@SP      // Load spa into A
A=M-1    // Load *spa-1 into A
{}
// end command: arithmetic

",
            op
        ))
    }

    fn translate_call(&mut self, fn_name: String, num_args: usize) {
        unimplemented!()
    }

    fn translate_fn_setup(&mut self, num_locals: usize) {
        unimplemented!()
    }

    fn translate_push(&mut self, segment: MemorySegment, index: usize) {
        use MemorySegment::*;
        let code = match segment {
            Constant => format!("@{}\nD=A", index),
            _ => unimplemented!(),
        };
        self.result
            .push_str(&format!("// command: push {:?} {}\n", segment, index));
        self.result.push_str(&code);
        self.push(D);
    }

    fn translate_pop(&mut self, segment: MemorySegment, index: usize) {
        use MemorySegment::*;
        unimplemented!()
        // let code = match segment {
        //     Constant => unreachable!(),
        //     Local => unimplemented!(),
        //     _ => unimplemented!()
        // };
        // self.result.push_str(&format!("// command: push {:?} {}\n", segment, index));
        // self.result.push_str(&code);
        // self.push(D);
    }

    fn translate(mut self, commands: Vec<VmCommand>) -> String {
        self.result.push_str(
            r"
// Set up stack pointer
@256
D=A
@SP
M=D

",
        );
        for command in commands {
            match command {
                VmCommand::Arithmetic(opcode) => self.translate_arithmetic_opcode(opcode),
                VmCommand::Call { fn_name, num_args } => self.translate_call(fn_name, num_args),
                VmCommand::FnSetup { num_locals } => self.translate_fn_setup(num_locals),
                VmCommand::Goto(label) => self.result.push_str(&format!("@{}\n0;JEQ\n", label)),
                VmCommand::IfGoto(label) => unimplemented!(),
                VmCommand::Label(label) => self.result.push_str(&format!("({})\n", label)),
                VmCommand::Push(segment, index) => self.translate_push(segment, index),
                VmCommand::Pop(segment, index) => self.translate_pop(segment, index),
                VmCommand::Return => unimplemented!(),
            };
        }
        self.result
    }
}

pub fn translate(program: VmProgram) -> Result<String, Box<dyn Error>> {
    let translator = Translator {
        next_unnamed_label_id: 0,
        result: String::new(),
    };
    Ok(translator.translate(program.into_commands()))
}
