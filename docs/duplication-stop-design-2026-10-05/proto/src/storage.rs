//! Lossless experiment: exact dictionary, greedy token backreferences, row runs.
use std::collections::{BTreeSet, HashMap};
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTokens {
    pub path: String,
    pub tokens: Vec<Option<Vec<u8>>>,
    pub rows: Vec<u32>,
    pub unsafe_units: usize,
    pub error: bool,
}
fn put(out: &mut Vec<u8>, mut n: usize) {
    while n >= 128 {
        out.push((n as u8 & 127) | 128);
        n >>= 7;
    }
    out.push(n as u8);
}
fn bytes(out: &mut Vec<u8>, b: &[u8]) {
    put(out, b.len());
    out.extend_from_slice(b);
}
fn checksum(b: &[u8]) -> u64 {
    b.iter().fold(0xcbf29ce484222325, |h, b| {
        (h ^ u64::from(*b)).wrapping_mul(0x100000001b3)
    })
}
#[derive(Default, Debug)]
pub struct Breakdown {
    pub dictionary: usize,
    pub chains: usize,
    pub rows: usize,
    pub metadata: usize,
    pub container: usize,
}
pub fn pack(files: &[FileTokens]) -> Vec<u8> {
    pack_with_breakdown(files).0
}
pub fn pack_with_breakdown(files: &[FileTokens]) -> (Vec<u8>, Breakdown) {
    let mut breakdown = Breakdown {
        container: 12,
        ..Default::default()
    };
    let dictionary: Vec<&[u8]> = files
        .iter()
        .flat_map(|f| &f.tokens)
        .filter_map(|t| t.as_deref())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let ids: HashMap<&[u8], usize> = dictionary
        .iter()
        .enumerate()
        .map(|(i, t)| (*t, i + 1))
        .collect();
    let mut out = b"KDS1".to_vec();
    put(&mut out, dictionary.len());
    let mut previous: &[u8] = &[];
    for token in dictionary {
        let prefix = previous
            .iter()
            .zip(token)
            .take_while(|(a, b)| a == b)
            .count();
        put(&mut out, prefix);
        bytes(&mut out, &token[prefix..]);
        previous = token;
    }
    breakdown.dictionary = out.len() - 4;
    let before = out.len();
    put(&mut out, files.len());
    breakdown.metadata += out.len() - before;
    let mut chain = Vec::new();
    let mut triples = HashMap::new();
    for f in files {
        assert_eq!(f.tokens.len(), f.rows.len());
        let before = out.len();
        bytes(&mut out, f.path.as_bytes());
        put(&mut out, f.unsafe_units);
        out.push(u8::from(f.error));
        put(&mut out, f.tokens.len());
        breakdown.metadata += out.len() - before;
        let before = out.len();
        let start = chain.len();
        chain.extend(f.tokens.iter().map(|t| t.as_deref().map_or(0, |t| ids[t])));
        let end = chain.len();
        let mut at = start;
        while at < end {
            let key = (at + 2 < end).then(|| (chain[at], chain[at + 1], chain[at + 2]));
            let mut length = 0;
            let mut distance = 0;
            if let Some(old) = key.and_then(|k| triples.get(&k).copied()) {
                distance = at - old;
                while at + length < end && chain[old + length] == chain[at + length] {
                    length += 1;
                }
            }
            if length >= 4 {
                put(&mut out, 1);
                put(&mut out, length);
                put(&mut out, distance);
            } else {
                length = 1;
                put(&mut out, chain[at] * 2);
            }
            // ponytail: one candidate per triple; use a bounded chain only if compression misses budget.
            for p in at..at + length {
                if p + 2 < end {
                    triples.insert((chain[p], chain[p + 1], chain[p + 2]), p);
                }
            }
            at += length;
        }
        breakdown.chains += out.len() - before;
        let before = out.len();
        let mut at = 0;
        let mut previous = 0;
        while at < f.rows.len() {
            assert!(f.rows[at] >= previous);
            let delta = f.rows[at] - previous;
            let mut n = 1;
            while at + n < f.rows.len() && f.rows[at + n] - f.rows[at + n - 1] == delta {
                n += 1;
            }
            put(&mut out, delta as usize);
            put(&mut out, n);
            previous = f.rows[at + n - 1];
            at += n;
        }
        breakdown.rows += out.len() - before;
    }
    let sum = checksum(&out);
    out.extend_from_slice(&sum.to_le_bytes());
    assert_eq!(
        out.len(),
        breakdown.dictionary
            + breakdown.chains
            + breakdown.rows
            + breakdown.metadata
            + breakdown.container
    );
    (out, breakdown)
}
struct Reader<'a> {
    b: &'a [u8],
    at: usize,
}
impl Reader<'_> {
    fn n(&mut self) -> Result<usize, String> {
        let mut n = 0usize;
        for shift in (0..usize::BITS).step_by(7) {
            let b = *self.b.get(self.at).ok_or("truncated integer")?;
            self.at += 1;
            let value = usize::from(b & 127);
            if value > usize::MAX >> shift {
                return Err("integer overflow".into());
            }
            n |= value << shift;
            if b < 128 {
                return Ok(n);
            }
        }
        Err("integer overflow".into())
    }
    fn bytes(&mut self) -> Result<&[u8], String> {
        let n = self.n()?;
        let end = self.at.checked_add(n).ok_or("length overflow")?;
        let b = self.b.get(self.at..end).ok_or("truncated bytes")?;
        self.at = end;
        Ok(b)
    }
}
pub fn unpack(bytes: &[u8]) -> Result<Vec<FileTokens>, String> {
    if bytes.len() < 12 || &bytes[..4] != b"KDS1" {
        return Err("bad header".into());
    }
    let end = bytes.len() - 8;
    if checksum(&bytes[..end]) != u64::from_le_bytes(bytes[end..].try_into().unwrap()) {
        return Err("checksum mismatch".into());
    }
    let mut r = Reader {
        b: &bytes[..end],
        at: 4,
    };
    let count = r.n()?;
    if count > r.b.len() {
        return Err("dictionary count".into());
    }
    let mut dictionary: Vec<Vec<u8>> = Vec::new();
    for _ in 0..count {
        let prefix = r.n()?;
        let previous = dictionary.last().map_or(&[][..], Vec::as_slice);
        let mut token = previous.get(..prefix).ok_or("bad prefix")?.to_vec();
        token.extend_from_slice(r.bytes()?);
        if dictionary.last().is_some_and(|p| p >= &token) {
            return Err("unordered dictionary".into());
        }
        dictionary.push(token);
    }
    let count = r.n()?;
    if count > r.b.len() {
        return Err("file count".into());
    }
    let mut files = Vec::new();
    let mut chain = Vec::new();
    for _ in 0..count {
        let path = String::from_utf8(r.bytes()?.to_vec()).map_err(|_| "invalid path")?;
        let unsafe_units = r.n()?;
        let error = match r.b.get(r.at) {
            Some(0) => false,
            Some(1) => true,
            _ => return Err("bad error flag".into()),
        };
        r.at += 1;
        let n = r.n()?;
        // Every run expands to at most this explicit per-file ceiling. Callers must
        // separately impose an application cache-size / decoded-size limit.
        if n > 100_000_000 {
            return Err("token count limit".into());
        }
        let start = chain.len();
        while chain.len() - start < n {
            let code = r.n()?;
            if code == 1 {
                let length = r.n()?;
                let distance = r.n()?;
                if length < 4
                    || length > n - (chain.len() - start)
                    || distance == 0
                    || distance > chain.len()
                {
                    return Err("bad backreference".into());
                }
                for _ in 0..length {
                    let value = chain[chain.len() - distance];
                    chain.push(value);
                }
            } else {
                if code % 2 != 0 || code / 2 > dictionary.len() {
                    return Err("bad token ID".into());
                }
                chain.push(code / 2);
            }
        }
        let mut rows = Vec::new();
        let mut previous = 0u32;
        while rows.len() < n {
            let delta = u32::try_from(r.n()?).map_err(|_| "row delta overflow")?;
            let run = r.n()?;
            if run == 0 || run > n - rows.len() {
                return Err("bad row run".into());
            }
            for _ in 0..run {
                previous = previous.checked_add(delta).ok_or("row overflow")?;
                rows.push(previous);
            }
        }
        let tokens = chain[start..]
            .iter()
            .map(|id| {
                if *id == 0 {
                    None
                } else {
                    Some(dictionary[*id - 1].clone())
                }
            })
            .collect();
        files.push(FileTokens {
            path,
            tokens,
            rows,
            unsafe_units,
            error,
        });
    }
    if r.at != r.b.len() {
        return Err("trailing data".into());
    }
    Ok(files)
}
