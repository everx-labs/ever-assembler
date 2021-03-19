/*
* Copyright 2018-2020 TON DEV SOLUTIONS LTD.
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

use crate::OperationError;
use ton_types::BuilderData;

use crate::debug::{DbgNode, DbgPos};

pub trait Writer : 'static {
    fn new() -> Self;
    fn write_command(&mut self, command: &[u8], dbg: DbgNode) -> Result<(), OperationError>;
    fn write_composite_command(&mut self, code: &[u8], reference: BuilderData, pos: DbgPos, dbg: DbgNode) -> Result<(), OperationError>;
    fn finalize(self) -> (BuilderData, DbgNode);
}

pub(crate) struct CodePage0 {
    cells: Vec<BuilderData>,
    dbg: Vec<DbgNode>,
}

impl Writer for CodePage0 {
    /// Constructs new Writer
    fn new() -> Self {
        Self {
            cells: vec![BuilderData::new()],
            dbg: vec![DbgNode::new()],
        }
    }
    /// writes simple command
    fn write_command(&mut self, command: &[u8], dbg: DbgNode) -> Result<(), OperationError> {
        if !self.cells.is_empty() {
            let offset = self.cells.last().unwrap().bits_used();
            if self.cells.last_mut().unwrap().append_raw(command, command.len() * 8).is_ok() {
                self.dbg.last_mut().unwrap().inline_node(offset, dbg);
                return Ok(());
            }
        }
        let mut code = BuilderData::new();
        if code.append_raw(command, command.len() * 8).is_ok() {
            self.cells.push(code);
            self.dbg.push(dbg);
            return Ok(());
        }
        Err(OperationError::NotFitInSlice)
    }
    /// writes command with additional reference
    fn write_composite_command(
        &mut self, 
        command: &[u8], 
        reference: BuilderData,
        pos: DbgPos, 
        dbg: DbgNode,
    ) -> Result<(), OperationError> {
        if !self.cells.is_empty() {
            let mut last = self.cells.last().unwrap().clone();
            let offset = last.bits_used();
            if last.references_free() > 1 // one cell remains reserved for finalization
                && last.append_raw(command, command.len() * 8).is_ok()
                && last.checked_append_reference(reference.clone().into()).is_ok() {

                *self.cells.last_mut().unwrap() = last;

                let dbgnode = self.dbg.last_mut().unwrap();
                dbgnode.append(offset, pos.clone());

                let mut stub = dbg.clone();
                stub.children.clear();
                dbgnode.inline_node(offset + command.len() * 8, stub);

                for child in dbg.children {
                    dbgnode.append_node(child);
                }
                return Ok(());
            }
        }
        let mut code = BuilderData::new();
        let cell = reference.into();
        if code.append_raw(command, command.len() * 8).is_ok()
            && code.checked_append_reference(cell).is_ok() {
            self.cells.push(code);

            let mut node = DbgNode::new();
            node.append(0, pos);
            node.append_node(dbg);
            self.dbg.push(node);

            return Ok(());
        }
        Err(OperationError::NotFitInSlice)
    }
    /// puts every cell as a reference to the previous one
    fn finalize(mut self) -> (BuilderData, DbgNode) {
        let mut cursor = self.cells.pop().expect("cells can't be empty");
        let mut dbg = self.dbg.pop().expect("dbgs can't be empty");
        while !self.cells.is_empty() {
            let mut destination = self.cells.pop()
                .expect("vector is not empty");
            destination.append_reference(cursor);
            cursor = destination;

            let mut next = self.dbg.pop().expect("dbg vector is not empty");
            next.append_node(dbg);
            dbg = next;
        }
        (cursor, dbg)
    }
}
