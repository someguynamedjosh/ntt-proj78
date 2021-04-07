use std::error::Error;

/// Creates an enum with a public function `from_name` that returns the corresponding enum variant
/// given a matching string.
macro_rules! keyword_enum {
    ($EnumName:ident {
        $($EnumVariantName:ident $name_in_source:ident),*$(,)?
    }) => {
        #[derive(Clone, Copy)]
        enum $EnumName {
            $($EnumVariantName),*
        }
        impl $EnumName {
            pub fn from_name(name: &str) -> Option<Self> {
                match name {
                    $(stringify!($name_in_source) => Some(Self::$EnumVariantName)),*,
                    _ => None,
                }
            }
        }
    }
}

// Everything on the left is the Rust identifier of an enum variant and the things on the right
// are the corresponding text of that keyword in the source file.
keyword_enum!(ArithmeticOpcode {
    Add add,
    Sub sub,
    Neg neg,
    Eq eq,
    Gt gt,
    Lt lt,
    And and,
    Or or,
    Not not,
});

keyword_enum!(MemorySegment {
    Argument argument,
    Local local,
    Static static,
    Constant constant,
    This this,
    That that,
    Pointer pointer,
    Temp temp,
});

#[derive(Clone, Copy, Debug)]
struct SymbolId(usize);

#[derive(Clone, Copy)]
enum VmCommand {
    Arithmetic(ArithmeticOpcode),
    Push(MemorySegment, usize),
    Pop(MemorySegment, usize),
    // Goto(SymbolId),
    // IfGoto(SymbolId),
    // Return,
}

struct VmProgram;

impl VmProgram {
    pub fn parse_from(src: &str) -> Result<Self, Box<dyn Error>> {
        Ok(Self)
    }

    fn translated(self) -> Result<String, Box<dyn Error>> {
        Ok(format!("Hello this is the code."))
    }
}

fn entry() -> Result<(), Box<dyn Error>> {
    let mut source = String::new();
    let first_filename = std::env::args().skip(1).next();
    let first_filename = first_filename.ok_or(format!("Must specify at least one .vm file."))?;
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
    let output_name = first_filename.replace(".vm", ".asm");
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
