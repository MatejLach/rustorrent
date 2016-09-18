use std::ops::{Index, IndexMut};
use sha1::{Sha1, Digest};
use metainfo::FileInfo;

pub struct PartialFile {
    collection: PieceCollection,
    info: FileInfo
}

impl PartialFile {
    pub fn new(info: &FileInfo) -> PartialFile {
        PartialFile {
            info: info.clone(),
            collection: PieceCollection::new(info.piece_length as usize,
                info.pieces.len() as u64)
        }
    }

    pub fn is_complete(&self) -> bool {
        for i in 0..self.info.pieces.len() {
            if !self._is_piece_complete(i) {
                return false;
            }
        }
        true
    }

    fn _is_piece_complete(&self, i: usize) -> bool {
        let mut sha1: Sha1 = Sha1::new();
        sha1.update(&self.collection[i]); 
        let ref bytes1 = sha1.digest().bytes();
        let ref bytes2 = self.info.pieces[i];
        bytes1 == bytes2.as_slice()
    }


    pub fn bit_array(&self) -> Vec<bool> {
        (0..self.info.pieces.len())
            .map(|i| self._is_piece_complete(i))
            .collect::<Vec<bool>>()
    }

    pub fn add_piece(&mut self, index: usize, offset: usize, block: Vec<u8>) -> bool {
        self.collection.add(index, offset, block)
    }
}

struct PieceCollection {
    pieces: Vec<Vec<u8>>,
    piece_size: u64
}

impl PieceCollection {
    pub fn new(pieces: usize, size: u64) -> PieceCollection {
        let mut vec = Vec::new();
        for i in 0..pieces {
            vec.push(Vec::new());
        }
        PieceCollection { pieces: vec, piece_size: size } 
    }

    pub fn add(&mut self, index: usize, offset: usize, block: Vec<u8>) -> bool {
        if index >= self.pieces.len() { return false; }
        if offset + block.len() > self.piece_size as usize {
            return false;
        }

        let existing_block = &mut self.pieces[index];
        existing_block.resize(offset + block.len(), 0);
        for i in 0..block.len() {
            existing_block[offset + i as usize] = block[i];
        }
        true
    }
}

impl Index<usize> for PieceCollection {
    type Output = Vec<u8>;

    fn index<'a>(&'a self, _index: usize) -> &'a Vec<u8> {
        &self.pieces[_index]
    }
}

impl IndexMut<usize> for PieceCollection {
    fn index_mut<'a>(&'a mut self, _index: usize) -> &'a mut Vec<u8> {
        &mut self.pieces[_index]
    }
}
