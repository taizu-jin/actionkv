mod cli;

use core::panic;
use std::{
    collections::HashMap,
    fs::{File, OpenOptions},
    io::{BufReader, BufWriter, Read, Seek, SeekFrom, Write},
    path::Path,
};

use byteorder::{LittleEndian, ReadBytesExt, WriteBytesExt};
pub use clap::Parser;
pub use cli::*;
use crc::{Crc, CRC_32_BZIP2};
use serde::{Deserialize, Serialize};

const CRC32: Crc<u32> = Crc::<u32>::new(&CRC_32_BZIP2);

type ByteStr = [u8];
type ByteString = Vec<u8>;

#[derive(Debug, Serialize, Deserialize)]
pub struct KeyValuePair {
    pub key: ByteString,
    pub value: ByteString,
}

#[derive(Debug)]
pub struct ActionKV {
    f: File,
    pub index: HashMap<ByteString, u64>,
}

impl ActionKV {
    pub fn open<P>(path: P) -> std::io::Result<Self>
    where
        P: AsRef<Path>,
    {
        let f = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .append(true)
            .open(path)?;
        let index = HashMap::new();

        Ok(Self { f, index })
    }

    pub fn load(&mut self) -> std::io::Result<()> {
        let mut f = BufReader::new(&mut self.f);
        loop {
            let position = f.stream_position()?;
            let maybe_kv = ActionKV::process_record(&mut f);

            let kv = match maybe_kv {
                Ok(kv) => kv,
                Err(err) => match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => {
                        break;
                    }
                    _ => return Err(err),
                },
            };

            self.index.insert(kv.key, position);
        }

        Ok(())
    }

    fn process_record<R>(f: &mut R) -> std::io::Result<KeyValuePair>
    where
        R: Read,
    {
        // To keep consistent byte ordering between file systems use explicit endian for reading
        // from files
        let saved_checksum = f.read_u32::<LittleEndian>()?;
        let key_len = f.read_u32::<LittleEndian>()?;
        let val_len = f.read_u32::<LittleEndian>()?;
        let data_len = key_len + val_len;

        let mut data = ByteString::with_capacity(data_len as usize);

        {
            f.by_ref().take(data_len as u64).read_to_end(&mut data)?;
        }

        debug_assert_eq!(data.len(), data_len as usize);

        let checksum = CRC32.checksum(&data);
        if checksum != saved_checksum {
            panic!(
                "data corruption encountered ({:08x} != {:08x})",
                checksum, saved_checksum
            );
        }

        let value = data.split_off(key_len as usize);
        let key = data;

        Ok(KeyValuePair { key, value })
    }

    pub fn insert(&mut self, key: &ByteStr, value: &ByteStr) -> std::io::Result<()> {
        let position = self.insert_but_ignore_index(key, value)?;
        self.index.insert(key.to_vec(), position);
        Ok(())
    }

    pub fn insert_but_ignore_index(
        &mut self,
        key: &ByteStr,
        value: &ByteStr,
    ) -> std::io::Result<u64> {
        let mut f = BufWriter::new(&mut self.f);

        let key_len = key.len();
        let val_len = value.len();
        let mut tmp = ByteString::with_capacity(key_len + val_len);

        for byte in key {
            tmp.push(*byte);
        }

        for byte in value {
            tmp.push(*byte);
        }

        let checksum = CRC32.checksum(&tmp);

        let next_byte = SeekFrom::End(0);
        let current_position = f.stream_position()?;
        f.seek(next_byte)?;
        // To keep consistent byte ordering between file systems use explicit endian for writing
        // to files
        f.write_u32::<LittleEndian>(checksum)?;
        f.write_u32::<LittleEndian>(key_len as u32)?;
        f.write_u32::<LittleEndian>(val_len as u32)?;
        f.write_all(&tmp)?;

        Ok(current_position)
    }

    pub fn get(&mut self, key: &ByteStr) -> std::io::Result<Option<ByteString>> {
        let position = match self.index.get(key) {
            None => return Ok(None),
            Some(position) => *position,
        };

        let kv = self.get_at(position)?;

        Ok(Some(kv.value))
    }

    fn get_at(&mut self, position: u64) -> std::io::Result<KeyValuePair> {
        let mut f = BufReader::new(&mut self.f);
        f.seek(SeekFrom::Start(position))?;
        let kv = Self::process_record(&mut f)?;

        Ok(kv)
    }

    pub fn find(&mut self, target: &ByteStr) -> std::io::Result<Option<(u64, ByteString)>> {
        let mut f = BufReader::new(&mut self.f);

        let mut found: Option<(u64, ByteString)> = None;

        loop {
            let position = f.stream_position()?;
            let maybe_kv = Self::process_record(&mut f);

            let kv = match maybe_kv {
                Ok(kv) => kv,
                Err(err) => match err.kind() {
                    std::io::ErrorKind::UnexpectedEof => break,
                    _ => return Err(err),
                },
            };

            if kv.key == target {
                found = Some((position, kv.value));
            }

            // important to keep looping until the end of the end of file
            // in case the key has been overwritten
        }

        Ok(found)
    }

    pub fn update(&mut self, key: &ByteStr, value: &ByteStr) -> std::io::Result<()> {
        self.insert(key, value)
    }

    pub fn delete(&mut self, key: &ByteStr) -> std::io::Result<()> {
        self.insert(key, b"")
    }
}
