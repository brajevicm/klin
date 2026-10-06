"""CLI-only exact Rust cohort comparison with independent diagonal oracle."""
import json
from pathlib import Path
import subprocess
import sys
import tempfile
ROOT = Path(__file__).resolve().parent
sys.path.insert(0, str(ROOT.parent / 'duplication-speed-2026-10-05'))
import region_cohorts
import families

DRIVER = r'''
mod storage { #[allow(dead_code)] pub struct FileTokens { pub path: String, pub tokens: Vec<Option<Vec<u8>>>, pub rows: Vec<u32>, pub unsafe_units: usize, pub error: bool } }
#[path = "MODULE"] mod cohorts;
use std::io::{self, Read};
fn hex(bytes: &[u8]) -> String { bytes.iter().map(|v| format!("{v:02x}")).collect() }
fn main() {
 let mut data = String::new(); io::stdin().read_to_string(&mut data).unwrap();
 let mut lines = data.lines(); let t: usize = lines.next().unwrap().parse().unwrap();
 let files: Vec<_> = lines.map(|line| storage::FileTokens { path: String::new(), rows: Vec::new(), unsafe_units: 0, error: false, tokens: if line.is_empty() { Vec::new() } else { line.split(',').map(|v| if v == "_" { None } else { Some((0..v.len()).step_by(2).map(|i| u8::from_str_radix(&v[i..i+2],16).unwrap()).collect()) }).collect() } }).collect();
 for region in cohorts::regions(&files,t) { println!("{}|{}", region.text.iter().map(|v| hex(v)).collect::<Vec<_>>().join(","), region.occurrences.iter().map(|(f,s)|format!("{f}:{s}")).collect::<Vec<_>>().join(",")); }
}
'''

def encoded(token):
    return json.dumps(token, separators=(',', ':'), ensure_ascii=False).encode()

def main():
    with tempfile.TemporaryDirectory(prefix='klin-cohort-cli-') as temp:
        temp = Path(temp)
        source = temp / 'main.rs'
        source.write_text(DRIVER.replace('MODULE', str(ROOT / 'proto/src/cohorts.rs')))
        binary = temp / 'cohorts'
        subprocess.run(['rustc', '--edition=2024', '-O', str(source), '-o', str(binary)], check=True)
        rows = []
        for label, records, threshold in region_cohorts.cases():
            lines = [str(threshold)]
            for _, stream in records:
                lines.append(','.join(encoded(token).hex() if safe else '_' for token, safe in zip(stream['tokens'], stream['safe'])))
            data = subprocess.check_output([str(binary)], input=('\n'.join(lines) + '\n').encode()).decode()
            actual = {}
            for line in data.splitlines():
                text, occurrences = line.split('|')
                key = tuple(json.loads(bytes.fromhex(token)) for token in text.split(','))
                actual[key] = {(records[int(file)][0], int(start), int(start) + len(key)) for file, start in (occ.split(':') for occ in occurrences.split(','))}
            expected = ({tuple(range(30)): {(str(i), 0, 30) for i in range(1000)}} if label == '1000-identical' else families.exhaustive(records, threshold))
            assert actual == expected, (label, actual, expected)
            rows.append({'case': label, 'families': len(actual), 'occurrences': sum(map(len, actual.values()))})
        print(json.dumps({'oracle_equal': True, 'cases': len(rows), 'rows': rows}, indent=2))

if __name__ == '__main__':
    main()
