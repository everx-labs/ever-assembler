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

use super::*;
use ton_types::Result;
use num::{bigint::Sign, BigInt, Num};

/// Decodes value from big endian octet string for PUSHINT primitive using the format
/// from TVM Spec A.3.1:
///  "82lxxx — PUSHINT xxx, where 5-bit 0 ≤ l ≤ 30 determines the length n = 8l + 19
///  of signed big-endian integer xxx. The total length of this instruction
///  is l + 4 bytes or n + 13 = 8l + 32 bits."
fn from_big_endian_octet_stream(mut get_next_byte: impl FnMut() -> Result<u8>) -> ton_types::Result<BigInt> {
    let first_byte = get_next_byte()?;
    let byte_len = ((first_byte & 0b11111000u8) as usize >> 3) + 3;
    let greatest3bits = (first_byte & 0b111) as u32;
    let digit_count = (byte_len + 3) >> 2;
    let mut digits: Vec<u32> = vec![0; digit_count];
    let (sign, mut value) = if greatest3bits & 0b100 == 0 {
        (Sign::Plus, greatest3bits)
    } else {
        (Sign::Minus, 0xFFFF_FFF8u32 | greatest3bits)
    };

    let mut upper = byte_len & 0b11;
    if upper == 0 {
        upper = 4;
    }
    for _ in 1..upper {
        value <<= 8;
        value |= get_next_byte()? as u32;
    }
    let last_index = digit_count - 1;
    digits[last_index] = value;

    for i in (0..last_index).rev() {
        let mut value = (get_next_byte()? as u32) << 24;
        value |= (get_next_byte()? as u32) << 16;
        value |= (get_next_byte()? as u32) << 8;
        value |= get_next_byte()? as u32;

        digits[i] = value;
    }

    if sign == Sign::Minus {
        twos_complement(&mut digits);
    }
    Ok(BigInt::new(sign, digits))
}

/// Perform in-place two's complement of the given digit iterator
/// starting from the least significant byte.
#[inline]
fn twos_complement<'a>(digits: impl IntoIterator<Item = &'a mut u32>) {
    let mut carry = true;
    for d in digits {
        *d = !*d;
        if carry {
            *d = d.wrapping_add(1);
            carry = d == &0;
        }
    }
}

#[test]
fn decimal_from_str_success() {
    test("0", &[0x00, 0x00, 0x00]);
    test("12345678", &[0x08, 0xBC, 0x61, 0x4E]);
    test("-12345678", &[0x0F, 0x43, 0x9E, 0xB2]);
    test("1234567", &[0x08, 0x12, 0xD6, 0x87]);
    test("-1234567", &[0x0F, 0xED, 0x29, 0x79]);
    test("65535", &[0x00, 0xFF, 0xFF]);
    test("65536", &[0x01, 0x00, 0x00]);
    test("131072", &[0x02, 0x00, 0x00]);
    test("262144", &[0x08, 0x04, 0x00, 0x00]);
    test("4294967296", &[0x11, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn decimal_from_str_big_negative() {
    test("-123456789123456789123456789123456789",
        &[
            0x6F, 0xE8, 0x39, 0x1C, 0x3F, 0xCD, 0x07, 0x6F,
            0xBA, 0x52, 0x8B, 0x99, 0x7B, 0xFB, 0xA0, 0xEB
        ]);
    test("-123456789123456789123456789123456789123456789123456789",
         &[
             0xA6, 0xB6, 0x07, 0x6F, 0xDF, 0xB4, 0xDB,
             0x4A, 0x86, 0x86, 0xd7, 0xB2, 0xBF, 0x9F, 0x1E,
             0xF8, 0x17, 0x8F, 0x49, 0x7B, 0xFB, 0xA0, 0xEB
         ]);
}

#[test]
fn decimal_from_str_big_positive() {
    test("123456789123456789123456789123456789",
        &[
            0x68, 0x17, 0xC6, 0xE3, 0xC0, 0x32, 0xF8, 0x90,
            0x45, 0xAD, 0x74, 0x66, 0x84, 0x04, 0x5F, 0x15
        ]);
    test("123456789123456789123456789123456789123456789123456789",
         &[
             0xA1, 0x49, 0xF8, 0x90, 0x20, 0x4B, 0x24,
             0xB5, 0x79, 0x79, 0x28, 0x4D, 0x40, 0x60, 0xE1,
             0x07, 0xE8, 0x70, 0xB6, 0x84, 0x04, 0x5F, 0x15
         ]);
}

#[test]
fn decimal_from_str_256_bit_positive() {
    test("115792089237316195423570985008687907853269984665640564039457584007913129639935",
         &[
             0xF0,
             0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
             0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
             0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF,
             0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF, 0xFF
         ]);
}

#[test]
fn decimal_from_str_256_bit_negative() {
    test("-115792089237316195423570985008687907853269984665640564039457584007913129639935",
        &[
            0xF7,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x01
        ]);
}

#[test]
fn decimal_from_str_256_bit_negative_2() {
    test("-115792089237316195423570985008687907853269984665640564039457584007913129639936",
         &[
             0xF7,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
             0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00
         ]);
}

#[test]
fn decimal_from_str_overflow() {
    test_overflow("115792089237316195423570985008687907853269984665640564039457584007913129639936");
    test_overflow("-115792089237316195423570985008687907853269984665640564039457584007913129639937");
}

fn test(literal: &'static str, expected_bytes: &[u8]) {
    let value = BigInt::from_str_radix(literal, 10).unwrap();
    let actual_bytes = to_big_endian_octet_string(&value).unwrap();
    println!("expected len = {}, actual len = {}", expected_bytes.len(), actual_bytes.len());
    println!("actual  : {:02X?}", actual_bytes);
    println!("expected: {:02X?}", expected_bytes);
    assert_eq!(expected_bytes.len(), actual_bytes.len());
    for (a, b) in expected_bytes.iter().zip(actual_bytes.iter()) {
        assert_eq!(a, b);
    }
}

fn test_overflow(literal: &'static str) {
    let value = BigInt::from_str_radix(literal, 10).unwrap();
    assert!(to_big_endian_octet_string(&value).is_none());
}

#[test]
fn test_serialization_deserialization_same_result() {
    test_ser_deser("0");
    test_ser_deser("1");
    test_ser_deser("-1");
    test_ser_deser("5");
    test_ser_deser("-5");
    test_ser_deser("255");
    test_ser_deser("-255");
    test_ser_deser("-256");
    test_ser_deser("65535");
    test_ser_deser("65536");
    test_ser_deser("4294967296");
    test_ser_deser("-12345678");
    test_ser_deser("123456789123456789123456789123456789");
    test_ser_deser("-123456789123456789123456789123456789");
    test_ser_deser("123456789123456789123456789123456789123456789123456789");
    test_ser_deser("-123456789123456789123456789123456789123456789123456789");
}

fn test_ser_deser(literal: &'static str) {
    println!("{}", literal);
    let value = BigInt::from_str_radix(literal, 10).unwrap();
    let serialized = to_big_endian_octet_string(&value).unwrap();
    let mut iterator = serialized.iter();
    let deserialized = from_big_endian_octet_stream(|| Ok(*iterator.next().unwrap())).unwrap();
    assert_eq!(iterator.next(), None);
    assert_eq!(value, deserialized, "Failed case: {}", literal);
}
