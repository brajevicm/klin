// Copied verbatim from tests/performance.rs at c1805539; the digests in the profiles check the copy.
#![allow(dead_code)]
use std::path::PathBuf;

pub struct Tree(pub PathBuf);

impl Tree {
    pub fn write(&self, path: &str, source: &str) {
        let target = self.0.join(path);
        std::fs::create_dir_all(target.parent().unwrap()).unwrap();
        std::fs::write(target, source).unwrap();
    }
}

pub const DECLARATIONS_PER_KLOC: std::ops::RangeInclusive<usize> = 270..=290;

#[derive(Clone, Copy)]
pub struct Profile {
    pub name: &'static str,
    pub units: Option<fn(usize) -> usize>,
    pub expected: Option<Generated>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Generated {
    pub loc: usize,
    pub declarations: usize,
    pub digest: u64,
}

pub const BASE: Profile = Profile {
    name: "file-count",
    units: None,
    expected: None,
};
pub const DENSE_300K: Profile = Profile {
    name: "source-dense-300k",
    units: Some(one_unit),
    expected: Some(Generated {
        loc: 325_077,
        declarations: 90_007,
        digest: 17_738_620_850_890_864_555,
    }),
};
pub const DENSE_1M: Profile = Profile {
    name: "source-dense-1m",
    units: Some(three_or_four_units),
    expected: Some(Generated {
        loc: 1_033_827,
        declarations: 292_507,
        digest: 5_045_938_053_977_738_811,
    }),
};

pub fn one_unit(_: usize) -> usize {
    1
}

pub fn three_or_four_units(index: usize) -> usize {
    3 + usize::from(index.is_multiple_of(4))
}
pub fn write_sources(
    tree: &Tree,
    files_per_language: usize,
    tsx: usize,
    profile: Profile,
) -> Generated {
    let mut generated = Generated {
        loc: 0,
        declarations: 0,
        digest: 0xcbf2_9ce4_8422_2325,
    };
    for index in 0..files_per_language {
        let source = rust_source_for(index, files_per_language, profile);
        record(tree, &mut generated, &rust_path(index), &source);
    }
    for index in 0..files_per_language {
        let source = typescript_source_for(index, tsx, files_per_language, profile);
        record(tree, &mut generated, &typescript_path(index, tsx), &source);
    }
    generated
}

pub fn record(tree: &Tree, generated: &mut Generated, path: &str, source: &str) {
    generated.loc += source.bytes().filter(|byte| *byte == b'\n').count();
    generated.declarations += source.lines().filter(|line| declares(line)).count();
    for byte in path.bytes().chain([0]).chain(source.bytes()).chain([0]) {
        generated.digest = (generated.digest ^ u64::from(byte)).wrapping_mul(0x0100_0000_01b3);
    }
    tree.write(path, source);
}

pub fn declares(line: &str) -> bool {
    [
        "const ",
        "struct ",
        "type ",
        "impl ",
        "fn ",
        "pub fn ",
        "    fn ",
        "interface ",
        "class ",
        "    adjust(",
        "function ",
        "export function ",
    ]
    .iter()
    .any(|start| line.starts_with(start))
}

pub fn rust_path(index: usize) -> String {
    match index {
        0 => "rust/src/lib.rs".to_string(),
        1 => "rust/src/held_escape.rs".to_string(),
        2 => "rust/src/held_stub.rs".to_string(),
        3 => "rust/tests/fixture_test.rs".to_string(),
        _ => format!("rust/src/module_{index:04}.rs"),
    }
}

pub fn typescript_path(index: usize, tsx: usize) -> String {
    match index {
        0 => "web/src/index.ts".to_string(),
        1 => "web/src/held_escape.ts".to_string(),
        2 => "web/src/held_stub.ts".to_string(),
        3 => "web/src/fixture.test.ts".to_string(),
        index if index < tsx + 4 => {
            format!("web/src/components/component_{:04}.tsx", index - 4)
        }
        _ => format!("web/src/module_{index:04}.ts"),
    }
}

pub fn rust_source(index: usize, total: usize) -> String {
    match index {
        0 => {
            "pub mod held_escape;\npub mod held_stub;\npub mod module_0004;\npub mod module_0005;\n"
                .to_string()
        }
        1 => {
            "pub fn held_escape(input: usize) -> usize {\n    Some(input).unwrap()\n}\n".to_string()
        }
        2 => "pub fn held_stub() -> usize {\n    todo!()\n}\n".to_string(),
        3 => "#[test]\nfn test_fixture_contract() {\n    assert_eq!(2 + 2, 4);\n}\n".to_string(),
        _ if index.is_multiple_of(10) => rust_complex(index, total),
        _ => rust_simple(index, total),
    }
}

pub fn rust_source_for(index: usize, total: usize, profile: Profile) -> String {
    match profile.units {
        Some(units) => dense_rust_source(index, total, units(index)),
        None => rust_source(index, total),
    }
}

pub fn dense_rust_source(index: usize, total: usize, units: usize) -> String {
    let body = dense_rust_body(index, total, units);
    match index {
        0 => format!(
            "pub mod held_escape;\npub mod held_stub;\npub mod module_0004;\npub mod module_0005;\nfn main() {{ value_0000(1); }}\n{body}"
        ),
        1 => format!(
            "pub fn held_escape(input: usize) -> usize {{\n    Some(input).{}\n}}\n{body}",
            "unwrap()"
        ),
        2 => format!("pub fn held_stub() -> usize {{\n    todo!()\n}}\n{body}"),
        3 => {
            format!("#[test]\nfn test_fixture_contract() {{\n    assert_eq!(2 + 2, 4);\n}}\n{body}")
        }
        _ => body,
    }
}

pub fn dense_rust_body(index: usize, total: usize, units: usize) -> String {
    let dependency = if index == 0 || index == 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let mut source = format!("use crate::module_{dependency:04}::value_{dependency:04};\n");
    for unit in 0..units {
        let id = unit_id(index, unit);
        let calls = unit_calls(index, unit, units, dependency);
        source.push_str(&format!(
            "const SLOT_{id}: usize = {index};\nconst PHASE_{id}: &str = \"base\";\nstruct Record_{id} {{\n    value: usize,\n}}\ntype Alias_{id} = Record_{id};\nfn shared_{bucket:02}_{unit}(input: usize) -> usize {{\n    input + SLOT_{id} + PHASE_{id}.len()\n}}\nimpl Record_{id} {{\n    fn adjust(&self) -> usize {{\n        self.value + SLOT_{id}\n    }}\n}}\npub fn value_{id}(input: usize) -> usize {{\n    let record: Alias_{id} = Record_{id} {{ value: input }};\n    shared_{bucket:02}_{unit}(record.adjust()) + branch_{id}(input){calls}\n}}\nfn branch_{id}(mut value: usize) -> usize {{\n    if value % 2 == 0 {{\n        value += SLOT_{id};\n    }} else {{\n        value += 1;\n    }}\n    match value % 3 {{\n        0 => value,\n        1 => value + 1,\n        _ => value + 2,\n    }}\n}}\n"
        ));
    }
    source
}

pub fn unit_id(index: usize, unit: usize) -> String {
    match unit {
        0 => format!("{index:04}"),
        _ => format!("{index:04}_{unit}"),
    }
}

pub fn unit_calls(index: usize, unit: usize, units: usize, dependency: usize) -> String {
    let mut calls = String::new();
    if unit == 0 {
        calls.push_str(&format!(" + value_{dependency:04}(input)"));
    }
    if unit + 1 < units {
        calls.push_str(&format!(" + value_{}(input)", unit_id(index, unit + 1)));
    }
    calls
}

pub fn rust_simple(index: usize, total: usize) -> String {
    let dependency = if index == 4 { total - 1 } else { index - 1 };
    format!(
        "use crate::module_{dependency:04}::value_{dependency};\n\npub fn value_{index}(input: usize) -> usize {{\n    input + value_{dependency}(input) + {index}\n}}\n// base\n"
    )
}

pub fn rust_complex(index: usize, total: usize) -> String {
    let dependency = if index == 4 { total - 1 } else { index - 1 };
    format!(
        "use crate::module_{dependency:04}::value_{dependency};\n\npub fn value_{index}(input: usize) -> usize {{\n    input + value_{dependency}(input) + {index}\n}}\n\npub fn branch_{index}(mut value: usize) -> usize {{\n    if value % 2 == 0 {{\n        value += 1;\n    }} else {{\n        value += 2;\n    }}\n    for step in 0..3 {{\n        if step == 1 {{\n            value += step;\n        }}\n    }}\n    if value % 5 == 0 {{\n        value += 5;\n    }}\n    if value % 7 == 0 {{\n        value += 7;\n    }}\n    match value % 3 {{\n        0 => value,\n        1 => value + 1,\n        _ => value + 2,\n    }}\n}}\n// base\n"
    )
}

pub fn typescript_source_for(index: usize, tsx: usize, total: usize, profile: Profile) -> String {
    match profile.units {
        Some(units) => dense_typescript_source(index, tsx, total, units(index)),
        None => typescript_source(index, tsx),
    }
}

pub fn dense_typescript_source(index: usize, tsx: usize, total: usize, units: usize) -> String {
    let body = dense_typescript_body(index, tsx, total, units);
    let first = tsx + 4;
    let component = if (4..first).contains(&index) {
        format!(
            "export const Component_{:04} = (input: number) => <span>{{input}}</span>;\n",
            index - 4
        )
    } else {
        String::new()
    };
    match index {
        0 => format!("export {{ value_{first:04} }} from \"./module_{first:04}\";\n{body}"),
        1 => format!(
            "export function heldEscape(input: unknown): unknown {{\n    return input as any;\n}}\n{body}"
        ),
        2 => format!(
            "export function heldStub(): never {{\n    throw new Error(\"not implemented\");\n}}\n{body}"
        ),
        3 => format!(
            "import {{ value_{first:04} }} from \"./module_{first:04}\";\nexport function test_fixture(): number {{\n    return value_{first:04}(1);\n}}\nit(\"keeps the fixture\", () => value_{first:04}(1));\n{body}"
        ),
        _ => component + &body,
    }
}

pub fn dense_typescript_body(index: usize, tsx: usize, total: usize, units: usize) -> String {
    let dependency = if index == 0 || index == tsx + 4 {
        total - 1
    } else {
        index - 1
    };
    let bucket = index % 256;
    let mut source =
        format!("import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n");
    for unit in 0..units {
        let id = unit_id(index, unit);
        let calls = unit_calls(index, unit, units, dependency);
        source.push_str(&format!(
            "const SLOT_{id}: number = {index};\nconst PHASE_{id} = \"base\";\ninterface Record_{id} {{\n    value: number;\n}}\ntype Alias_{id} = Record_{id};\nclass Holder_{id} {{\n    constructor(private value: number) {{}}\n    adjust(): number {{\n        return this.value + SLOT_{id};\n    }}\n}}\nfunction shared_{bucket:02}_{unit}(input: number): number {{\n    return input + SLOT_{id} + PHASE_{id}.length;\n}}\nexport function value_{id}(input: number): number {{\n    const record: Alias_{id} = {{ value: input }};\n    const holder = new Holder_{id}(record.value);\n    return shared_{bucket:02}_{unit}(holder.adjust()) + branch_{id}(input){calls};\n}}\nfunction branch_{id}(value: number): number {{\n    let result = value;\n    if (result % 2 === 0) {{\n        result += SLOT_{id};\n    }} else {{\n        result += 1;\n    }}\n    switch (result % 3) {{\n        case 0: return result;\n        case 1: return result + 1;\n        default: return result + 2;\n    }}\n}}\n"
        ));
    }
    source
}

pub fn typescript_source(index: usize, tsx: usize) -> String {
    match index {
        0 => format!(
            "export {{ value_{first:04} }} from \"./module_{first:04}\";\n",
            first = tsx + 4
        ),
        1 => "export function heldEscape(input: unknown): unknown {\n    return input as any;\n}\n"
            .to_string(),
        2 => "export function heldStub(): never {\n    throw new Error(\"not implemented\");\n}\n"
            .to_string(),
        3 => format!(
            "import {{ value_{first:04} }} from \"./module_{first:04}\";\nexport function test_fixture(): number {{\n    return value_{first}(1);\n}}\nit(\"keeps the fixture\", () => value_{first}(1));\n",
            first = tsx + 4
        ),
        index if index < tsx + 4 => format!(
            "export const Component_{index:04} = (input: number) => <span>{{input}}</span>;\n// base\n"
        ),
        _ if index.is_multiple_of(10) => typescript_complex(index, tsx),
        _ => typescript_simple(index, tsx),
    }
}

pub fn typescript_simple(index: usize, tsx: usize) -> String {
    let first = tsx + 4;
    if index == first {
        format!(
            "export function value_{index}(input: number): number {{\n    return input + {index};\n}}\n// base\n"
        )
    } else {
        let dependency = index - 1;
        format!(
            "import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n\nexport function value_{index}(input: number): number {{\n    return input + value_{dependency}(input) + {index};\n}}\n// base\n"
        )
    }
}

pub fn typescript_complex(index: usize, tsx: usize) -> String {
    let first = tsx + 4;
    let import_line = if index == first {
        String::new()
    } else {
        format!(
            "import {{ value_{dependency:04} }} from \"./module_{dependency:04}\";\n\n",
            dependency = index - 1
        )
    };
    let value = if index == first {
        format!("    return input + {index};")
    } else {
        format!("    return input + value_{}(input) + {index};", index - 1)
    };
    format!(
        "{import_line}export function value_{index}(input: number): number {{\n{value}\n}}\n\nexport function branch_{index}(value: number): number {{\n    let result = value;\n    if (result % 2 === 0) {{\n        result += 1;\n    }} else {{\n        result += 2;\n    }}\n    for (const step of [0, 1, 2]) {{\n        if (step === 1) {{\n            result += step;\n        }}\n    }}\n    if (result % 5 === 0) {{\n        result += 5;\n    }}\n    if (result % 7 === 0) {{\n        result += 7;\n    }}\n    switch (result % 3) {{\n        case 0: return result;\n        case 1: return result + 1;\n        default: return result + 2;\n    }}\n}}\n// base\n"
    )
}
