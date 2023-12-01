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
    compile_code,
};

const COMMAND_NAMES: [&'static str; 15] = [
    "DIVMOD",        //  0
    "MOD",           //  1: !Q
    "DIV",           //  2: !R
    "RSHIFTMOD",     //  3: div-by-shift
    "MODPOW2",       //  4: !Q + div-by-shift
    "RSHIFT",        //  5: !R + div-by-shift
    "MULDIVMOD",     //  6: premultiply
    "MULMOD",        //  7: !Q + premultiply
    "MULDIV",        //  8: !R + premultiply
    "MULRSHIFTMOD",  //  9: premultiply + div-by-shift
    "MULMODPOW2",    // 10: !Q + premultiply + div-by-shift
    "MULRSHIFT",     // 11: !R + premultiply + div-by-shift
    "LSHIFTDIVMOD",  // 12: premultiply + mul-by-shift
    "LSHIFTMOD",     // 13: !Q + premultiply + mul-by-shift
    "LSHIFTDIV",     // 14: !R + premultiply + mul-by-shift
];

const ROUNDING_MODE_SUFFIXES: [&'static str; 3] = [
    "",  // Floor
    "R", // Nearest integer
    "C"  // Ceiling
];

const PRE_MULTIPLICATION: u8                    = 0b10000000;
const MULTIPLICATION_REPLACED_BY_LEFT_SHIFT: u8 = 0b01000000;
const DIVISION_REPLACED_BY_RIGHT_SHIFT: u8      = 0b00100000;
const SHIFT_OPERATION_PARAMETER_PASSED: u8      = 0b00010000;
const REMAINDER_RESULT_REQUIRED: u8             = 0b00001000;
const QUOTIENT_RESULT_REQUIRED: u8              = 0b00000100;
const ROUNDING_MODE: u8                         = 0b00000011;

fn contains(flags: u8, bitmask: u8) -> bool {
    flags & bitmask == bitmask
}

fn is_valid(flags: u8) -> bool {
    !contains(flags, DIVISION_REPLACED_BY_RIGHT_SHIFT | MULTIPLICATION_REPLACED_BY_LEFT_SHIFT)
        && !contains(flags, ROUNDING_MODE)
        && (contains(flags, QUOTIENT_RESULT_REQUIRED) || contains(flags, REMAINDER_RESULT_REQUIRED))
        && (!contains(flags, MULTIPLICATION_REPLACED_BY_LEFT_SHIFT) || contains(flags, PRE_MULTIPLICATION))
        && (!contains(flags, SHIFT_OPERATION_PARAMETER_PASSED)
            || contains(flags, MULTIPLICATION_REPLACED_BY_LEFT_SHIFT)
            || contains(flags, DIVISION_REPLACED_BY_RIGHT_SHIFT))
}

fn get_command(flags: u8, quiet: bool) -> String {
    let mut index = 0;
    if contains(flags, PRE_MULTIPLICATION) {
        index += 6;
    }
    if contains(flags, PRE_MULTIPLICATION | MULTIPLICATION_REPLACED_BY_LEFT_SHIFT) {
        index += 6
    }
    if contains(flags, DIVISION_REPLACED_BY_RIGHT_SHIFT) {
        index += 3
    }
    if !contains(flags, REMAINDER_RESULT_REQUIRED) {
        index += 2
    } else if !contains(flags, QUOTIENT_RESULT_REQUIRED) {
        index += 1
    }

    let cmd_prefix = match quiet {
        true => "Q",
        false => "",
    };

    let cmd_suffix = if contains(flags, SHIFT_OPERATION_PARAMETER_PASSED) {
        " 2"
    } else {
        ""
    };

    [
        cmd_prefix,
        COMMAND_NAMES[index],
        ROUNDING_MODE_SUFFIXES[(flags & ROUNDING_MODE) as usize],
        cmd_suffix
    ].concat()
}

fn test_primitive_compilation(flags: u8, quiet: bool) {
    let command = get_command(flags, quiet);
    match compile_code(command.as_str()) {
        Ok(actual_bytecode) => {
            let expected_bytecode = render_to_bytecode(flags, quiet);
            println!("Flags: {:#010b}, Cmd: \"{}\", Expected Bytecode: <{:02X?}>, Actual Bytecode: <{:02X?}>",
                     flags, command, expected_bytecode, actual_bytecode);
            assert_eq!(0, actual_bytecode.remaining_references());
            assert_eq!(&expected_bytecode, &actual_bytecode.get_bytestring(0));
        }
        Err(e) => {
            panic!("Flags: {:#010b}, Cmd: {}, Error: {}", flags, command, e);
        },
    };
}

fn render_to_bytecode(flags: u8, quiet: bool) -> Vec<u8> {
    let mut res = Vec::<u8>::with_capacity(5);
    if quiet {
        res.push(0xB7);
    }

    let (search_result, bytecode) = search_for_exclusion(flags);
    if search_result {
        res.extend(bytecode.iter());
    } else {
        res.push(0xA9);
        res.push(flags);
    }
    if contains(flags, SHIFT_OPERATION_PARAMETER_PASSED) {
        res.push(0x01);
    }

    res
}

fn search_for_exclusion(flags: u8) -> (bool, &'static [u8]) {
    const EXCLUSIONS: [(u8, &'static [u8]); 2] = [
        (0b00100100, &[0xAD]),        // RSHIFT
        (0b00110100, &[0xAB]),        // RSHIFT cc + 1
    ];

    for (ex_flags, ex_bytecode) in &EXCLUSIONS {
        if ex_flags == &flags {
            return (true, &ex_bytecode);
        }
    }
    return (false, &[]);
}

#[test]
fn test_division_primitives_compilation() {
    let mut count = 0;
    for flags in 0..=0b11111111 {
        if !is_valid(flags) {
            println!("Flags: {:#010b}, Cmd: <NOT IMPLEMENTED>", flags);
            continue;
        }
        test_primitive_compilation(flags, false);
        test_primitive_compilation(flags, true);
        if !contains(flags, SHIFT_OPERATION_PARAMETER_PASSED) {
            count += 1;
        }
    }
    assert_eq!(45, count);
}
