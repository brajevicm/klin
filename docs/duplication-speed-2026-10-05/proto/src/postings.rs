const SAMPLE: usize = 256;

pub fn encode(entries: &[(u64, u32)], width: u32) -> Vec<u8> {
    let n = entries.len().max(1) as u64;
    let top = entries.last().map_or(0, |entry| entry.0);
    let low_width = (top / n).checked_ilog2().unwrap_or(0).min(56);
    let high_bits = entries.len() + (top >> low_width) as usize + 1;
    let mut highs = vec![0u64; high_bits.div_ceil(64)];
    let mut lows = vec![0u8; (entries.len() * low_width as usize).div_ceil(8) + 8];
    let mut offsets = vec![0u8; (entries.len() * width as usize).div_ceil(8) + 8];
    for (index, (key, offset)) in entries.iter().enumerate() {
        let bit = (key >> low_width) as usize + index;
        highs[bit / 64] |= 1 << (bit % 64);
        pack(&mut lows, index * low_width as usize, key & mask(low_width));
        pack(&mut offsets, index * width as usize, u64::from(*offset));
    }
    let mut samples = Vec::new();
    let mut zeros = 0usize;
    for bit in 0..highs.len() * 64 {
        if highs[bit / 64] >> (bit % 64) & 1 == 0 {
            if zeros % SAMPLE == 0 {
                samples.push(bit as u32);
            }
            zeros += 1;
        }
    }
    let mut out = Vec::new();
    for value in [entries.len(), width as usize, low_width as usize, highs.len(), samples.len()] {
        out.extend_from_slice(&(value as u32).to_le_bytes());
    }
    out.extend(lows);
    highs.iter().for_each(|word| out.extend_from_slice(&word.to_le_bytes()));
    samples.iter().for_each(|bit| out.extend_from_slice(&bit.to_le_bytes()));
    out.extend(offsets);
    out
}

fn mask(width: u32) -> u64 {
    if width == 0 { 0 } else { u64::MAX >> (64 - width) }
}

fn pack(buffer: &mut [u8], bit: usize, value: u64) {
    let word = u64::from_le_bytes(buffer[bit / 8..bit / 8 + 8].try_into().unwrap());
    buffer[bit / 8..bit / 8 + 8].copy_from_slice(&(word | (value << (bit % 8))).to_le_bytes());
}

fn unpack(buffer: &[u8], bit: usize, width: u32) -> u64 {
    let word = u64::from_le_bytes(buffer[bit / 8..bit / 8 + 8].try_into().unwrap());
    (word >> (bit % 8)) & mask(width)
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

pub struct Postings<'a> {
    width: u32,
    low_width: u32,
    lows: &'a [u8],
    highs: &'a [u8],
    samples: &'a [u8],
    offsets: &'a [u8],
}

impl<'a> Postings<'a> {
    pub fn read(bytes: &'a [u8]) -> (Postings<'a>, usize) {
        let word = |at: usize| u32::from_le_bytes(bytes[at..at + 4].try_into().unwrap()) as usize;
        let (len, width, low_width, high_words, samples) = (word(0), word(4), word(8), word(12), word(16));
        let mut at = 20;
        let mut take = |size: usize| {
            let slice = &bytes[at..at + size];
            at += size;
            slice
        };
        let lows = take((len * low_width).div_ceil(8) + 8);
        let highs = take(high_words * 8);
        let samples = take(samples * 4);
        let offsets = take((len * width).div_ceil(8) + 8);
        let postings = Postings { width: width as u32, low_width: low_width as u32, lows, highs, samples, offsets };
        (postings, at)
    }

    fn high(&self, word: usize) -> u64 {
        u64::from_le_bytes(self.highs[word * 8..word * 8 + 8].try_into().unwrap())
    }

    fn select_zero(&self, rank: usize) -> Option<usize> {
        let sample = rank / SAMPLE;
        if sample * 4 >= self.samples.len() {
            return None;
        }
        let mut bit = u32::from_le_bytes(self.samples[sample * 4..sample * 4 + 4].try_into().unwrap()) as usize;
        let mut left = rank % SAMPLE;
        loop {
            if bit / 64 >= self.highs.len() / 8 {
                return None;
            }
            let zeros = !self.high(bit / 64) >> (bit % 64);
            let available = (zeros.count_ones() as usize).min(64 - bit % 64);
            if left < available {
                let mut word = zeros;
                for _ in 0..left {
                    word &= word - 1;
                }
                return Some(bit + word.trailing_zeros() as usize);
            }
            left -= available;
            bit = (bit / 64 + 1) * 64;
        }
    }

    pub fn find(&self, key: u64, out: &mut Vec<u32>) {
        let bucket = (key >> self.low_width) as usize;
        let low = key & mask(self.low_width);
        let mut bit = match bucket {
            0 => 0,
            _ => match self.select_zero(bucket - 1) {
                Some(zero) => zero + 1,
                None => return,
            },
        };
        let mut index = bit - bucket;
        while bit / 64 < self.highs.len() / 8 && self.high(bit / 64) >> (bit % 64) & 1 == 1 {
            let candidate = unpack(self.lows, index * self.low_width as usize, self.low_width);
            if candidate == low {
                out.push(unpack(self.offsets, index * self.width as usize, self.width) as u32);
            } else if candidate > low {
                return;
            }
            bit += 1;
            index += 1;
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
        entries.push((1 << 40, 3));
        let bytes = encode(&entries, 10);
        let (postings, end) = Postings::read(&bytes);
        assert_eq!(end, bytes.len());
        for key in [0, 1000, 77_000, 166_000, 5, 1 << 40] {
            let mut found = Vec::new();
            postings.find(key, &mut found);
            let expected: Vec<u32> = entries.iter().filter(|e| e.0 == key).map(|e| e.1).collect();
            assert_eq!(found, expected, "key {key}");
        }
    }

    #[test]
    fn matches_a_linear_scan_on_random_keys() {
        let mut state = 0x9e37_79b9_7f4a_7c15u64;
        let mut next = || {
            state = state.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
            state >> 33
        };
        let mut entries: Vec<(u64, u32)> = (0..5000).map(|i| (next() % 40_000, i)).collect();
        entries.sort_unstable();
        let bytes = encode(&entries, 13);
        let (postings, _) = Postings::read(&bytes);
        for key in 0..40_100 {
            let mut found = Vec::new();
            postings.find(key, &mut found);
            let expected: Vec<u32> = entries.iter().filter(|e| e.0 == key).map(|e| e.1).collect();
            assert_eq!(found, expected, "key {key}");
        }
    }
}
