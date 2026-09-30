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

const TANGLED: &str = r##"fn tangled(a: i32) -> i32 {
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
    let ceilings: serde_json::Value = serde_json::from_str(ceilings)
        .unwrap_or_else(|why| panic!("the ceilings are not JSON: {why}"));
    format!(
        r#"{{ "complexity": {{ "in": "src", "cc": {}, "lines": {} }} }}"#,
        ceilings["cc"], ceilings["lines"]
    )
}

fn tree(ceilings: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", &config(ceilings));
    tree
}

fn accepted(entries: &str) -> String {
    format!(
        r#"{{ "accepted": [{entries}],
             "complexity": {{ "in": "src", "cc": 8, "lines": 60 }} }}"#
    )
}

#[test]
fn compact_policy_selects_source_and_excludes_a_subtree() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{"complexity":{"cc":0,"lines":60,"in":"src","except":"src/generated"}}"#,
    );
    tree.write("src/read.rs", "fn read() {}\n");
    tree.write("src/generated/write.rs", "fn write() {}\n");
    tree.write("tools/run.rs", "fn run() {}\n");

    let run = tree.run(&["complexity"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/read.rs:1"), "{}", run.out);
    assert!(!run.says("src/generated/write.rs:1"), "{}", run.out);
    assert!(!run.says("tools/run.rs:1"), "{}", run.out);
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
fn a_function_over_the_gate_at_the_base_is_held() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.write("src/knot.py", PYTHON);
    tree.base();

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("OK: 5 function(s) judged, 2 over the gate"),
        "{}",
        run.out
    );
}

#[test]
fn a_function_that_moved_down_the_file_still_matches_the_base() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.base();
    tree.write("src/knot.rs", &format!("// a header\n// and more\n{RUST}"));

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_function_whose_cyclomatic_grew_since_the_base_fails() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.base();
    tree.write(
        "src/knot.rs",
        &RUST.replace("a == 0 ||", "a == 0 || a == -2 ||"),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(
        run.says("cc 10, 13 lines, was cc 9, 13 lines"),
        "{}",
        run.out
    );
}

#[test]
fn a_function_whose_length_grew_since_the_base_fails_too() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.base();
    tree.write(
        "src/knot.rs",
        &RUST.replace(
            "fn tangled(a: i32) -> i32 {\n",
            "fn tangled(a: i32) -> i32 {\n    let _ = a;\n",
        ),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(
        run.says("cc 9, 14 lines, was cc 9, 13 lines"),
        "{}",
        run.out
    );
}

#[test]
fn a_function_that_improved_since_the_base_passes_with_nothing_to_say() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.base();
    tree.write("src/knot.rs", &RUST.replace("a == 0 || a == -1", "a == 0"));

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("NOTE"), "{}", run.out);
}

#[test]
fn a_function_under_the_ceiling_that_grew_is_not_judged() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "src/small.rs",
        "fn f(a: i32) -> i32 {\n    if a > 0 { 1 } else { 0 }\n}\n",
    );
    tree.base();
    tree.write(
        "src/small.rs",
        "fn f(a: i32) -> i32 {\n    if a > 0 && a < 9 { 1 } else { 0 }\n}\n",
    );

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/small.rs"), "{}", run.out);
}

#[test]
fn a_renamed_file_is_measured_at_its_old_path() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", RUST);
    tree.base();
    tree.git(&["mv", "src/knot.rs", "src/moved.rs"]);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/moved.rs"), "{}", run.out);
}

#[test]
fn an_accepted_entry_holds_a_function_at_its_value_and_fails_above_it() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/knot.rs",
                "text": "fn tangled(a: i32) -> i32 {", "cc": 9, "lines": 13}"#,
        ),
    );
    tree.write("src/knot.rs", RUST);

    let held = tree.run(&["complexity"]);
    assert_eq!(held.code, 0, "{}", held.out);

    tree.write(
        "src/knot.rs",
        &RUST.replace("a == 0 ||", "a == 0 || a == -2 ||"),
    );
    let worse = tree.run(&["complexity"]);
    assert_eq!(worse.code, 1, "{}", worse.out);
    assert!(worse.says("got worse"), "{}", worse.out);
}

#[test]
fn an_accepted_entry_the_base_also_holds_stays_matched_under_strict() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/knot.rs",
                "text": "fn tangled(a: i32) -> i32 {", "cc": 9, "lines": 13}"#,
        ),
    );
    tree.write("src/knot.rs", RUST);
    tree.base();

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("matched nothing"), "{}", run.out);
}

#[test]
fn an_accepted_entry_that_names_some_of_the_values_is_a_tool_error() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/knot.rs",
                "text": "fn tangled(a: i32) -> i32 {", "cc": 9}"#,
        ),
    );
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("does not give a number for lines"), "{}", run.out);
}

#[test]
fn a_stale_accepted_entry_does_not_make_a_site_the_base_holds_worse() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "accepted": [{"gate": "complexity", "file": "src/lib.rs",
                           "text": "fn f(a: bool) -> i32 {", "cc": 2, "lines": 6}],
             "complexity": { "in": "src", "cc": 0, "lines": 0 } }"#,
    );
    let body = |test: &str| {
        format!(
            "fn f(a: bool) -> i32 {{\n    if {test} {{\n        1\n    }} else {{\n        0\n    }}\n}}\n"
        )
    };
    tree.write("src/lib.rs", &body("a && a"));
    tree.base();
    tree.write("src/lib.rs", &body("a"));

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
    assert!(run.says("matched nothing"), "{}", run.out);
}

#[test]
fn an_accepted_entry_whose_value_is_not_a_number_is_a_tool_error() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/knot.rs",
                "text": "fn tangled(a: i32) -> i32 {", "cc": 9, "lines": null}"#,
        ),
    );
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("does not give a number for lines"), "{}", run.out);
}

#[test]
fn an_accepted_entry_that_matches_nothing_is_a_note_and_a_strict_failure() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/gone.rs", "text": "fn vanished() {",
                "cc": 20, "lines": 40}"#,
        ),
    );
    tree.write("src/simple.rs", "fn f() -> i32 { 1 }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("1 accepted entry matched nothing"), "{}", run.out);

    let strict = tree.run(&["complexity", "--strict"]);
    assert_eq!(strict.code, 1, "{}", strict.out);
    assert!(strict.says("matched nothing"), "{}", strict.out);
}

#[test]
fn a_key_the_section_leaves_out_is_derived_beside_the_one_it_pins() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "complexity": { "cc": 8, "lines": 60 } }"#);
    tree.write("src/knot.rs", RUST);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("pinned: complexity cc 8"), "{}", run.out);
    assert!(run.says("pinned: complexity lines 60"), "{}", run.out);

    tree.write("klin.json", r#"{ "complexity": { "in": "src" } }"#);
    let derived_ceilings = tree.run(&["complexity"]);
    assert_eq!(derived_ceilings.code, 0, "{}", derived_ceilings.out);
    assert!(
        derived_ceilings.says("derived: complexity cc 10 (the floor of 10"),
        "{}",
        derived_ceilings.out
    );
}

/// Fifty functions whose 95th percentile is cc 9, committed as the base, and a new function
/// of cc 10 beside them, which a floor below 10 would fail.
fn a_percentile_below_the_floor() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let simple = (0..47)
        .map(|at| format!("fn simple_{at}() -> i32 {{ 1 }}\n"))
        .collect::<String>();
    tree.write("src/simple.rs", &simple);
    let tangled = (0..3)
        .map(|at| TANGLED.replacen("fn tangled", &format!("fn tangled_{at}"), 1))
        .collect::<String>();
    tree.write("src/tangled.rs", &tangled);
    tree.base();
    tree.write(
        "src/knot.rs",
        &TANGLED.replace("a == 0 ||", "a == 0 || a == -2 ||"),
    );
    tree
}

#[test]
fn a_percentile_below_ten_derives_a_cc_ceiling_of_ten() {
    let tree = a_percentile_below_the_floor();

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("derived: complexity cc 10 (the floor of 10, over 50 function(s) at"),
        "{}",
        run.out
    );
    assert!(!run.says("src/knot.rs"), "{}", run.out);
}

#[test]
fn a_pinned_cc_below_ten_still_judges_at_the_pinned_value() {
    let tree = a_percentile_below_the_floor();
    tree.write("klin.json", r#"{ "complexity": { "cc": 8 } }"#);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("pinned: complexity cc 8"), "{}", run.out);
    assert!(run.says("src/knot.rs:1  cc 10, 13 lines"), "{}", run.out);
}

#[test]
fn an_uncommitted_scope_uses_the_whole_repository_sample_its_commit_recorded() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let many = (0..50)
        .map(|at| format!("fn outside_{at}() {{ if true {{}} }}\n"))
        .collect::<String>();
    tree.write("src/outside.rs", &many);
    tree.write("src/small/inside.rs", "fn inside() {}\n");
    tree.base();

    let whole = tree.run(&["complexity"]);
    assert_eq!(whole.code, 0, "{}", whole.out);
    assert!(whole.says("over 51 function(s)"), "{}", whole.out);

    tree.write("klin.json", r#"{"complexity":{"in":"src/small"}}"#);
    let scoped = tree.run(&["complexity"]);
    assert_eq!(scoped.code, 0, "{}", scoped.out);
    assert!(scoped.says("over 51 function(s)"), "{}", scoped.out);
    assert!(scoped.says("today's complexity scope"), "{}", scoped.out);
}

#[test]
fn an_explicit_scope_with_no_applicable_file_is_a_configuration_error() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("has an \"in\" scope with no applicable file"),
        "{}",
        run.out
    );
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
fn overlapping_roots_measure_each_file_once() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "cc": 0, "lines": 0 } }"#,
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
    tree.base();

    let run = tree.run(&["complexity", "--only", "src/a.rs"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 1 function(s) judged"), "{}", run.out);
}

#[test]
fn a_ceiling_of_the_wrong_shape_is_named_as_malformed_not_missing() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "cc": "eight", "lines": 60 } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("\"cc\" must be a whole number"), "{}", run.out);
}

#[test]
fn an_inserted_third_twin_is_the_new_one_not_a_neighbour() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    tree.write("src/lib.rs", "fn f() {}\n// a\n// b\n// c\nfn f() {}\n");
    tree.base();
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
fn a_shared_value_keeps_a_moved_twin_matched_over_a_nearer_function() {
    let tree = tree(r#"{"cc": 0, "lines": 0}"#);
    let branching = ["fn twin() -> i32 {", "    if 1 > 0 { 1 } else { 0 }", "}"];
    let plain = ["fn twin() -> i32 {", "    1", "}"];
    let laid_out = |first: [&str; 3], second: [&str; 3], lead: usize, gap: usize| {
        let lines: Vec<&str> = std::iter::repeat_n("// pad", lead)
            .chain(first)
            .chain(std::iter::repeat_n("// pad", gap))
            .chain(second)
            .collect();
        lines.join("\n") + "\n"
    };
    tree.write("src/lib.rs", &laid_out(branching, plain, 2, 14));
    tree.base();
    tree.write("src/lib.rs", &laid_out(branching, branching, 18, 28));

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
fn a_growing_suite_callback_in_a_test_file_is_not_a_complexity_finding() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity":{"cc":8,"test_lines":25}}"#);
    let fillers = "  void 0;\n".repeat(30);
    let containers = [
        "describe('suite',",
        "describe.each([1])('suite',",
        "describe.only('suite',",
        "describe.skip('suite',",
        "context('suite',",
        "context.each([1])('suite',",
        "context.only('suite',",
        "context.skip('suite',",
        "suite('suite',",
        "suite.each([1])('suite',",
        "suite.only('suite',",
        "suite.skip('suite',",
        "fdescribe('suite',",
        "xdescribe('suite',",
    ];
    let mut source = containers
        .iter()
        .enumerate()
        .map(|(at, container)| {
            format!("{container} () => {{\n{fillers}  it('existing {at}', () => {{}});\n}});\n")
        })
        .collect::<String>();
    tree.write("tests/suites.ts", &source);
    let mut javascript =
        format!("describe('javascript', () => {{\n{fillers}  it('existing', () => {{}});\n}});\n");
    tree.write("tests/suites.js", &javascript);
    tree.base();

    let added = source.rfind("});").expect("last suite closes");
    source.insert_str(added, "  it('added', () => {});\n");
    tree.write("tests/suites.ts", &source);
    let added = javascript.rfind("});").expect("javascript suite closes");
    javascript.insert_str(added, "  it('added', () => {});\n");
    tree.write("tests/suites.js", &javascript);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("17 function(s) judged"), "{}", run.out);
}

#[test]
fn a_long_test_callback_inside_a_suite_is_still_measured() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity":{"cc":8,"test_lines":25}}"#);
    let fillers = "    void 0;\n".repeat(30);
    tree.write(
        "src/suite.test.ts",
        &format!("describe('suite', () => {{\n  it('long', () => {{\n{fillers}  }});\n}});\n"),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/suite.test.ts:2"), "{}", run.out);
    assert!(run.says("it('long', () => {"), "{}", run.out);
}

#[test]
fn suite_callbacks_do_not_raise_the_derived_lines_ceiling() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let mut source = String::from("describe('short tests', () => {\n");
    for at in 0..47 {
        source.push_str(&format!("  it('case {at}', () => {{}});\n"));
    }
    source.push_str("});\n");
    let fillers = "  void 0;\n".repeat(30);
    for at in 0..3 {
        source.push_str(&format!(
            "describe('long suite {at}', () => {{\n{fillers}}});\n"
        ));
    }
    tree.write("tests/suites.test.ts", &source);
    tree.base();

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity lines 25"), "{}", run.out);
    assert!(run.says("over 47 function(s)"), "{}", run.out);
}

#[test]
fn a_suite_callback_in_a_production_file_is_still_measured() {
    let tree = tree(r#"{"cc":8,"lines":25}"#);
    let fillers = "  void 0;\n".repeat(30);
    tree.write(
        "src/suites.ts",
        &format!("describe('suite', () => {{\n{fillers}}});\n"),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/suites.ts:1"), "{}", run.out);
    assert!(run.says("describe('suite', () => {"), "{}", run.out);
}

#[test]
fn a_call_returned_by_a_suite_name_is_not_a_suite_container() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity":{"cc":8,"test_lines":25}}"#);
    let fillers = "  void 0;\n".repeat(30);
    for (file, call) in [
        ("tests/returned.test.ts", "describe()"),
        ("tests/returned-member.test.ts", "describe.only()"),
    ] {
        tree.write(
            file,
            &format!("{call}('not a suite', () => {{\n{fillers}}});\n"),
        );
    }

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("tests/returned.test.ts:1"), "{}", run.out);
    assert!(run.says("tests/returned-member.test.ts:1"), "{}", run.out);
}

#[test]
fn typescript_wrappers_do_not_change_suite_callback_classification() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{"complexity":{"cc":8,"test_lines":25}}"#);
    let fillers = "  void 0;\n".repeat(30);
    tree.write(
        "tests/wrapped.test.ts",
        &format!(
            "describe('as', (() => {{\n{fillers}}}) as () => void);\n\
             describe('satisfies', (() => {{\n{fillers}}}) satisfies () => void);\n\
             describe('non-null', (() => {{\n{fillers}}})!);\n\
             describe('assertion', <() => void>(() => {{\n{fillers}}}));\n"
        ),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
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
fn a_vendored_directory_under_a_root_is_not_measured() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "complexity": { "cc": 0, "lines": 0 } }"#);
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
fn an_explicit_scope_inside_a_skipped_directory_names_the_coverage_problem() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "node_modules/dep", "cc": 0, "lines": 0 } }"#,
    );
    tree.write(
        "node_modules/dep/index.ts",
        "function vendored() { return 1; }\n",
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(
        run.says("has an \"in\" scope with no applicable file"),
        "{}",
        run.out
    );
}

#[test]
fn a_retired_skip_dirs_key_is_rejected_actionably() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "skip_dirs": ["legacy"], "cc": 0, "lines": 0 } }"#,
    );
    tree.write("legacy/old.rs", "fn old() {}\n");
    tree.write("src/new.rs", "fn new() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"skip_dirs\""), "{}", run.out);
}

#[test]
fn mixed_languages_are_measured_in_one_policy() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "cc": 0, "lines": 0 } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");
    tree.write("src/b.ts", "function b() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.rs:1"), "{}", run.out);
    assert!(run.says("src/b.ts:1"), "{}", run.out);
}

#[test]
fn a_language_name_covers_every_grammar_the_escapes_gate_gives_it() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "cc": 0, "lines": 0 } }"#,
    );
    tree.write("src/a.ts", "function a() { return 1; }\n");
    tree.write("src/b.tsx", "function b() { return 1; }\n");
    tree.write("src/c.js", "function c() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/a.ts:1"), "{}", run.out);
    assert!(run.says("src/b.tsx:1"), "{}", run.out);
    assert!(run.says("src/c.js:1"), "{}", run.out);
}

#[test]
fn a_retired_language_selector_is_rejected_actionably() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "languages": ["cobol"], "cc": 0, "lines": 0 } }"#,
    );
    tree.write("src/a.rs", "fn a() {}\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("no longer reads \"languages\""), "{}", run.out);
}

#[test]
fn except_drops_a_subtree() {
    let tree = Tree::new();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "except": "src/tests", "cc": 0, "lines": 0 } }"#,
    );
    tree.write("src/app.ts", "function app() { return 1; }\n");
    tree.write("src/tests/app.ts", "function spec() { return 1; }\n");
    tree.write("src/test-runner.ts", "function runner() { return 1; }\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/app.ts:1"), "{}", run.out);
    assert!(run.says("src/test-runner.ts:1"), "{}", run.out);
    assert!(!run.says("src/tests/app.ts"), "{}", run.out);
    assert!(run.says("2 new function(s)"), "{}", run.out);
}

#[test]
fn a_file_the_grammar_cannot_parse_is_exit_two_under_strict_too() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/bad.rs", "%%% not rust %%%\n");
    tree.write("src/good.rs", "fn simple() -> i32 { 1 }\n");

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("src/bad.rs"), "{}", run.out);
}

#[test]
fn a_function_moved_to_another_file_with_its_body_unchanged_keeps_its_site() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/knot.rs", "");
    tree.write("src/moved.rs", TANGLED);

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/moved.rs"), "{}", run.out);
}

#[test]
fn a_moved_function_whose_body_changed_is_new() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/knot.rs", "");
    tree.write(
        "src/moved.rs",
        &TANGLED.replace("    match a {", "    let _ = a;\n    match a {"),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
    assert!(run.says("src/moved.rs:1"), "{}", run.out);
}

#[test]
fn a_copy_of_a_function_beside_its_original_is_new() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/copy.rs", TANGLED);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
    assert!(run.says("src/copy.rs:1"), "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
}

#[test]
fn two_identical_bodies_that_moved_match_one_to_one() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/one.rs", TANGLED);
    tree.write("src/two.rs", TANGLED);
    tree.base();
    tree.write("src/one.rs", "");
    tree.write("src/two.rs", "");
    tree.write("src/three.rs", TANGLED);
    tree.write("src/four.rs", TANGLED);

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("new function(s)"), "{}", run.out);
}

#[test]
fn two_moved_bodies_take_the_entry_that_shares_their_values() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    let spread = TANGLED.replace("    match a {", "\n    match a {");
    tree.write("src/one.rs", TANGLED);
    tree.write("src/two.rs", &spread);
    tree.base();
    tree.write("src/one.rs", "");
    tree.write("src/two.rs", "");
    tree.write("src/three.rs", TANGLED);
    tree.write("src/four.rs", &spread);

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("worse"), "{}", run.out);
}

#[test]
fn a_whitespace_only_reformat_of_a_moved_body_still_matches() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/knot.rs", "");
    tree.write("src/moved.rs", &TANGLED.replace("\n    ", "\n        "));

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/moved.rs"), "{}", run.out);
}

#[test]
fn a_renamed_function_whose_body_did_not_change_keeps_its_site() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write(
        "src/knot.rs",
        &TANGLED.replace("fn tangled(", "fn untangled("),
    );

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("untangled"), "{}", run.out);
}

#[test]
fn a_one_line_function_that_moved_keeps_its_site() {
    let tree = tree(r#"{"cc": 1, "lines": 60}"#);
    tree.write("src/one.rs", "fn a() -> i32 { if true { 1 } else { 0 } }\n");
    tree.base();
    tree.write("src/one.rs", "");
    tree.write("src/two.rs", "fn a() -> i32 { if true { 1 } else { 0 } }\n");

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/two.rs"), "{}", run.out);
}

#[test]
fn an_accepted_entry_beside_the_base_entry_does_not_free_it_for_a_copy() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write(
        "klin.json",
        &accepted(
            r#"{"gate": "complexity", "file": "src/knot.rs",
                "text": "fn tangled(a: i32) -> i32 {", "cc": 9, "lines": 13}"#,
        ),
    );
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/copy.rs", TANGLED);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
    assert!(run.says("src/copy.rs:1"), "{}", run.out);
}

#[test]
fn the_lower_of_two_bodies_in_one_file_takes_the_moved_entry() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write(
        "src/knot.rs",
        &format!(
            "{}\n{}",
            TANGLED.replace("fn tangled(", "fn zzz("),
            TANGLED.replace("fn tangled(", "fn aaa(")
        ),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("1 new function(s)"), "{}", run.out);
    assert!(run.says("fn aaa("), "{}", run.out);
    assert!(!run.says("fn zzz("), "{}", run.out);
}

#[test]
fn a_function_that_moved_and_grew_names_the_site_it_matched() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);
    tree.base();
    tree.write("src/knot.rs", "");
    tree.write(
        "src/moved.rs",
        &TANGLED.replace("    match a {", "\n    match a {"),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("at src/knot.rs"), "{}", run.out);
}

#[test]
fn a_file_measured_at_the_base_and_excluded_now_is_a_note_naming_it() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/kept.rs", "fn simple() -> i32 { 1 }\n");
    tree.write("src/gone.rs", "fn other() -> i32 { 2 }\n");
    tree.base();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "except": "src/gone.rs",
             "cc": 8, "lines": 60 } }"#,
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("NOTE: src/gone.rs was measured at the base"),
        "{}",
        run.out
    );
    assert!(run.says("an exclusion drops it now"), "{}", run.out);
    assert!(!run.says("src/kept.rs was measured"), "{}", run.out);
}

#[test]
fn a_file_measured_at_the_base_and_not_now_is_exit_two_under_strict() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/gone.rs", "fn other() -> i32 { 2 }\n");
    tree.base();
    tree.write(
        "klin.json",
        r#"{ "complexity": { "in": "src", "except": "src/gone.rs",
             "cc": 8, "lines": 60 } }"#,
    );

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("FAIL:"), "{}", run.out);
    assert!(run.says("src/gone.rs"), "{}", run.out);
}

#[test]
fn a_file_added_or_deleted_in_the_window_is_not_a_coverage_loss() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/deleted.rs", "fn other() -> i32 { 2 }\n");
    tree.write("src/same.rs", "fn same() -> i32 { 3 }\n");
    tree.base();
    tree.remove("src/deleted.rs");
    tree.write("src/added.rs", "fn added() -> i32 { 4 }\n");

    let run = tree.run(&["complexity", "--strict"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("measured at the base"), "{}", run.out);
}

#[test]
fn a_file_the_grammar_refuses_now_is_unreadable_and_a_coverage_loss_too() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/bad.rs", "fn fine() -> i32 { 1 }\n");
    tree.base();
    tree.write("src/bad.rs", "%%% not rust %%%\n");

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("could not parse"), "{}", run.out);
    assert!(
        run.says("NOTE: src/bad.rs was measured at the base"),
        "{}",
        run.out
    );
    assert!(run.says("the grammar refused it"), "{}", run.out);
}

#[test]
fn a_scope_spelled_with_dot_slash_or_a_trailing_slash_measures_its_files() {
    let tree = Tree::new();
    tree.write("real/knot.rs", RUST);
    tree.base();
    for root in ["./real", "real/"] {
        tree.write(
            "klin.json",
            &format!(r#"{{ "complexity": {{ "in": "{root}", "cc": 1, "lines": 60 }} }}"#),
        );
        let run = tree.run(&["complexity"]);
        assert_eq!(run.code, 0, "{root}: {}", run.out);
        assert!(run.says("1 over the gate"), "{root}: {}", run.out);
        assert!(run.says("1 measured"), "{root}: {}", run.out);
    }
}

#[test]
fn failure_output_asks_for_a_design_fix_and_not_a_split_to_the_number() {
    let tree = tree(r#"{"cc": 8, "lines": 60}"#);
    tree.write("src/knot.rs", TANGLED);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("Reduce the function's responsibility or decision complexity."),
        "{}",
        run.out
    );
    assert!(
        run.says("coherent behavior boundaries, not into arbitrary helpers"),
        "{}",
        run.out
    );
    assert!(
        run.says("policy decision for a person, in the config, in a reviewed commit."),
        "{}",
        run.out
    );
}

/// A Rust function of `lines` lines that decides nothing.
fn long(name: &str, lines: usize) -> String {
    format!(
        "fn {name}() {{\n{}}}\n",
        "    let a = 1;\n".repeat(lines - 2)
    )
}

/// A Rust function of cc 11, over the derived floor of 10, in 13 lines.
fn knotted(name: &str) -> String {
    TANGLED
        .replace("a == 0 ||", "a == 0 || a == -2 || a == -3 ||")
        .replacen("fn tangled", &format!("fn {name}"), 1)
}

fn inline_tests(body: &str) -> String {
    format!("fn production() -> i32 {{ 1 }}\n\n#[cfg(test)]\nmod tests {{\n{body}}}\n")
}

#[test]
fn with_no_test_lines_a_test_function_past_the_lines_ceiling_does_not_fail() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("tests/long.rs", &long("long_test", 40));
    tree.write("src/lib.rs", &inline_tests(&long("long_inline_test", 40)));
    tree.write(
        "web/long.test.ts",
        &format!("function longCase() {{\n{}}}\n", "  void 0;\n".repeat(38)),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("derived: complexity lines 25"), "{}", run.out);
}

#[test]
fn with_no_test_lines_a_test_function_past_the_cc_ceiling_still_fails() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("tests/knot.rs", &knotted("knot_test"));
    tree.write("src/lib.rs", &inline_tests(&knotted("inline_knot")));

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("tests/knot.rs:1  cc 11"), "{}", run.out);
    assert!(run.says("src/lib.rs:5  cc 11"), "{}", run.out);
}

#[test]
fn a_test_function_whose_length_grew_is_held_while_its_cc_holds_with_no_test_lines() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let knot = knotted("knot_test");
    tree.write("tests/knot.rs", &knot);
    tree.base();
    tree.write(
        "tests/knot.rs",
        &knot.replace(
            "    match a {",
            &format!("{}    match a {{", "    let _ = a;\n".repeat(20)),
        ),
    );

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn with_test_lines_pinned_a_test_function_past_it_fails() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "complexity": { "test_lines": 30 } }"#);
    tree.write("tests/long.rs", &long("long_test", 40));
    tree.write("src/lib.rs", &inline_tests(&long("long_inline_test", 40)));
    tree.write("tests/short.rs", &long("short_test", 28));

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("pinned: complexity test_lines 30"), "{}", run.out);
    assert!(run.says("tests/long.rs:1  cc 1, 40 lines"), "{}", run.out);
    assert!(run.says("src/lib.rs:5  cc 1, 40 lines"), "{}", run.out);
    assert!(!run.says("tests/short.rs:1"), "{}", run.out);
    assert!(!run.says("not judged on length"), "{}", run.out);
}

#[test]
fn with_only_lines_pinned_a_production_function_is_judged_and_a_test_function_is_not() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "complexity": { "lines": 30 } }"#);
    tree.write("src/long.rs", &long("long_production", 40));
    tree.write("tests/long.rs", &long("long_test", 40));

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/long.rs:1  cc 1, 40 lines"), "{}", run.out);
    assert!(!run.says("tests/long.rs:1"), "{}", run.out);
}

#[test]
fn with_no_test_lines_the_coverage_line_says_test_code_was_not_judged_on_length() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("tests/old.rs", &long("old_test", 5));
    tree.write("tests/before.rs", &long("renamed_test", 5));
    tree.write("src/lib.rs", &inline_tests(&long("inline_test", 5)));
    tree.base();
    tree.write("tests/new.rs", &long("new_test", 5));
    tree.git(&["mv", "tests/before.rs", "tests/after.rs"]);

    let run = tree.run(&["complexity"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("4 test function(s) not judged on length, with no test_lines pinned"),
        "{}",
        run.out
    );
    assert!(
        run.says("added or renamed: tests/after.rs, tests/new.rs"),
        "{}",
        run.out
    );
    assert!(!run.says("tests/old.rs"), "{}", run.out);
}

#[test]
fn test_code_stays_in_the_derived_sample_for_both_ceilings() {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    let simple = (0..47)
        .map(|at| format!("fn simple_{at}() -> i32 {{ 1 }}\n"))
        .collect::<String>();
    tree.write("src/simple.rs", &simple);
    let tests = (0..3)
        .map(|at| {
            knotted(&format!("knot_{at}")).replace(
                "    match a {",
                &format!("{}    match a {{", "    let _ = a;\n".repeat(20)),
            )
        })
        .collect::<String>();
    tree.write("tests/knots.rs", &tests);
    tree.base();

    let run = tree.run(&["complexity"]);
    assert!(
        run.says("derived: complexity cc 11 (95th percentile of 50"),
        "{}",
        run.out
    );
    assert!(
        run.says("derived: complexity lines 33 (95th percentile of 50"),
        "{}",
        run.out
    );
}
