/*
* Copyright (C) 2019-2022 TON Labs. All Rights Reserved.
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

use serde::{Serialize, Deserialize};
use std::collections::BTreeMap;
use ton_types::{Cell, UInt256};

pub type Lines = Vec<Line>;
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Line {
    pub text: String,
    pub pos: DbgPos
}

impl Line {
    pub fn new(text: &str, filename: &str, line: usize) -> Self {
        Line {
            text: String::from(text),
            pos: DbgPos { filename: String::from(filename), line, line_code: line }
        }
    }
    pub fn new_extended(text: &str, filename: &str, line: usize, line_code: usize) -> Self {
        Line {
            text: String::from(text),
            pos: DbgPos { filename: String::from(filename), line, line_code }
        }
    }
}

pub fn lines_to_string(lines: &Lines) -> String {
    lines
        .iter()
        .fold(String::new(), |result, line| result + line.text.as_str())
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct DbgPos {
    pub filename: String,
    pub line: usize,
    #[serde(skip)]
    pub line_code: usize,
}

impl std::fmt::Display for DbgPos {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let filename = if self.filename.is_empty() {
            "<none>"
        } else {
            self.filename.as_str()
        };
        write!(f, "{}:{}", filename, self.line)
    }
}

#[derive(Clone, Default)]
pub struct DbgNode {
    pub offsets: BTreeMap<usize, DbgPos>,
    pub children: Vec<DbgNode>,
}

impl DbgNode {
    pub fn from_ext(pos: DbgPos, dbgs: Vec<DbgNode>) -> Self {
        Self {
            offsets: BTreeMap::from([(0, pos)]),
            children: dbgs
        }
    }
    pub fn from(pos: DbgPos) -> Self {
        Self::from_ext(pos, vec!())
    }
    pub fn inline_node(&mut self, offset: usize, dbg: DbgNode) {
        for entry in dbg.offsets {
            self.offsets.insert(entry.0 + offset, entry.1);
        }
        for child in dbg.children {
            self.append_node(child);
        }
    }
    pub fn append_node(&mut self, dbg: DbgNode) {
        assert!(self.children.len() < 4);
        self.children.push(dbg)
    }
}

impl std::fmt::Display for DbgNode {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        for entry in self.offsets.iter() {
            writeln!(f, "{}:{}", entry.0, entry.1)?
        }
        write!(f, "{} children", self.children.len())
    }
}

#[derive(Default, Serialize, Deserialize)]
pub struct DbgInfo {
    map: BTreeMap<[u8; 32], BTreeMap<usize, DbgPos>>
}

impl std::fmt::Debug for DbgInfo {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map().entries(self.map.iter()).finish()
    }
}

impl DbgInfo {
    pub fn from(cell: &Cell, node: &DbgNode) -> Self {
        let mut info = DbgInfo { map: BTreeMap::new() };
        info.collect(cell, node);
        info
    }
    pub fn len(&self) -> usize {
        self.map.len()
    }
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
    pub fn append(&mut self, other: &mut Self) {
        self.map.append(&mut other.map);
    }
    pub fn insert(&mut self, key: UInt256, tree: BTreeMap<usize, DbgPos>) {
        self.map.entry(key.inner()).or_insert(tree);
    }
    pub fn remove(&mut self, key: &UInt256) -> Option<BTreeMap<usize, DbgPos>> {
        self.map.remove(key.as_slice())
    }
    pub fn get(&self, key: &UInt256) -> Option<&BTreeMap<usize, DbgPos>> {
        self.map.get(key.as_slice())
    }
    pub fn first_entry(&self) -> Option<&BTreeMap<usize, DbgPos>> {
        self.map.iter().next().map(|k_v| k_v.1)
    }
    fn collect(&mut self, cell: &Cell, dbg: &DbgNode) {
        let hash = cell.repr_hash().inner();
        // note existence of identical cells in a tree is normal
        self.map.entry(hash).or_insert_with(|| dbg.offsets.clone());
        for i in 0..cell.references_count() {
            let child_cell = cell.reference(i).unwrap();
            let child_dbg = dbg.children[i].clone();
            self.collect(&child_cell, &child_dbg);
        }
    }
}
