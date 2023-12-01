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

use std::collections::BTreeMap;

use crate::{compile_code_debuggable, DbgPos};
use ton_types::SliceData;

fn make_dbgpos(line: usize) -> DbgPos {
    DbgPos {
        filename: String::from("sample.code"),
        line,
    }
}

#[test]
fn test_pushcont_1() {
    let code = "
        NOP
        PUSHCONT {
            NOP
            CALLREF {
                NOP
            }
        }
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 1);
    assert_eq!(slice0.as_hex_string(), "008e8300db3c");

    let slice1 = SliceData::load_cell(slice0.reference(0).unwrap()).unwrap();
    assert_eq!(slice1.remaining_references(), 0);
    assert_eq!(slice1.as_hex_string(), "00");

    let mut map0 = BTreeMap::new();
    map0.insert( 0, make_dbgpos(2));
    map0.insert( 8, make_dbgpos(3));
    map0.insert(24, make_dbgpos(4));
    map0.insert(32, make_dbgpos(5));

    assert_eq!(dbginfo.get(&slice0.clone().into_cell().repr_hash()).unwrap(), &map0);

    let mut map1 = BTreeMap::new();
    map1.insert(0, make_dbgpos(6));

    assert_eq!(dbginfo.get(&slice1.clone().into_cell().repr_hash()).unwrap(), &map1);
}

#[test]
fn test_pushcont_2() {
    let code = "
        CALLREF {
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP ; 16
        }
        NOP NOP NOP NOP NOP NOP NOP ; 7
        ; offset 72
        PUSHCONT {
            ; offset 88
            NOP NOP NOP NOP NOP NOP NOP NOP NOP ; 9
            ; offset 160
            PUSHCONT {
                NOP
                CALLREF {
                    NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP ; 15
                }
                NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP ; 12
            }
            PUSHCONT {
                NOP ; 1
            }
            NOP NOP NOP NOP ; 4
        }
        NOP NOP NOP ; 3
    ";

    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 2);
    assert_eq!(slice0.as_hex_string(), "db3c000000000000008ea00000000000000000008e8f00db3c000000000000000000000000910000000000000000");

    let slice1 = SliceData::load_cell(slice0.reference(0).unwrap()).unwrap();
    assert_eq!(slice1.remaining_references(), 0);
    assert_eq!(slice1.as_hex_string(), "00000000000000000000000000000000");

    let slice2 = SliceData::load_cell(slice0.reference(1).unwrap()).unwrap();
    assert_eq!(slice2.remaining_references(), 0);
    assert_eq!(slice2.as_hex_string(), "000000000000000000000000000000");

    let map0 = dbginfo.get(&slice0.into_cell().repr_hash()).unwrap();
    assert_eq!(map0.get( &72).unwrap().line,  7);
    assert_eq!(map0.get( &88).unwrap().line,  9);
    assert_eq!(map0.get(&160).unwrap().line, 11);
}

#[test]
fn test_fragment_1() {
    let code = "
        .fragment foo, {
            NOP
        }
        .inline foo
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 0);
    assert_eq!(slice0.as_hex_string(), "00");

    let mut map0 = BTreeMap::new();
    map0.insert(0, make_dbgpos(3));

    assert_eq!(dbginfo.get(&slice0.clone().into_cell().repr_hash()).unwrap(), &map0);
}

#[test]
fn test_fragment_2() {
    let code = "
        .fragment foo, {
            .loc sample.sol, 13
            NOP
            .loc sample.sol, 14
            NOP
        }
        .inline foo
        NOP
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 0);
    assert_eq!(slice0.as_hex_string(), "000000");

    let mut map0 = BTreeMap::new();
    map0.insert( 0, DbgPos { filename: String::from("sample.sol"), line: 13 });
    map0.insert( 8, DbgPos { filename: String::from("sample.sol"), line: 14 });
    map0.insert(16, DbgPos { filename: String::from("sample.code"), line: 9 });

    assert_eq!(dbginfo.get(&slice0.clone().into_cell().repr_hash()).unwrap(), &map0);
}

#[test]
fn test_codedict_1() {
    let code = "
        .fragment foo, {
            NOP
        }
        .code-dict-cell 19, {
            xaaaab_ = foo,
        }
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 1);
    assert_eq!(slice0.remaining_bits(), 0);

    let cell1 = slice0.reference(0).unwrap();

    let slice1 = SliceData::load_cell(cell1).unwrap();
    assert_eq!(slice1.remaining_references(), 0);
    assert_eq!(slice1.as_hex_string(), "a75555402_");

    let mut map1 = BTreeMap::new();
    map1.insert(26, make_dbgpos(3));

    assert_eq!(dbginfo.get(&slice1.clone().into_cell().repr_hash()).unwrap(), &map1);
}

#[test]
fn test_codedict_2() {
    let code = "
        .fragment foo, {
            NOP
        }
        .fragment bar, {
            NOP
            NOP
        }
        .code-dict-cell 19, {
            xaaaab_ = foo,
            xaaabb_ = bar,
        }
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 1);
    assert_eq!(slice0.remaining_bits(), 0);

    let cell1 = slice0.reference(0).unwrap();
    let slice1 = SliceData::load_cell(cell1).unwrap();
    assert_eq!(slice1.remaining_references(), 2);
    assert_eq!(slice1.as_hex_string(), "9f5556_");

    let cell2 = slice1.reference(0).unwrap();
    let slice2 = SliceData::load_cell_ref(&cell2).unwrap();
    assert_eq!(slice2.remaining_references(), 0);
    assert_eq!(slice2.as_hex_string(), "ba01_");

    let cell3 = slice1.reference(1).unwrap();
    let slice3 = SliceData::load_cell_ref(&cell3).unwrap();
    assert_eq!(slice3.remaining_references(), 0);
    assert_eq!(slice3.as_hex_string(), "ba0001_");

    let mut map2 = BTreeMap::new();
    map2.insert(7, make_dbgpos(3));
    assert_eq!(dbginfo.get(&cell2.repr_hash()).unwrap(), &map2);

    let mut map3 = BTreeMap::new();
    map3.insert(7, make_dbgpos(6));
    map3.insert(15, make_dbgpos(7));
    assert_eq!(dbginfo.get(&cell3.repr_hash()).unwrap(), &map3);
}

#[test]
fn test_codedict_3() {
    let code = "
        .fragment foo, {
            ; 127 bytes
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
            NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP NOP
        }
        .code-dict-cell 19, {
            xaaaab_ = foo,
        }
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 1);
    assert_eq!(slice0.remaining_bits(), 0);

    let cell1 = slice0.reference(0).unwrap();

    let slice1 = SliceData::load_cell(cell1).unwrap();
    assert_eq!(slice1.remaining_references(), 1);
    assert_eq!(slice1.as_hex_string(), "a755556_");

    let cell2 = slice1.reference(0).unwrap();
    let slice2 = SliceData::load_cell_ref(&cell2).unwrap();
    assert_eq!(slice2.remaining_references(), 0);
    assert_eq!(slice2.as_hex_string(), "00".repeat(127));

    let mut map = BTreeMap::new();
    let mut offset = 0;
    while offset < 1016 {
        map.insert(offset, make_dbgpos(4 + offset / (8 * 16)));
        offset += 8;
    }

    assert_eq!(dbginfo.get(&cell2.repr_hash()).unwrap(), &map);
}

#[test]
fn test_computed_cell_1() {
    let code = "
        .fragment foo, {
            NEWC STONE ENDC
            NEWC STREF ENDC
        }
        .inline-computed-cell foo, 0x0
    ";
    let (slice0, dbginfo) = compile_code_debuggable(code, "sample.code").unwrap();
    assert_eq!(slice0.remaining_references(), 1);
    assert_eq!(slice0.remaining_bits(), 0);

    let map0 = BTreeMap::new();
    assert_eq!(dbginfo.get(&slice0.clone().into_cell().repr_hash()).unwrap(), &map0);

    let cell1 = slice0.reference(0).unwrap();
    let slice1 = SliceData::load_cell_ref(&cell1).unwrap();
    assert_eq!(slice1.remaining_references(), 0);
    assert_eq!(slice1.as_hex_string(), "c_");

    let map1 = BTreeMap::new();
    assert_eq!(dbginfo.get(&cell1.repr_hash()).unwrap(), &map1);
}

#[test]
fn test_library_cell() {
    let code = "
        .library-cell 81bf51b6da362217e4ec557c52f2eae028e7c83abf64175ef127ba3c6b67a1be
    ";
    compile_code_debuggable(code, "sample.code").unwrap();
}
