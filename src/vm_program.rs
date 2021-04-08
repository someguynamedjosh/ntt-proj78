/// Creates an enum with a public function `from_name` that returns the corresponding enum variant
/// given a matching string.
macro_rules! keyword_enum {
    ($EnumName:ident {
        $($EnumVariantName:ident $name_in_source:ident),*$(,)?
    }) => {
        #[derive(Clone, Copy)]
        pub enum $EnumName {
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
pub struct SymbolId(usize);

#[derive(Clone, Copy)]
pub enum VmCommand {
    Arithmetic(ArithmeticOpcode),
    Push(MemorySegment, usize),
    Pop(MemorySegment, usize),
    // Goto(SymbolId),
    // IfGoto(SymbolId),
    // Return,
}

pub struct VmProgram;