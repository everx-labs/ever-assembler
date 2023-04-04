/*
 * Copyright (C) 2022 TON Labs. All Rights Reserved.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific TON DEV software governing permissions and
 * limitations under the License.
 */

use std::{error::Error, io::Write};

use ton_labs_assembler::{compile_code_debuggable, Line, DbgPos, DbgInfo};
use ton_types::{SliceData};

fn usage() {
    eprintln!("Usage: asm <code> [<boc> [<dbgmap>]]")
}

fn main() {
    let mut args = std::env::args();
    args.next(); // skip executable name
    let input = args.next().unwrap_or_else(|| {
        usage();
        std::process::exit(1)
    });
    let prefix = input.strip_suffix(".code").unwrap_or(&input);
    let output = args.next().unwrap_or_else(|| format!("{}.boc", prefix));
    let dbgmap = args.next().unwrap_or_else(|| format!("{}.dbg.json", prefix));
    if args.next().is_some() {
        usage();
        std::process::exit(2)
    }
    let lines = read(input).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(3)
    });
    let (slice, dbg) = compile_code_debuggable(lines).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(4)
    });
    write_boc(slice, &output).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(5)
    });
    println!("wrote boc to {}", output);
    write_dbg(dbg, &dbgmap).unwrap_or_else(|e| {
        eprintln!("{}", e);
        std::process::exit(6)
    });
    println!("wrote dbg to {}", dbgmap);
}

fn read(input: String) -> Result<Vec<Line>, Box<dyn Error>> {
    let mut lines = vec!();
    for (lineno, line) in std::fs::read_to_string(input.clone())?.lines().enumerate() {
        lines.push(Line {
            text: format!("{}\n", line),
            pos: DbgPos {
                filename: input.clone(),
                line: lineno,
                line_code: 0
            }
        })
    }
    Ok(lines)
}

fn write_boc(slice: SliceData, output: &str) -> Result<(), Box<dyn Error>> {
    let bytes = ton_types::write_boc(&slice.into_cell())?;
    let mut file = std::fs::File::create(output)?;
    file.write_all(&bytes)?;
    Ok(())
}

fn write_dbg(dbg: DbgInfo, output: &str) -> Result<(), Box<dyn Error>> {
    let json = serde_json::to_string_pretty(&dbg)?;
    let mut file = std::fs::File::create(output)?;
    file.write_all(json.as_bytes())?;
    Ok(())
}
