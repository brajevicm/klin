mod harness;

use harness::Tree;

const RUST: &str = r##"fn tangled(a: i32) -> i32 {
    if a > 0 && a < 10 {
        for x in 0..a {
            if x == 3 { return 1; }
        }
    } else if a == 0 || a == -1 {
        return 2;
    }
    match a {
        1 => 1,
        9 => 0,
    }
}

fn simple() -> i32 { 1 }

fn quoted() -> &'static str {
    r#"if a && b || c { match m { 1 => 1, _ => 0 } } x?"#
}
"##;

const PYTHON: &str = r#"def tangled(a, b):
    if a and b:
        for x in b:
            if x:
                pass
    elif a or b:
        pass
    try:
        c = [y for y in b if y]
    except ValueError:
        c = None
    return 1 if a else c


def simple(a):
    return a
"#;

const TYPESCRIPT: &str = r#"function tangled(a: number, b: string[]): number {
  if (a > 0 && a < 10) {
    for (const x of b) {
      if (x) { return 1; }
    }
  } else if (a === 0 || a === -1) {
    switch (a) {
      case 1: return 2;
      default: break;
    }
  }
  try {
    while (a) { a -= 1; }
  } catch (e) {
    return a ? 3 : 4;
  }
  return b.filter((x) => x.length > 0).length;
}

function simple(a: number): number { return a; }
"#;

const GO: &str = r#"package main

func tangled(a int, b []string) int {
	if a > 0 && a < 10 {
		for _, x := range b {
			if x != "" && len(x) > 1 {
				return 1
			}
		}
	} else if a == 0 || a == -1 {
		switch a {
		case 1:
			return 2
		default:
			return 3
		}
	}
	return len(b)
}

func simple(a int) int { return a }
"#;

const JAVA: &str = r#"class Knot {
    int tangled(int a, String[] b) {
        if (a > 0 && a < 10) {
            for (String x : b) {
                if (!x.isEmpty()) { return 1; }
            }
        } else if (a == 0 || a == -1) {
            switch (a) {
                case 1: return 2;
                default: break;
            }
        }
        try {
            while (a > 0) { a -= 1; }
        } catch (RuntimeException e) {
            return a > 0 ? 3 : 4;
        }
        return b.length;
    }

    int simple(int a) { return a; }
}
"#;

const RUBY: &str = r#"def tangled(a, b)
  if a && b
    b.each do |x|
      return 1 if x
    end
  elsif a || b
    case a
    when 1 then return 2
    else return 3
    end
  end
  begin
    while a
      a -= 1
    end
  rescue StandardError
    return a ? 4 : 5
  end
  b.length
end

def simple(a)
  a
end
"#;

const SWIFT: &str = r#"func tangled(_ a: Int, _ b: [String]) -> Int {
    if a > 0 && a < 10 {
        for x in b {
            if !x.isEmpty { return 1 }
        }
    } else if a == 0 || a == -1 {
        switch a {
        case 1: return 2
        default: break
        }
    }
    guard a > 0 else { return 3 }
    do {
        while a > 0 { break }
    } catch {
        return a > 0 ? 4 : 5
    }
    return b.count
}

func simple(_ a: Int) -> Int { return a }
"#;

const KOTLIN: &str = r#"fun tangled(a: Int, b: List<String>): Int {
    if (a > 0 && a < 10) {
        for (x in b) {
            if (x.isNotEmpty()) return 1
        }
    } else if (a == 0 || a == -1) {
        when (a) {
            1 -> return 2
            else -> return 3
        }
    }
    try {
        while (a > 0) break
    } catch (e: Exception) {
        return if (a > 0) 4 else 5
    }
    return b.size
}

fun simple(a: Int): Int = a
"#;

fn config(ceilings: &str) -> String {
    format!(
        r#"{{ "project": "t", "complexity": {{ "sources": ["src"], "ceilings": {ceilings},
             "baseline": "detent/complexity-baseline.json" }} }}"#
    )
}

fn tree(ceilings: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", &config(ceilings));
    tree
}

fn baseline(entries: &str) -> String {
    format!(r#"{{ "entries": [{entries}] }}"#)
}

fn entry(file: &str, text: &str, line: u64, cc: u64, lines: u64) -> String {
    format!(
        r#"{{"file": {file:?}, "text": {text:?}, "line": {line}, "cc": {cc}, "lines": {lines}}}"#
    )
}

#[test]
fn rust_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/knot.rs:1  cc 9, 13 lines  fn tangled(a: i32) -> i32 {"),
        "{}",
        run.out
    );
    assert!(!run.says("knot.rs:15"), "{}", run.out);
    assert!(!run.says("knot.rs:17"), "{}", run.out);
}

#[test]
fn python_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.py", PYTHON);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/knot.py:1  cc 11, 12 lines  def tangled(a, b):"),
        "{}",
        run.out
    );
    assert!(!run.says("knot.py:15"), "{}", run.out);
}

#[test]
fn a_rust_raw_string_is_text_not_code_so_it_adds_no_branches() {
    let tree = tree(r#"{"cc": 0, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert!(
        run.says("src/knot.rs:17  cc 1, 3 lines  fn quoted() -> &'static str {"),
        "{}",
        run.out
    );
}

#[test]
fn the_measurement_runs_in_process_so_an_empty_path_changes_nothing() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write("src/knot.py", PYTHON);

    let bare = tree.run_without_path(&["complexity"]);
    assert_eq!(bare.code, 1, "{}", bare.out);
    assert!(bare.says("src/knot.rs:1  cc 9"), "{}", bare.out);
    assert!(bare.says("src/knot.py:1  cc 11"), "{}", bare.out);
}

#[test]
fn a_function_over_the_length_ceiling_alone_fails() {
    let tree = tree(r#"{"cc": 8, "lines": 5}"#);
    tree.write(
        "src/long.rs",
        &format!("fn stretched() {{\n{}}}\n", "    let a = 1;\n".repeat(6)),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/long.rs:1  cc 1, 8 lines"), "{}", run.out);
}

#[test]
fn write_baseline_accepts_what_is_over_the_gate_and_the_rerun_holds() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write("src/knot.py", PYTHON);

    let written = tree.run(&["complexity", "--write-baseline"]);
    assert_eq!(written.code, 0, "{}", written.out);
    assert!(written.says("2 function(s) over the"), "{}", written.out);

    let stored =
        std::fs::read_to_string(tree.path("detent/complexity-baseline.json")).expect("read");
    assert!(stored.contains("fn tangled(a: i32) -> i32 {"), "{stored}");
    assert!(stored.contains("\"cc\": 9"), "{stored}");
    assert!(stored.contains("\"lines\": 13"), "{stored}");

    let rerun = tree.run(&["complexity", "--strict"]);
    assert_eq!(rerun.code, 0, "{}", rerun.out);
    assert!(
        rerun.says("OK: 5 function(s) judged, 2 over the gate"),
        "{}",
        rerun.out
    );
}

#[test]
fn a_baselined_function_that_moved_down_the_file_still_matches() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", &format!("// a header\n// and more\n{RUST}"));
    tree.write(
        "detent/complexity-baseline.json",
        &baseline(&entry(
            "src/knot.rs",
            "fn tangled(a: i32) -> i32 {",
            1,
            9,
            13,
        )),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_baselined_function_whose_cyclomatic_grew_fails() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write(
        "detent/complexity-baseline.json",
        &baseline(&entry(
            "src/knot.rs",
            "fn tangled(a: i32) -> i32 {",
            1,
            8,
            13,
        )),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(
        run.says("cc 9, 13 lines, was cc 8, 13 lines"),
        "{}",
        run.out
    );
    assert!(!run.says("--write-baseline"), "{}", run.out);
}

#[test]
fn a_baselined_function_whose_length_grew_fails_too() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write(
        "detent/complexity-baseline.json",
        &baseline(&entry(
            "src/knot.rs",
            "fn tangled(a: i32) -> i32 {",
            1,
            9,
            12,
        )),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("cc 9, 13 lines, was cc 9, 12 lines"),
        "{}",
        run.out
    );
}

#[test]
fn a_function_that_improved_is_a_note_locally_and_a_failure_under_strict() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write(
        "detent/complexity-baseline.json",
        &baseline(&entry(
            "src/knot.rs",
            "fn tangled(a: i32) -> i32 {",
            1,
            12,
            13,
        )),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("cc 9, 13 lines, baseline says cc 12, 13 lines"),
        "{}",
        run.out
    );
    assert!(
        run.says("detent complexity --write-baseline"),
        "{}",
        run.out
    );

    let strict = tree.run(&["complexity", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
}

#[test]
fn a_missing_key_is_a_tool_error_naming_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "ceilings": {"cc": 8, "lines": 60},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"sources\""), "{}", run.out);

    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"],
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    let missing_ceilings = tree.run(&["complexity"]);
    assert_eq!(missing_ceilings.code, 2, "{}", missing_ceilings.out);
    assert!(
        missing_ceilings.says("\"ceilings\""),
        "{}",
        missing_ceilings.out
    );
}

#[test]
fn a_source_the_reader_cannot_open_is_a_tool_error_not_a_gate_failure() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("could not be read"), "{}", run.out);
}

#[test]
fn a_file_the_grammar_cannot_parse_is_named_while_the_rest_of_the_tree_is_still_judged() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/bad.rs",
        "%%% not rust %%%\nfn hidden() { if true {} }\n",
    );
    tree.write("src/good.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("could not parse"), "{}", run.out);
    assert!(run.says("src/bad.rs"), "{}", run.out);
    assert!(run.says("the Rust grammar rejected it"), "{}", run.out);
    assert!(run.says("src/good.rs:1"), "{}", run.out);
    assert!(!run.says("src/bad.rs:2"), "{}", run.out);
}

#[test]
fn a_baseline_entry_for_an_unparseable_file_is_neither_stale_nor_lost() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/good.rs", "fn simple() -> i32 { 1 }\n");
    tree.write("src/later.rs", "fn held() -> i32 { 2 }\n");
    assert_eq!(tree.run(&["complexity", "--write-baseline"]).code, 0);
    tree.write("src/later.rs", "%%% not rust %%%\n");

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("the Rust grammar rejected it"), "{}", run.out);
    assert!(!run.says("matched nothing this run"), "{}", run.out);
    assert!(!run.says("looser"), "{}", run.out);
}

#[test]
fn writing_a_baseline_is_refused_while_a_file_goes_unparsed() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/good.rs", "fn simple() -> i32 { 1 }\n");
    tree.write("src/bad.rs", "%%% not rust %%%\n");

    let run = tree.run(&["complexity", "--write-baseline"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("src/bad.rs"), "{}", run.out);
    assert!(
        !tree.path("detent/complexity-baseline.json").exists(),
        "{}",
        run.out
    );
}

#[test]
fn a_file_the_grammar_cannot_parse_is_out_of_scope_when_only_names_other_files() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/bad.rs", "%%% not rust %%%\n");
    tree.write("src/good.rs", "fn simple() -> i32 { 1 }\n");

    let run = tree.run(&["complexity", "--only", "src/good.rs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/bad.rs"), "{}", run.out);
}

#[test]
fn a_nested_function_is_measured_on_its_own_not_folded_into_the_one_around_it() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/nest.rs",
        "fn outer() {\n    fn inner(x: i32) -> i32 { if x > 0 { 1 } else { 0 } }\n    inner(1);\n}\n",
    );
    tree.write(
        "src/nest.py",
        "def outer():\n    def inner(x):\n        if x:\n            return 1\n        return 0\n    return inner\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(run.says("src/nest.rs:1  cc 1, 4 lines"), "{}", run.out);
    assert!(run.says("src/nest.rs:2  cc 2, 1 lines"), "{}", run.out);
    assert!(run.says("src/nest.py:1  cc 1, 6 lines"), "{}", run.out);
    assert!(run.says("src/nest.py:2  cc 2, 4 lines"), "{}", run.out);
}

#[test]
fn overlapping_sources_measure_each_file_once() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src", "src/deep"], "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/deep/c.rs", "fn f() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.out.matches("src/deep/c.rs:1").count(), 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
}

#[cfg(unix)]
#[test]
fn a_symlinked_directory_is_not_followed_so_a_loop_cannot_hang_the_run() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/c.rs", "fn f() {}\n");
    std::os::unix::fs::symlink(tree.path("src"), tree.path("src/loop")).expect("symlink");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.out.matches("fn f() {}").count(), 1, "{}", run.out);
}

#[test]
fn only_reports_a_judged_count_for_the_files_it_judged() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/a.rs", "fn a() {}\n");
    tree.write("src/b.rs", "fn b() {}\nfn c() {}\n");
    tree.run(&["complexity", "--write-baseline"]);

    let run = tree.run(&["complexity", "--only", "src/a.rs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 function(s) judged"), "{}", run.out);
}

#[test]
fn a_ceilings_key_of_the_wrong_shape_is_named_as_malformed_not_missing() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"], "ceilings": 8,
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"ceilings\" must be"), "{}", run.out);
}

#[test]
fn an_inserted_third_twin_is_the_new_one_not_a_neighbour() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/lib.rs", "fn f() {}\n// a\n// b\n// c\nfn f() {}\n");
    tree.run(&["complexity", "--write-baseline"]);
    tree.write(
        "src/lib.rs",
        "fn f() {}\n// a\nfn f() {}\n// b\n// c\nfn f() {}\n",
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
    assert!(run.says("src/lib.rs:3"), "{}", run.out);
    assert!(!run.says("matched nothing"), "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
}

#[test]
fn a_shared_value_keeps_a_moved_twin_matched_over_a_nearer_entry() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    let twin = ["fn twin() -> i32 {", "    if 1 > 0 { 1 } else { 0 }", "}"];
    let lines: Vec<&str> = std::iter::repeat_n("// pad", 18)
        .chain(twin)
        .chain(std::iter::repeat_n("// pad", 28))
        .chain(twin)
        .collect();
    tree.write("src/lib.rs", &(lines.join("\n") + "\n"));
    tree.write(
        "detent/complexity-baseline.json",
        &baseline(&format!(
            "{}, {}",
            entry("src/lib.rs", "fn twin() -> i32 {", 3, 2, 3),
            entry("src/lib.rs", "fn twin() -> i32 {", 20, 1, 3)
        )),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("src/lib.rs:50"), "{}", run.out);
    assert!(
        !run.says("src/lib.rs:19  cc 2, 3 lines, was"),
        "{}",
        run.out
    );
}

#[test]
fn typescript_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.ts", TYPESCRIPT);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "src/knot.ts:1  cc 11, 18 lines  function tangled(a: number, b: string[]): number {"
        ),
        "{}",
        run.out
    );
    assert!(!run.says("knot.ts:20"), "{}", run.out);
}

#[test]
fn go_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.go", GO);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/knot.go:3  cc 9, 17 lines  func tangled(a int, b []string) int {"),
        "{}",
        run.out
    );
    assert!(!run.says("knot.go:21"), "{}", run.out);
}

#[test]
fn java_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/Knot.java", JAVA);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/Knot.java:2  cc 11, 18 lines  int tangled(int a, String[] b) {"),
        "{}",
        run.out
    );
    assert!(!run.says("Knot.java:21"), "{}", run.out);
}

#[test]
fn ruby_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rb", RUBY);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/knot.rb:1  cc 10, 20 lines  def tangled(a, b)"),
        "{}",
        run.out
    );
    assert!(!run.says("knot.rb:22"), "{}", run.out);
}

#[test]
fn every_ecmascript_extension_is_measured_by_the_grammar_that_fits_it() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/a.ts", "function a(): number { return 1 as number; }\n");
    tree.write("src/b.tsx", "function b(): unknown { return <p>hi</p>; }\n");
    tree.write("src/c.jsx", "function c() { return <p>hi</p>; }\n");
    tree.write("src/d.mjs", "export function d() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    for file in ["src/a.ts:1", "src/b.tsx:1", "src/c.jsx:1", "src/d.mjs:1"] {
        assert!(run.says(file), "{}", run.out);
    }
}

#[test]
fn swift_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/Knot.swift", SWIFT);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says(
            "src/Knot.swift:1  cc 12, 19 lines  func tangled(_ a: Int, _ b: [String]) -> Int {"
        ),
        "{}",
        run.out
    );
    assert!(!run.says("Knot.swift:21"), "{}", run.out);
}

#[test]
fn kotlin_functions_carry_their_hand_checked_numbers() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/Knot.kt", KOTLIN);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("src/Knot.kt:1  cc 11, 18 lines  fun tangled(a: Int, b: List<String>): Int {"),
        "{}",
        run.out
    );
    assert!(!run.says("Knot.kt:20"), "{}", run.out);
}

#[test]
fn a_kotlin_script_is_measured_like_any_other_kotlin_file() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/build.kts", "fun one(): Int = 1\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/build.kts:1"), "{}", run.out);
}

#[test]
fn a_trailing_lambda_is_part_of_the_call_not_a_function_of_its_own() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/build.kts",
        "plugins { id(\"a\") }\nfun one(): Int = 1\n",
    );
    tree.write(
        "src/View.swift",
        "func body() -> Int {\n    return count(items) { x in x + 1 }\n}\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(!run.says("src/build.kts:1"), "{}", run.out);
    assert!(run.says("src/build.kts:2"), "{}", run.out);
    assert_eq!(run.out.matches("src/View.swift:").count(), 1, "{}", run.out);
}

#[test]
fn an_accessor_or_initializer_body_is_measured_like_any_other_function() {
    let tree = tree(r#"{"cc": 1, "lines": 60}"#);
    tree.write(
        "src/Acc.swift",
        "struct S {\n    var score: Int {\n        if stored > 0 { return 1 }\n        return 0\n    }\n}\n",
    );
    tree.write(
        "src/Acc.kt",
        "class C(val a: Int) {\n    init { if (a > 0) println(a) }\n}\n",
    );
    tree.write(
        "src/Acc.java",
        "class J {\n    static int s;\n    static { if (s > 0) s = 1; }\n}\n",
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/Acc.swift:2  cc 2"), "{}", run.out);
    assert!(run.says("src/Acc.kt:2  cc 2"), "{}", run.out);
    assert!(run.says("src/Acc.java:3  cc 2"), "{}", run.out);
}

#[test]
fn a_swift_accessor_is_measured_once_and_carries_its_own_branches() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/Prop.swift",
        "struct S {\n    var total: Int {\n        get {\n            if a { return 1 }\n            return 2\n        }\n    }\n}\n",
    );
    tree.write(
        "src/Sub.swift",
        "struct T {\n    subscript(i: Int) -> Int {\n        get { return a[i] }\n        set { a[i] = newValue }\n    }\n}\n",
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.out.matches("src/Prop.swift:").count(), 1, "{}", run.out);
    assert!(run.says("src/Prop.swift:3  cc 2"), "{}", run.out);
    assert_eq!(run.out.matches("src/Sub.swift:").count(), 2, "{}", run.out);
    assert!(run.says("src/Sub.swift:3  cc 1"), "{}", run.out);
    assert!(run.says("src/Sub.swift:4  cc 1"), "{}", run.out);
}

#[test]
fn a_fall_through_arm_is_not_a_decision() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/Pick.java",
        "class P {\n    int pick(int a) {\n        switch (a) {\n            case 1: return 1;\n            default: return 3;\n        }\n    }\n}\n",
    );
    tree.write(
        "src/Pick.swift",
        "func pick(_ a: Int) -> Int {\n    switch a {\n    case 1: return 1\n    default: return 3\n    }\n}\n",
    );
    tree.write(
        "src/Pick.kt",
        "fun pick(a: Int): Int {\n    return when (a) {\n        1 -> 1\n        else -> 3\n    }\n}\n",
    );
    tree.write(
        "src/pick.rs",
        "fn pick(n: i32) -> i32 {\n    match n {\n        1 => 1,\n        _ => 3,\n    }\n}\n",
    );
    tree.write(
        "src/pick.py",
        "def pick(n):\n    match n:\n        case 1:\n            return 1\n        case _:\n            return 3\n",
    );
    tree.write(
        "src/pick.go",
        "package main\n\nfunc pick(a int) int {\n\tswitch a {\n\tcase 1:\n\t\treturn 1\n\tdefault:\n\t\treturn 3\n\t}\n}\n",
    );
    tree.write(
        "src/pick.ts",
        "function pick(a: number): number {\n  switch (a) {\n    case 1: return 1;\n    default: return 3;\n  }\n}\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(run.says("src/Pick.java:2  cc 2"), "{}", run.out);
    assert!(run.says("src/Pick.swift:1  cc 2"), "{}", run.out);
    assert!(run.says("src/Pick.kt:1  cc 2"), "{}", run.out);
    assert!(run.says("src/pick.rs:1  cc 2"), "{}", run.out);
    assert!(run.says("src/pick.py:1  cc 2"), "{}", run.out);
    assert!(run.says("src/pick.go:3  cc 2"), "{}", run.out);
    assert!(run.says("src/pick.ts:1  cc 2"), "{}", run.out);
}

#[test]
fn a_guarded_catch_all_arm_is_still_a_decision() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/guarded.rs",
        "fn pick(n: i32) -> i32 {\n    match n {\n        _ if n > 2 => 1,\n        _ => 3,\n    }\n}\n",
    );
    tree.write(
        "src/guarded.py",
        "def pick(n):\n    match n:\n        case _ if n > 2:\n            return 1\n        case _:\n            return 3\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(run.says("src/guarded.rs:1  cc 2"), "{}", run.out);
    assert!(run.says("src/guarded.py:1  cc 2"), "{}", run.out);
}

#[test]
fn a_pattern_list_that_holds_a_wildcard_is_not_a_catch_all() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/pair.py",
        "def pick(n):\n    match n:\n        case 1, _:\n            return 1\n        case _:\n            return 3\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(run.says("src/pair.py:1  cc 2"), "{}", run.out);
}

#[test]
fn each_accessor_in_a_file_carries_its_own_site() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write(
        "src/Two.swift",
        "struct S {\n    var area: Int {\n        get {\n            return 1\n        }\n    }\n    var size: Int {\n        get {\n            return 2\n        }\n    }\n}\n",
    );
    tree.write(
        "src/Two.kt",
        "class C {\n    var area: Int\n        get() {\n            return 1\n        }\n    var size: Int\n        get() {\n            return 2\n        }\n}\n",
    );
    tree.write(
        "src/Sub.swift",
        "struct T {\n    subscript(i: Int) -> Int {\n        get {\n            return a[i]\n        }\n        set {\n            a[i] = newValue\n        }\n    }\n}\n",
    );

    let run = tree.run(&["complexity"]);
    assert!(run.says("var area: Int { get {"), "{}", run.out);
    assert!(run.says("var size: Int { get {"), "{}", run.out);
    assert!(run.says("var area: Int get() {"), "{}", run.out);
    assert!(run.says("var size: Int get() {"), "{}", run.out);
    assert!(run.says("subscript(i: Int) -> Int { get {"), "{}", run.out);
    assert!(run.says("subscript(i: Int) -> Int { set {"), "{}", run.out);
}

#[test]
fn a_vendored_directory_under_a_sources_root_is_not_measured() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["."], "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write(
        "node_modules/dep/index.ts",
        "function vendored() { return 1; }\n",
    );
    tree.write("web/app.ts", "function mine() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("web/app.ts:1"), "{}", run.out);
    assert!(!run.says("node_modules"), "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
}

#[test]
fn a_vendored_directory_named_as_a_source_is_measured() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["node_modules/dep"], "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write(
        "node_modules/dep/index.ts",
        "function vendored() { return 1; }\n",
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("node_modules/dep/index.ts:1"), "{}", run.out);
}

#[test]
fn skip_dirs_adds_to_the_default_list() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["."], "skip_dirs": ["legacy"],
             "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("legacy/old.rs", "fn old() {}\n");
    tree.write("src/new.rs", "fn new() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/new.rs:1"), "{}", run.out);
    assert!(!run.says("legacy"), "{}", run.out);
}

#[test]
fn only_the_named_languages_are_measured() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"], "languages": ["rust"],
             "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");
    tree.write("src/b.ts", "function b() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.rs:1"), "{}", run.out);
    assert!(!run.says("src/b.ts"), "{}", run.out);
}

#[test]
fn a_language_name_covers_every_grammar_the_escapes_gate_gives_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"], "languages": ["typescript"],
             "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/a.ts", "function a() { return 1; }\n");
    tree.write("src/b.tsx", "function b() { return 1; }\n");
    tree.write("src/c.js", "function c() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:1"), "{}", run.out);
    assert!(run.says("src/b.tsx:1"), "{}", run.out);
    assert!(!run.says("src/c.js"), "{}", run.out);
}

#[test]
fn an_unknown_language_is_refused_naming_the_ones_that_exist() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"], "languages": ["cobol"],
             "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("cobol"), "{}", run.out);
    assert!(run.says("rust"), "{}", run.out);
}

#[test]
fn an_exclude_glob_drops_a_file_and_exclude_except_keeps_a_named_path_back() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "sources": ["src"], "exclude": ["*test*"],
             "exclude_except": ["src/test-runner.ts"],
             "ceilings": {"cc": 0, "lines": 0},
             "baseline": "detent/complexity-baseline.json" } }"#,
    );
    tree.write("src/app.ts", "function app() { return 1; }\n");
    tree.write("src/app.test.ts", "function spec() { return 1; }\n");
    tree.write("src/test-runner.ts", "function runner() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/app.ts:1"), "{}", run.out);
    assert!(run.says("src/test-runner.ts:1"), "{}", run.out);
    assert!(!run.says("app.test.ts"), "{}", run.out);
    assert!(run.says("2 new function(s)"), "{}", run.out);
}

#[test]
fn a_baseline_from_the_old_measure_version_is_a_note_locally_and_a_failure_under_strict() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/knot.rs", RUST);
    tree.run(&["complexity", "--write-baseline"]);
    let stored =
        std::fs::read_to_string(tree.path("detent/complexity-baseline.json")).unwrap_or_default();
    tree.write(
        "detent/complexity-baseline.json",
        &stored.replace("\"version\": \"2\"", "\"version\": \"1\""),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("measured by complexity 1, this run by 2"),
        "{}",
        run.out
    );
    assert!(run.says("may not be comparable"), "{}", run.out);

    let strict = tree.run(&["complexity", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
}
