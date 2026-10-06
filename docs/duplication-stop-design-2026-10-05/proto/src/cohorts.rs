//! Exact current-content regions; no witness-pair enumeration or completeness caps.
use crate::storage::FileTokens;
use std::collections::{BTreeMap, BTreeSet, HashMap};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Region {
    pub text: Vec<Vec<u8>>,
    pub occurrences: Vec<(usize, usize)>,
}

pub fn regions(files: &[FileTokens], threshold: usize) -> Vec<Region> {
    assert!(threshold > 0, "threshold must be positive");
    let mut grouped = BTreeMap::<&[Option<Vec<u8>>], Vec<usize>>::new();
    for (file, stream) in files.iter().enumerate() {
        grouped.entry(&stream.tokens).or_default().push(file);
    }
    let classes: Vec<_> = grouped.into_iter().collect();
    let mut ids = BTreeMap::<&[u8], usize>::new();
    for (tokens, _) in &classes {
        for token in tokens.iter().flatten() {
            let next = ids.len() + 1;
            ids.entry(token).or_insert(next);
        }
    }
    let mut values = Vec::new();
    let mut locations = Vec::new();
    let mut predecessors = Vec::new();
    let mut output = BTreeMap::<Vec<Vec<u8>>, BTreeSet<(usize, usize)>>::new();
    let mut boundary = ids.len() + 1;
    for (class, (tokens, names)) in classes.iter().enumerate() {
        let mut first = 0;
        for end in 0..=tokens.len() {
            if end < tokens.len() && tokens[end].is_some() {
                continue;
            }
            if names.len() > 1 && end - first >= threshold {
                let text = tokens[first..end]
                    .iter()
                    .map(|v| v.as_ref().unwrap().clone())
                    .collect();
                output
                    .entry(text)
                    .or_default()
                    .extend(names.iter().map(|&file| (file, first)));
            }
            if end > first {
                for pos in first..end {
                    values.push(ids[tokens[pos].as_ref().unwrap().as_slice()]);
                    locations.push(Some((class, pos)));
                    predecessors.push(if pos == first {
                        boundary
                    } else {
                        ids[tokens[pos - 1].as_ref().unwrap().as_slice()]
                    });
                }
                values.push(boundary);
                locations.push(None);
                predecessors.push(boundary);
                boundary += 1;
            }
            first = end + 1;
        }
    }
    let n = values.len();
    let mut order: Vec<usize> = (0..n).collect();
    let mut ranks = values.clone();
    let mut width = 1;
    // ponytail: comparison-sort doubling costs O(n log² n); radix sorting is the
    // upgrade if measured index construction cannot meet the Stop budget.
    while width < n {
        let key = |i: usize| (ranks[i], ranks.get(i + width).map(|&r| r + 1).unwrap_or(0));
        order.sort_unstable_by_key(|&i| key(i));
        let mut updated = vec![0; n];
        for j in 1..n {
            updated[order[j]] =
                updated[order[j - 1]] + usize::from(key(order[j - 1]) != key(order[j]));
        }
        ranks = updated;
        if ranks[order[n - 1]] == n - 1 {
            break;
        }
        width = width.saturating_mul(2);
    }
    if n <= 1 {
        order.sort_unstable_by_key(|&i| values[i]);
    }
    let mut inverse = vec![0; n];
    for (rank, &pos) in order.iter().enumerate() {
        inverse[pos] = rank;
    }
    let mut lcp = vec![0; n];
    let mut height = 0;
    for pos in 0..n {
        let rank = inverse[pos];
        if rank == 0 {
            height = 0;
            continue;
        }
        let other = order[rank - 1];
        while pos + height < n
            && other + height < n
            && values[pos + height] == values[other + height]
        {
            height += 1;
        }
        lcp[rank] = height;
        height = height.saturating_sub(1);
    }
    let mut stack = Vec::<(usize, usize)>::new();
    let mut intervals = Vec::new();
    for right in 1..=n {
        let depth = lcp.get(right).copied().unwrap_or(0);
        let mut left = right - 1;
        while stack.last().is_some_and(|&(old, _)| old > depth) {
            let (old, start) = stack.pop().unwrap();
            left = start;
            if old >= threshold {
                intervals.push((old, left, right));
            }
        }
        if depth > 0 && stack.last().is_none_or(|&(old, _)| old < depth) {
            stack.push((depth, left));
        }
    }
    // ponytail: nested LCP intervals can revisit leaves quadratically; this is
    // uncapped and correct, but interval aggregation is needed for worst-case speed.
    for (depth, left, right) in intervals {
        let mut total = HashMap::<usize, usize>::new();
        for &pos in &order[left..right] {
            *total.entry(predecessors[pos]).or_default() += 1;
        }
        let mut child = left;
        for end in left + 1..=right {
            if end < right && lcp[end] > depth {
                continue;
            }
            let mut local = HashMap::<usize, usize>::new();
            for &pos in &order[child..end] {
                *local.entry(predecessors[pos]).or_default() += 1;
            }
            for &pos in &order[child..end] {
                let pred = predecessors[pos];
                if (right - left) + local[&pred] > (end - child) + total[&pred] {
                    let (class, start) = locations[pos].unwrap();
                    let (tokens, names) = &classes[class];
                    let text = tokens[start..start + depth]
                        .iter()
                        .map(|v| v.as_ref().unwrap().clone())
                        .collect();
                    output
                        .entry(text)
                        .or_default()
                        .extend(names.iter().map(|&file| (file, start)));
                }
            }
            child = end;
        }
    }
    output
        .into_iter()
        .map(|(text, occurrences)| Region {
            text,
            occurrences: occurrences.into_iter().collect(),
        })
        .collect()
}
