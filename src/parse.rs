use crate::vm_program::{ArithmeticOpcode, CommandName, MemorySegment, VmCommand, VmProgram};
use std::error::Error;

struct Parser<'a> {
    source: &'a str,
    file_path: &'a str,
    current_line: usize,
    current_col: usize,
    /// Where (push static 0) and (pop static 0) should go.
    static_base: usize,
    output: &'a mut VmProgram,
}

/* CONSTRUCTOR */

impl<'a> Parser<'a> {
    fn new(output: &'a mut VmProgram, source: &'a str, file_path: &'a str) -> Self {
        Self {
            source,
            file_path,
            current_line: 1,
            current_col: 1,
            // Our static variables should go after any other static variables in the program.
            static_base: output.static_size,
            output,
        }
    }
}

/* ERROR HANDLING */

type SavedPosition = (usize, usize);

impl<'a> Parser<'a> {
    fn save_pos(&self) -> SavedPosition {
        (self.current_line, self.current_col)
    }

    fn error_footer(&self, pos: SavedPosition) -> String {
        format!("\nEncountered at {}:{}:{}", self.file_path, pos.0, pos.1,)
    }

    fn expected_one_of_error_message<'i, T>(
        &self,
        pos: SavedPosition,
        expected: T,
        problem: &str,
    ) -> Box<dyn Error>
    where
        T: Iterator<Item = &'i &'i str>,
    {
        let expected_desc = expected
            .map(|s| s.to_owned())
            .collect::<Vec<_>>()
            .join(", ");
        let footer = self.error_footer(pos);
        let msg = format!(
            "{}, expected one of:\n{}.{}",
            problem, expected_desc, footer
        );
        msg.into()
    }

    fn expected_one_of_found_error_message<'i, T>(
        &self,
        pos: SavedPosition,
        expected: T,
        found: &str,
    ) -> Box<dyn Error>
    where
        T: Iterator<Item = &'i &'i str>,
    {
        let problem = format!("Found unknown symbol \"{}\"", found);
        self.expected_one_of_error_message(pos, expected, &problem)
    }
    fn expected_one_of_eof_error_message<'i, T>(&self, expected: T) -> Box<dyn Error>
    where
        T: Iterator<Item = &'i &'i str>,
    {
        let pos = self.save_pos();
        self.expected_one_of_error_message(pos, expected, "Unexpected end of file")
    }
}

/* PARSING */

pub type ParseResult<T = ()> = Result<T, Box<dyn Error>>;

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<char> {
        self.source.chars().next()
    }

    /// Panics if at the end of the file.
    fn advance(&mut self) {
        let next = self.peek().unwrap();
        // Not all characters take 1 byte.
        self.source = &self.source[next.len_utf8()..];
        if next == '\n' {
            self.current_line += 1;
            self.current_col = 1;
        } else {
            self.current_col += 1;
        }
    }

    /// Grabs the next symbol (contiguous group of characters without whitespace) and advances the
    /// internal pointer beyond that point.
    fn advance_symbol(&mut self) -> Option<(SavedPosition, &str)> {
        let mut comment = false;
        while let Some(peeked) = self.peek() {
            if comment {
                if peeked == '\n' {
                    comment = false;
                }
                self.advance()
            } else if peeked.is_whitespace() {
                self.advance()
            } else if peeked == '/' {
                if self.source.chars().skip(1).next() == Some('/') {
                    self.advance();
                    self.advance();
                    comment = true;
                } else {
                    // Just a single slash, not a comment. This is not valid syntax but acting
                    // like it is part of a symbol will trigger an appropriate error later on with
                    // context as to what we were expecting instead of a slash.
                    break;
                }
            } else {
                break;
            }
        }
        let position = self.save_pos();
        let mut end_index = 0;
        loop {
            if end_index >= self.source.len() {
                break;
            }
            if let Some(next_char) = (&self.source[end_index..]).chars().next() {
                if next_char.is_whitespace() {
                    break;
                } else {
                    end_index += next_char.len_utf8();
                }
            } else {
                break;
            }
        }
        if end_index > 0 {
            let next_symbol = &self.source[..end_index];
            // A single column can contain multiple bytes. Count characters, not length.
            // We don't need to use advance() because next_symbol does not include whitespace
            // therefore it does not include newlines, so only current_col is updated.
            self.current_col += next_symbol.chars().count();
            self.source = &self.source[end_index..];
            Some((position, next_symbol))
        } else {
            // We have reached the end of the file, there are no more non-whitespace characters
            // to return.
            None
        }
    }

    /// Parses the next command. Asserts that the current parser state is Command. Updates the
    /// parser state according to what command was read. Returns false if EOF has been reached.
    fn advance_command(&mut self) -> ParseResult<bool> {
        let next = if let Some(next) = self.advance_symbol() {
            next
        } else {
            return Ok(false);
        };
        let (pos, symbol) = next;
        let command_name = CommandName::from_name(symbol);
        if let Some(command_name) = command_name {
            self.advance_command_arguments(command_name)?;
            Ok(true)
        } else {
            let expected = CommandName::all_names()
                .iter()
                .chain(ArithmeticOpcode::all_names().iter());
            // Because lifetime problems.
            let symbol = symbol.to_owned();
            Err(self.expected_one_of_found_error_message(pos, expected, &symbol[..]))
        }
    }

    fn advance_mem_segment(&mut self) -> ParseResult<(SavedPosition, MemorySegment)> {
        if let Some((pos, symbol)) = self.advance_symbol() {
            let segment = MemorySegment::from_name(symbol);
            let symbol = symbol.to_owned();
            let segment = segment.ok_or_else(|| {
                self.expected_one_of_found_error_message(
                    pos,
                    MemorySegment::all_names().iter(),
                    &symbol[..],
                )
            })?;
            Ok((pos, segment))
        } else {
            Err(self.expected_one_of_eof_error_message(MemorySegment::all_names().iter()))
        }
    }

    fn advance_constant(&mut self) -> ParseResult<usize> {
        if let Some((pos, symbol)) = self.advance_symbol() {
            let symbol = symbol.to_owned();
            let parsed = symbol.parse::<usize>();
            let parsed = parsed.map_err(|_err| {
                let symbol = symbol.to_owned();
                let footer = self.error_footer(pos);
                format!(
                    "Expected a nonnegative integer, got \"{}\" instead.{}",
                    symbol, footer
                )
            })?;
            if parsed > 32767 {
                let symbol = symbol.to_owned();
                let footer = self.error_footer(pos);
                Err(format!(
                    "The integer \"{}\" is too big (expected 32767 or below).{}",
                    symbol, footer
                )
                .into())
            } else {
                Ok(parsed)
            }
        } else {
            let footer = self.error_footer(self.save_pos());
            Err(format!("Unexpected end of file, expected an integer.{}", footer).into())
        }
    }

    fn advance_identifier(&mut self) -> ParseResult<String> {
        if let Some((pos, symbol)) = self.advance_symbol() {
            let symbol = symbol.to_owned();
            for (idx, ch) in symbol.chars().enumerate() {
                // If it is an illegal character or it is the first character and is a number...
                if !(ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' || ch == ':')
                    || (ch.is_ascii_digit() && idx == 0)
                {
                    let footer = self.error_footer(pos);
                    return Err(format!(
                        "Encountered illegal character \'{}\' in identifier \"{}\".{}",
                        ch, symbol, footer
                    )
                    .into());
                }
            }
            Ok(symbol)
        } else {
            let footer = self.error_footer(self.save_pos());
            Err(format!("Unexpected end of file, expected an identifier.{}", footer).into())
        }
    }

    fn parse_push_pop_args(&mut self, is_push: bool) -> ParseResult {
        let (msp, memory_segment) = self.advance_mem_segment()?;
        let mut index = self.advance_constant()?;
        if let MemorySegment::Static = memory_segment {
            index += self.static_base;
            // Ensures that any files parsed later will not use this same spot to store a static
            // variable.
            self.output.increase_static_size(index + 1);
        }
        self.output.push_command(if is_push {
            VmCommand::Push(memory_segment, index)
        } else {
            if memory_segment == MemorySegment::Constant {
                let footer = self.error_footer(msp);
                Err(format!(
                    "It is illegal to pop data into the `const` segment.{}",
                    footer
                ))?;
            }
            VmCommand::Pop(memory_segment, index)
        });
        Ok(())
    }

    /// Takes us out of the Argument state assuming we have found all needed arguments.
    fn advance_command_arguments(&mut self, command: CommandName) -> ParseResult {
        match command {
            CommandName::Arithmetic(op) => self.output.push_command(VmCommand::Arithmetic(op)),
            CommandName::Call => {
                let fn_name = self.advance_identifier()?;
                let num_args = self.advance_constant()?;
                let command = VmCommand::Call { fn_name, num_args };
                self.output.push_command(command);
            }
            CommandName::Function => {
                let ident = self.advance_identifier()?;
                let num_locals = self.advance_constant()?;
                self.output.push_command(VmCommand::Label(ident));
                self.output.push_command(VmCommand::FnSetup { num_locals });
            }
            CommandName::Goto => {
                let ident = self.advance_identifier()?;
                self.output.push_command(VmCommand::Goto(ident))
            }
            CommandName::IfGoto => {
                let ident = self.advance_identifier()?;
                self.output.push_command(VmCommand::IfGoto(ident))
            }
            CommandName::Label => {
                let ident = self.advance_identifier()?;
                self.output.push_command(VmCommand::Label(ident))
            }
            CommandName::Push => self.parse_push_pop_args(true)?,
            CommandName::Pop => self.parse_push_pop_args(false)?,
            CommandName::Return => self.output.push_command(VmCommand::Return),
        }
        Ok(())
    }
}

pub fn parse(into: &mut VmProgram, source: &str, file_path: &str) -> Result<(), Box<dyn Error>> {
    let mut parser = Parser::new(into, source, file_path);
    // Parse commands until we encounter an error or there are no commands left to parse.
    while parser.advance_command()? {}
    Ok(())
}
