const BLOCK: usize = 64;

pub fn encode(entries: &[(u64, u32)], width: u32) -> Vec<u8> {
    let mut headers = Vec::new();
    let mut keys = Vec::new();
    for (index, (key, _)) in entries.iter().enumerate() {
        if index % BLOCK == 0 {
            headers.extend_from_slice(&key.to_le_bytes());
            headers.extend_from_slice(&(keys.len() as u32).to_le_bytes());
        } else {
            varint(&mut keys, key - entries[index - 1].0);
        }
    }
    let mut offsets = vec![0u8; (entries.len() * width as usize).div_ceil(8) + 8];
    for (index, (_, offset)) in entries.iter().enumerate() {
        let bit = index * width as usize;
        let word = u64::from_le_bytes(offsets[bit / 8..bit / 8 + 8].try_into().unwrap());
        let word = word | (u64::from(*offset) << (bit % 8));
        offsets[bit / 8..bit / 8 + 8].copy_from_slice(&word.to_le_bytes());
    }
    let mut out = Vec::new();
    for value in [entries.len() as u32, width, headers.len() as u32, keys.len() as u32] {
        out.extend_from_slice(&value.to_le_bytes());
    }
    out.extend(headers);
    out.extend(keys);
    out.extend(offsets);
    out
}

pub fn partition(len: usize, below: impl Fn(usize) -> bool) -> usize {
    let (mut low, mut high) = (0, len);
    while low < high {
        let middle = (low + high) / 2;
        if below(middle) {
            low = middle + 1;
        } else {
            high = middle;
        }
    }
    low
}

fn varint(out: &mut Vec<u8>, mut value: u64) {
    while value >= 0x80 {
        out.push(value as u8 | 0x80);
        value >>= 7;
    }
    out.push(value as u8);
}

pub struct Postings<'a> {
    len: usize,
    width: u32,
    headers: &'a [u8],
    keys: &'a [u8],
    offsets: &'a [u8],
}

impl<'a> Postings<'a> {
    pub fn read(bytes: &'a [u8]) -> (Postings<'a>, usize) {
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let (len, width, header_len, key_len) = (word(0), word(4) as u32, word(8), word(12));
        let offset_len = (len * width as usize).div_ceil(8) + 8;
        let headers = &bytes[16..16 + header_len];
        let keys = &bytes[16 + header_len..16 + header_len + key_len];
        let end = 16 + header_len + key_len + offset_len;
        let offsets = &bytes[16 + header_len + key_len..end];
        (Postings { len, width, headers, keys, offsets }, end)
    }

    fn first(&self, block: usize) -> u64 {
        u64::from_le_bytes(self.headers[block * 12..block * 12 + 8].try_into().unwrap())
    }

    fn start(&self, block: usize) -> usize {
        u32::from_le_bytes(self.headers[block * 12 + 8..block * 12 + 12].try_into().unwrap()) as usize
    }

    fn offset(&self, index: usize) -> u32 {
        let bit = index * self.width as usize;
        let word = u64::from_le_bytes(self.offsets[bit / 8..bit / 8 + 8].try_into().unwrap());
        ((word >> (bit % 8)) & ((1u64 << self.width) - 1)) as u32
    }

    pub fn find(&self, key: u64, out: &mut Vec<u32>) {
        let blocks = self.headers.len() / 12;
        let mut block = partition(blocks, |b| self.first(b) < key).saturating_sub(1);
        while block < blocks {
            let mut current = self.first(block);
            let mut at = self.start(block);
            let base = block * BLOCK;
            let count = BLOCK.min(self.len - base);
            for index in 0..count {
                if index > 0 {
                    let mut delta = 0u64;
                    let mut shift = 0;
                    loop {
                        let byte = self.keys[at];
                        at += 1;
                        delta |= u64::from(byte & 0x7f) << shift;
                        shift += 7;
                        if byte < 0x80 {
                            break;
                        }
                    }
                    current += delta;
                }
                if current == key {
                    out.push(self.offset(base + index));
                } else if current > key {
                    return;
                }
            }
            block += 1;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn finds_every_offset_of_a_key_across_blocks() {
        let mut entries: Vec<(u64, u32)> = (0..500u32).map(|i| (u64::from(i / 3) * 1000, i)).collect();
        entries.extend((0..200u32).map(|i| (77_000, 600 + i)));
        entries.sort_unstable();
        let bytes = encode(&entries, 10);
        let (postings, end) = Postings::read(&bytes);
        assert_eq!(end, bytes.len());
        for key in [0, 1000, 77_000, 166_000, 5] {
            let mut found = Vec::new();
            postings.find(key, &mut found);
            let expected: Vec<u32> = entries.iter().filter(|e| e.0 == key).map(|e| e.1).collect();
            assert_eq!(found, expected, "key {key}");
        }
    }
}
