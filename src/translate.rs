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
    /// Used to determine how many locals should be popped when a return command is encountered.
    current_num_locals: usize,
}

impl Translator {
    fn make_label(&mut self) -> String {
        let label = format!("__VM_IMPL_LABEL_{}", self.next_unnamed_label_id);
        self.next_unnamed_label_id += 1;
        label
    }

    fn push(&mut self, from: Register) {
        self.result.push_str("// action: push\n");
        if from != D {
            self.result.push_str(&format!("D={}\n", from));
        }
        self.result.push_str(
            r"@SP      // load stack pointer address into A
A=M      // load *spa into A.
M=D      // load D into **spa
D=A+1    // load *(*spa + 1) into D
@SP      // load spa into A
M=D      // load D (==*(*spa + 1)) into *spa
",
        );
    }

    fn pop(&mut self, into: Register) {
        self.result.push_str("// action: pop\n");
        self.result.push_str(
            r"@SP      // load stack pointer address into A
A=M-1    // load *spa-1 into A
D=M      // load *(*spa-1) into D
@R13     // load r13addr into A
M=D      // load *(*spa-1) into *r13addr
@SP      // load spa into A
M=M-1    // load *spa-1 into *spa
@R13     // load r13addr into A
",
        );
        self.result.push_str(&format!(
            "{0}=M      // Copy *r13addr (==**spa) into {0}\n",
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
A=M-1    // load *spa-1 into A
D=M-D    // perform comparison between D and *(*spa-1)
M=-1     // load true into *(*spa-1)
@{0}
D;{1}    // skip setting value to false if condition is true
@SP      // load spa into A
A=M-1    // load *spa-1 into A
M=0      // load false into *(*spa-1)
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

    // Stack stuff for function calls:
    // arg 0 (*ARG)
    // arg 1
    // arg 2
    // ...
    // arg N
    // return address
    // old LCL
    // old ARG
    // old THIS
    // old THAT
    // local 0 (*LCL)
    // local 1
    // local 2
    // ...
    // local N (*SP)
    // Eventual return value (moved to R14 on return.)
    fn translate_call(&mut self, fn_name: String, num_args: usize) {
        let ret_label = self.make_label();
        self.result.push_str(&format!(
            r"// command: call {0} {1}
// push return address onto stack.
@{2}
",
            fn_name, num_args, ret_label
        ));
        self.push(A);
        self.result.push_str("// push old LCL onto stack\n@LCL\n");
        self.push(M);
        self.result.push_str("// push old ARG onto stack\n@ARG\n");
        self.push(M);
        self.result.push_str("// push old THIS onto stack\n@THIS\n");
        self.push(M);
        self.result.push_str("// push old THAT onto stack\n@THAT\n");
        self.push(M);
        self.result.push_str(&format!(
            r"// create new ARG pointer
@{0} 
D=A      // load numargs into D
@5
D=D+A    // add five to compensate for additional pushed values.
@SP      // load spa into A
D=M-D    // load *spa - (numargs + 5) into D
@ARG     // load argptr into A
M=D      // load *spa - (numargs + 5) into *argptr
// create new LCL pointer
@SP
D=M
@LCL
M=D
// jump to function
@{1}
0;JEQ
({2})
",
            num_args, fn_name, ret_label
        ));
        self.result.push_str("// end command: call {0} {1}\n\n");
    }

    fn translate_fn_setup(&mut self, num_locals: usize) {
        self.current_num_locals = num_locals;
        self.result
            .push_str(&format!("// command: function {}\n", num_locals));
        for idx in 0..num_locals {
            self.result
                .push_str(&format!("// push local #{}\nD=0\n", idx));
            self.push(D);
        }
        self.result
            .push_str(&format!("// end command: function {}\n\n", num_locals));
    }

    fn translate_return(&mut self) {
        self.result.push_str(&format!(
            "// command: return ({0} locals)\n// pop return value\n",
            self.current_num_locals
        ));
        self.pop(D);
        self.result.push_str("// store in R14\n@R14\nM=D\n");
        self.result.push_str(
            r"// deallocate locals
@LCL
D=M      // load *localptr into D
@SP
M=D      // load D (==*localptr) into *stackptr
// store ARG value in R15
@ARG
D=M
@R15
M=D
// restore old THAT value
",
        );
        self.pop(D);
        self.result
            .push_str("@THAT\nM=D\n// restore old THIS value\n");
        self.pop(D);
        self.result
            .push_str("@THIS\nM=D\n// restore old ARG value\n");
        self.pop(D);
        self.result
            .push_str("@ARG\nM=D\n// restore old LCL value\n");
        self.pop(D);
        self.result
            .push_str("@LCL\nM=D\n// store return address in R13\n");
        self.pop(D);
        self.result.push_str("@R13\nM=D\n");
        self.result
            .push_str("// reset stack pointer from R15 and push return value\n");
        self.result.push_str("@R15\nD=M\n@SP\nM=D\n@R14\n");
        self.push(M);
        self.result.push_str("// jump to return address\n");
        self.result.push_str("@R13\nA=M\n0;JEQ\n");
        self.result.push_str("// end command: return\n\n");
    }

    fn load_d_from_offset(offset: usize) -> String {
        format!("@{}\nD=M\n", offset)
    }

    fn load_d_from_ptr_offset(ptr_name: &str, offset: usize) -> String {
        format!("@{}\nD=M\n@{}\nA=D+A\nD=M\n", ptr_name, offset)
    }

    fn store_d_into_offset(offset: usize) -> String {
        format!("@{}\nM=D\n", offset)
    }

    fn store_d_into_ptr_offset(ptr_name: &str, offset: usize) -> String {
        // ew...
        format!(
            "@R13\nM=D\n@{0}\nD=A\n@{1}\nD=D+A\n@R14\nM=D\n@R13\nD=M\n@R14\nA=M\nM=D\n",
            ptr_name, offset
        )
    }

    fn translate_push(&mut self, segment: MemorySegment, index: usize) {
        use MemorySegment::*;
        let code = match segment {
            Constant => format!("@{}\nD=A", index),
            Local => Self::load_d_from_ptr_offset("LCL", index),
            Argument => Self::load_d_from_ptr_offset("ARG", index),
            This => Self::load_d_from_ptr_offset("THIS", index),
            That => Self::load_d_from_ptr_offset("THAT", index),
            Pointer => Self::load_d_from_offset(3 + index),
            Temp => Self::load_d_from_offset(5 + index),
            _ => unimplemented!("{:?}", segment),
        };
        self.result
            .push_str(&format!("// command: push {:?} {}\n", segment, index));
        self.result.push_str(&code);
        self.push(D);
        self.result.push_str("// end command: push\n\n");
    }

    fn translate_pop(&mut self, segment: MemorySegment, index: usize) {
        use MemorySegment::*;
        let code = match segment {
            Constant => unreachable!(),
            Local => Self::store_d_into_ptr_offset("LCL", index),
            Argument => Self::store_d_into_ptr_offset("ARG", index),
            This => Self::store_d_into_ptr_offset("THIS", index),
            That => Self::store_d_into_ptr_offset("THAT", index),
            Pointer => Self::store_d_into_offset(3 + index),
            Temp => Self::store_d_into_offset(5 + index),
            _ => unimplemented!("{:?}", segment),
        };
        self.result
            .push_str(&format!("// command: pop {:?} {}\n", segment, index));
        self.pop(D);
        self.result.push_str(&code);
    }

    fn translate(mut self, commands: Vec<VmCommand>) -> String {
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
                VmCommand::Return => self.translate_return(),
            };
        }
        self.result
    }
}

pub fn translate(program: VmProgram) -> Result<String, Box<dyn Error>> {
    let translator = Translator {
        next_unnamed_label_id: 0,
        result: String::new(),
        current_num_locals: 0,
    };
    Ok(translator.translate(program.into_commands()))
}
