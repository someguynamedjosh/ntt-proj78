use crate::vm_program::VmProgram;
use std::{error::Error, path::Path};

mod parse;
mod translate;
mod vm_program;

fn add_file(to: &mut VmProgram, path: &Path) -> Result<(), Box<dyn Error>> {
    let path_str = path.to_string_lossy().to_owned();
    let path_str = &path_str[..];
    let contents = std::fs::read_to_string(path);
    let contents =
        contents.map_err(|err| format!("Failed to open \"{}\", caused by:\n{}", path_str, err))?;
    parse::parse(to, &contents[..], path_str)?;
    Ok(())
}

fn entry() -> Result<(), Box<dyn Error>> {
    let arg1 = std::env::args().skip(1).next();
    let source_path_str = arg1.ok_or(format!("Must specify a file or folder."))?;
    let source_path = Path::new(&source_path_str[..]);

    let mut program = VmProgram::new();
    if source_path.is_file() {
        if !source_path_str.ends_with(".vm") {
            Err(format!(
                "The file \"{}\" has the wrong extension (expected .vm).",
                source_path_str
            ))?;
        }
        add_file(&mut program, source_path)?;
    } else {
        let reader = source_path.read_dir();
        let reader = reader.map_err(|err| {
            format!(
                "Failed to view directory \"{}\", caused by:\n{}",
                source_path_str, err
            )
        })?;
        let mut any = false;
        for entry in reader {
            let entry = entry.map_err(|err| {
                format!(
                    "Failed to view item in directory \"{}\", caused by:\n{}",
                    source_path_str, err
                )
            })?;
            let path = entry.path();
            // I dont know why this is necessary VVVVVVVVVVVVVVVVVVVVVVVVVVVVVV but hey it works.
            if path.is_file() && path.extension().map(|ext| ext == "vm") == Some(true) {
                println!("Including file {}...", path.to_string_lossy());
                add_file(&mut program, &path)?;
                any = true;
            }
        }
        if !any {
            return Err(format!("The provided directory contains no .vm files.").into());
        }
    }

    // Optional printing of intermediate representation.
    if cfg!(feature = "dump") {
        println!("\nInternal Representation:\n{:#?}\n", program);
    }
    let result = translate::translate(program)?;
    if cfg!(feature = "dump") {
        println!("Translated Program:\n{}\n", result);
    }

    let output_path = if source_path.is_file() {
        source_path.with_extension("asm")
    } else {
        let folder_name = source_path.file_name().unwrap().to_string_lossy();
        // A file inside the folder called FolderName.asm
        source_path.join(format!("{}.asm", folder_name))
    };
    let result = std::fs::write(&output_path, result);
    let output_path = output_path.to_string_lossy();
    result.map_err(|err| {
        format!(
            "Failed to write result to \"{}\", caused by:\n{:?}",
            output_path, err
        )
    })?;
    println!("Wrote output to \"{}\"", output_path);
    Ok(())
}

fn main() {
    match entry() {
        Ok(_) => {
            println!("Operation completed sucessfully.");
            std::process::exit(0);
        }
        Err(err) => {
            eprintln!("Encountered an error:\n{}", err);
            drop(err);
            std::process::exit(1);
        }
    }
}
