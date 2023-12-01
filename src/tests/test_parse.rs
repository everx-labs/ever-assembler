/*
* Copyright (C) 2019-2021 TON Labs. All Rights Reserved.
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

use crate::{
    compile_code, complex::compile_slice, parse::parse_slice,
    CompileError, Position, Engine
};
use ton_types::{BuilderData, IBitstring, SliceData};

#[test]
fn test_parseslice() {
    assert_eq!(parse_slice("x8",    4), Ok(vec![8, 0x80]));
    assert_eq!(parse_slice("x8_",   4), Ok(vec![8]));
    assert_eq!(parse_slice("x8A",   4), Ok(vec![8, 0xA8]));
    assert_eq!(parse_slice("x8A_",  4), Ok(vec![8, 0xA0]));
    assert_eq!(parse_slice("x8A0",  4), Ok(vec![8, 0xA0, 0x80]));
    assert_eq!(parse_slice("x8A0_", 4), Ok(vec![8, 0xA0]));

    assert_eq!(parse_slice("x8",    0), Ok(vec![0x88]));
    assert_eq!(parse_slice("x8_",   0), Ok(vec![0x80]));
    assert_eq!(parse_slice("x8A",   0), Ok(vec![0x8A, 0x80]));
    assert_eq!(parse_slice("x8A_",  0), Ok(vec![0x8A]));
    assert_eq!(parse_slice("x8A0",  0), Ok(vec![0x8A, 0x08]));
    assert_eq!(parse_slice("x8A0_", 0), Ok(vec![0x8A]));
}

#[test]
fn test_parseslice_full() {
    fn check_slice(value: usize, digits: usize, compl: bool) {
        for i in 0..8 {
            let mut b = BuilderData::new();
            for _ in 0..i {
                b.append_bit_zero().unwrap();
            }
            b.append_bits(value, digits * 4).unwrap();
            let s = if compl {
                b.append_bit_one().unwrap();
                format!("x{:0X}", value)
            } else {
                format!("x{:0X}_", value)
            };

            let mut b = SliceData::load_builder(b).unwrap();
            let len = b.remaining_bits();
            let mut vec = b.get_next_bits(len).unwrap();
            if vec.last().unwrap() == &0 {
                vec.pop().unwrap();
            }
            println!("digits: {} bits {}: {} == {:?}", digits, i, s, vec);
            assert_eq!(parse_slice(&s[..], i), Ok(vec));
        }
    }
    let mut end = 0x10;
    for digits in 1..=1 {
        for i in 1..end {
            check_slice(i, digits, false);
            check_slice(i, digits, true);
        }
        end <<= 8;
    }
}

#[test]
fn test_pushslice() {
    assert_eq!(compile_code("PUSHSLICE x5").unwrap().get_bytestring(0), vec![0x8B, 0x15, 0x80]);
    assert_eq!(compile_code("PUSHSLICE x5_").unwrap().get_bytestring(0), vec![0x8B, 0x05]);
    assert_eq!(compile_code("PUSHSLICE x0123456789012345678901234567890_").unwrap().get_bytestring(0),
        vec![0x8B, 0xF0,
        0x12, 0x34, 0x56, 0x78, 0x90,
        0x12, 0x34, 0x56, 0x78, 0x90,
        0x12, 0x34, 0x56, 0x78, 0x90]
    );
}

#[test]
fn test_compile_slice() {
    let buffer = compile_slice("x77998_", vec![0xD7, 0x28], 14, 0, 7).unwrap();
    assert_eq!(buffer, vec![0xD7, 0x28, 0x13, 0xBC, 0xCC]);

    let buffer = compile_slice("x77998_", vec![0xCF, 0x80], 9, 2, 3).unwrap();
    assert_eq!(buffer, vec![0xCF, 0x89, 0xDE, 0x66]);
}

#[test]
fn test_incorrect_command() {
    assert_eq!(compile_code("AAAAAA").err().unwrap(), CompileError::UnknownOperation{ 0: Position {
        filename: "".to_string(),
        line: 1,
        column: 1
    }, 1: "AAAAAA".to_string() });
}

#[test]
fn test_comments_parsing_bug() {
    // https://github.com/tonlabs/ton-labs-assembler/issues/28
    let code = super::compile_code("
        PUSHINT 7; we push 7 on the stack
        PUSHINT; we push 15 now
           15
        DROP; we drop the 15 from the stack, keeping 7
    ").unwrap();
    // 77   PUSHINT 7
    // 800f PUSHINT 15
    // 30   DROP
    assert_eq!(code.as_hex_string(), "77800f30");
}

use std::ops::Range;
fn prepare_slice(range: Range<u8>) -> String {
    range.map(|x| { format!("{:02X}", x) }).collect::<Vec<_>>().join("")
}

mod test_sdbegins {
    use super::*;

    #[test]
    fn test_extra_tag_expl() {
        let slice = prepare_slice(0..127);
        let code = format!("SDBEGINS x{}F_", slice);
        compile_code(&code[..]).expect_err("OutOfRange");
    }

    #[test]
    fn test_extra_tag_impl() {
        let slice = prepare_slice(0..127);
        let code = format!("SDBEGINS x{}A", slice);
        compile_code(&code[..]).expect_err("OutOfRange");
    }

    #[test]
    fn test_max() {
        let slice = prepare_slice(0..124);
        let code = format!("SDBEGINS x{}A", slice);
        compile_code(&code[..]).expect_err("NotFitInSlice");

        let code = format!("SDBEGINS x{}F_", slice);
        compile_code(&code[..]).expect_err("NotFitInSlice");

        let code = format!("SDBEGINS x{}A_", slice);
        let bytecode = compile_code(&code[..]).unwrap().get_bytestring(0);
        assert_eq!(&bytecode[..4], &[0xD7, 0x2B, 0xE0, 0x00]);
    }
}

mod test_pushslice {
    use super::*;

    #[test]
    fn test_short_max() {
        let slice = prepare_slice(0..15);
        let code = format!("PUSHSLICE x{}F_", slice);
        let bytecode = compile_code(&code[..]).unwrap().get_bytestring(0);
        assert_eq!(&bytecode.as_slice()[..4], &[0x8B, 0xF0, 0x00, 0x10]);
    }
    #[test]
    fn test_max() {
        let slice = prepare_slice(0..124);
        let code = format!("PUSHSLICE x{}FE_", slice);
        compile_code(&code[..]).expect_err("NotFitInSlice");

        let code = format!("PUSHSLICE x{}FC", slice);
        compile_code(&code[..]).expect_err("NotFitInSlice");

        let code = format!("PUSHSLICE x{}FC_", slice);
        let bytecode = compile_code(&code[..]).unwrap().get_bytestring(0);
        assert_eq!(&bytecode.as_slice()[..4], &[0x8D, 0x1F, 0x00, 0x00]);
    }
}

mod test_stsliceconst {
    use super::*;

    #[test]
    fn test_extra_tag_expl() {
        let slice = prepare_slice(0..7);
        let code = format!("STSLICECONST x{}E_", slice);
        compile_code(&code[..]).expect_err("OutOfRange");
    }

    #[test]
    fn test_extra_tag_impl() {
        let slice = prepare_slice(0..7);
        let code = format!("STSLICECONST x{}C", slice);
        compile_code(&code[..]).expect_err("OutOfRange");
    }

    #[test]
    fn test_max() {
        let slice = prepare_slice(0..7);
        let code = format!("STSLICECONST x{}C_", slice);
        let bytecode = compile_code(&code[..]).unwrap().get_bytestring(0);
        assert_eq!(&bytecode.as_slice()[..3], &[0xCF, 0x9C, 0x00]);
    }
}

#[test]
fn test_ifrefelseref() {
    let code = compile_code("
        IFREFELSEREF
        {
            THROW 100
        }
        {
            THROW 200
        }
    ").unwrap();
    assert_eq!(code.get_references(), 0..2);
    assert_eq!(code.to_hex_string(), "e30f");
    assert_eq!(code.reference(0).unwrap().references_count(), 0);
    assert_eq!(code.reference(0).unwrap().to_hex_string(true), "f2c064");
    assert_eq!(code.reference(1).unwrap().references_count(), 0);
    assert_eq!(code.reference(1).unwrap().to_hex_string(true), "f2c0c8");
}


mod test_dbginfo {
    use crate::debug::{DbgInfo, DbgPos, DbgNode};

    #[test]
    fn test_serdes() {
        let mut b = ton_types::BuilderData::new();
        b.append_raw(&[0x00, 0x01, 0x02, 0x03], 32).unwrap();
        let c = b.into_cell().unwrap();

        let mut m = Vec::new();
        let filename = String::from("sample.sol");
        m.push((0,  DbgPos { filename: filename.clone(), line: 1 }));
        m.push((16, DbgPos { filename: filename.clone(), line: 2 }));
        m.push((24, DbgPos { filename: filename.clone(), line: 3 }));
        let n = DbgNode {
            offsets: m,
            children: vec!()
        };

        let d1 = DbgInfo::from(c, n);
        let s1 = serde_json::to_string_pretty(&d1).unwrap();
        let d2: DbgInfo = serde_json::from_str(&s1).unwrap();
        assert_eq!(d1, d2);

        let j = serde_json::json!({
            "c8ca3b71f4d9dfe1108ad4fc7abf15b48af662b8ef3cf6927a0c8cd7035f3f43": {
                "0": {
                    "filename": filename,
                    "line": 1
                },
                "16": {
                    "filename": filename,
                    "line": 2
                },
                "24": {
                    "filename": filename,
                    "line": 3
                }
            }
        });
        let d3: DbgInfo = serde_json::from_value(j).unwrap();
        assert_eq!(d1, d3);
    }
}

#[test]
fn test_inline() {
    let mut assembler = Engine::new("sample.code");
    let code1 = "
        PUSHINT 1
        PUSHINT 2
    ";
    assembler.build(Some("1".to_string()), code1).unwrap();
    let code2 = "
        PUSHCONT {
            .inline 1
        }
    ";
    let unit = assembler.build(Some("2".to_string()), code2).unwrap();
    assert_eq!(hex::decode("927172").unwrap(), unit.finalize().0.cell().data());
}
