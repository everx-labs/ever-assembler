/*
 * Copyright 2018-2023 EVERX DEV SOLUTIONS LTD.
 *
 * Licensed under the SOFTWARE EVALUATION License (the "License"); you may not use
 * this file except in compliance with the License.
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific EVERX DEV software governing permissions and
 * limitations under the License.
 */

use ever_block::{Result, SliceData};
use self::loader::Loader;

pub mod codedict;
mod handlers;
pub mod loader;
pub mod fmt;
pub mod types;

pub fn disasm(slice: &mut SliceData) -> Result<String> {
    disasm_ex(slice, false)
}

pub fn disasm_ex(slice: &mut SliceData, collapsed: bool) -> Result<String> {
    let mut loader = Loader::new(collapsed);
    let mut code = loader.load(slice, false)?;
    code.elaborate_dictpushconst_dictugetjmp();
    Ok(code.print("", true, 0))
}
