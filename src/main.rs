use std::error::Error;

mod vm_program;

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

impl VmProgram {
    pub fn parse_from(src: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self)
    }

    fn translated(self) -> Result<String, Box<dyn Error>> {
        Ok(format!("Hello this is the code."))
    }
}

fn entry() -> Result<(), Box<dyn Error>> {
    // TODO: Accept directories.
    let mut source = String::new();
    let base_name = std::env::args().skip(1).next();
    let base_name = base_name.ok_or(format!("Must specify at least one file or folder."))?;
    for filename in std::env::args().skip(1) {
        if !filename.contains(".vm") {
            Err(format!(
                "The file \"{}\" has the wrong extension (expected .vm).",
                filename
            ))?;
        }
        let contents = std::fs::read_to_string(&filename);
        let contents = contents
            .map_err(|err| format!("Failed to open \"{}\", caused by:\n{:?}", filename, err))?;
        source.push_str(&contents.to_lowercase());
    }
    let program = VmProgram::parse_from(&source[..])?;
    let result = program.translated()?;
    println!("{}", result);
    let output_name = if base_name.contains("vm") {
        base_name.replace(".vm", ".asm")
    } else {
        format!("{}.asm", base_name)
    };
    let result = std::fs::write(&output_name, result);
    result.map_err(|err| {
        format!(
            "Failed to write result to \"{}\", caused by:\n{:?}",
            output_name, err
        )
    })?;
    println!("Wrote output to \"{}\"", output_name);
    Ok(())
}

fn main() {
    match entry() {
        Ok(_) => {
            println!("Operation completed sucessfully.");
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("Encountered error: {:?}", err);
            drop(err);
            std::process::exit(1);
        }
    }
}
