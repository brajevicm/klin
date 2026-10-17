mod harness;

use std::path::Path;

use harness::Tree;

/// A token body no placeholder word can hide in, `length` characters long.
fn body(length: usize) -> String {
    "a1B2c3D4e5F6g7H8j9K0"
        .chars()
        .cycle()
        .take(length)
        .collect()
}

/// A token shaped like a live one, built here so no full token sits in the repository.
fn token(prefix: &str, body: &str) -> String {
    [prefix, body].concat()
}

fn openai() -> String {
    token("sk-proj-", &body(48))
}

/// One token of every vendor the check reads, at its usual length.
fn every_vendor() -> Vec<String> {
    vec![
        openai(),
        token("sk-ant-", &["api03-", &body(93)].concat()),
        token("sk_live_", &body(24)),
        token("ghp_", &body(36)),
        token("github_pat_", &body(82)),
        token("AKIA", "Q7W2E5R8N3Y6U9P4"),
        token("xoxb-", "1234567890-1234567890123-a1B2c3D4e5F6g7H8j9K0a1B2"),
        token("AIza", &body(35)),
        token("glpat-", &body(20)),
    ]
}

fn source(key: &str) -> String {
    format!("const KEY = \"{key}\";\nexport function key() {{\n  return KEY;\n}}\n")
}

/// Whether any file under this directory holds the text.
fn holds(directory: &Path, text: &str) -> bool {
    std::fs::read_dir(directory)
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| {
            let path = entry.path();
            match path.is_dir() {
                true => holds(&path, text),
                false => String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default())
                    .contains(text),
            }
        })
}

#[test]
fn a_new_openai_project_key_fails() {
    let tree = Tree::new();
    tree.write("src/app.ts", &source(&openai()));

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new secret(s)"), "{}", run.out);
    assert!(run.says("src/app.ts:1"), "{}", run.out);
    assert!(run.says("OpenAI project key"), "{}", run.out);
}

#[test]
fn every_vendor_token_of_its_usual_length_fails() {
    let tree = Tree::new();
    let lines: String = every_vendor()
        .iter()
        .enumerate()
        .map(|(at, key)| format!("const KEY_{at} = \"{key}\";\n"))
        .collect();
    tree.write("src/app.ts", &lines);

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 9 new secret(s)"), "{}", run.out);
    assert!(run.says("src/app.ts:9"), "{}", run.out);
}

#[test]
fn a_token_shorter_than_its_usual_length_passes() {
    let tree = Tree::new();
    tree.write("src/app.ts", &source(&token("sk-proj-", &body(12))));

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn no_text_json_or_journal_holds_the_key() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/app.ts", "export const one = 1;\n");
    tree.base();
    let key = openai();
    tree.write("src/app.ts", &source(&key));
    let tail = &key[key.len() - 16..];

    let text = tree.run(&["check", "secrets"]);
    let json = tree.run(&["check", "secrets", "--json"]);
    let stop = tree.stop();

    assert_eq!(text.code, 1, "{}", text.out);
    assert_eq!(
        json.json()["findings"][0]["check"],
        "secrets",
        "{}",
        json.out
    );
    assert!(stop.says("secret"), "{}", stop.out);
    for run in [&text, &json, &stop] {
        assert!(!run.says(tail), "{}", run.out);
    }
    assert!(!holds(&tree.path(".git/klin"), tail));
}

#[test]
fn a_token_that_holds_a_placeholder_word_passes() {
    let tree = Tree::new();
    let placeholders = [
        token("sk-proj-", &["test", &body(44)].concat()),
        token("ghp_", &["xxx", &body(33)].concat()),
        token("AKIA", "IOSFODNN7EXAMPLE"),
        token("xoxb-", "test-token-a1B2c3D4e5F6"),
    ];
    let lines: String = placeholders
        .iter()
        .map(|key| format!("const KEY = \"{key}\";\n"))
        .collect();
    tree.write("src/app.ts", &lines);

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("OK: 0 secret(s) in the tree"), "{}", run.out);
}

#[test]
fn a_person_accepts_a_secret_without_writing_the_key() {
    let tree = Tree::new();
    let key = openai();
    tree.write("src/app.ts", &source(&key));
    let found = tree.run(&["check", "secrets", "--json"]).json();
    let text = found["findings"][0]["text"]
        .as_str()
        .unwrap_or_default()
        .to_string();
    assert!(!text.is_empty() && !key.contains(&text), "{found}");

    let entry = serde_json::json!({
        "accepted": [{"gate": "secrets", "file": "src/app.ts", "text": text, "count": 1}]
    });
    tree.write("klin.json", &entry.to_string());

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("all on the accepted list"), "{}", run.out);
}

#[test]
fn a_key_the_base_holds_passes_where_its_line_moved() {
    let tree = Tree::new();
    tree.write("src/app.ts", &source(&openai()));
    tree.base();
    tree.write("src/app.ts", &format!("// moved\n{}", source(&openai())));

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("all held at the base"), "{}", run.out);
}

#[test]
fn a_tracked_env_file_is_read_and_an_ignored_one_is_not() {
    let tree = Tree::new();
    tree.write("src/app.ts", "export const one = 1;\n");
    tree.write(".gitignore", ".env\n");
    tree.write(".env", &format!("OPENAI_API_KEY={}\n", openai()));
    tree.write(
        ".env.production",
        &format!("GITHUB_TOKEN={}\n", token("ghp_", &body(36))),
    );

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 1 new secret(s)"), "{}", run.out);
    assert!(run.says(".env.production:1"), "{}", run.out);
}

#[test]
fn a_key_in_a_test_file_fails() {
    let tree = Tree::new();
    tree.write("src/app.test.ts", &source(&openai()));

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/app.test.ts:1"), "{}", run.out);
}

#[test]
fn a_token_cut_short_by_an_ellipsis_or_a_slack_placeholder_passes() {
    let tree = Tree::new();
    let lines = format!(
        "const A = \"{}...\";\nconst B = \"{}\";\n",
        openai(),
        token("xoxb-", "your-bot-token-goes-here")
    );
    tree.write("src/app.ts", &lines);

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_key_that_ends_in_a_dash_before_its_quote_fails() {
    let tree = Tree::new();
    let lines = format!(
        "const A = \"{}\";\nconst B = \"{}\";\n",
        token("AIza", &[&body(34), "-"].concat()),
        token("glpat-", &[&body(19), "-"].concat())
    );
    tree.write("src/app.ts", &lines);

    let run = tree.run(&["check", "secrets"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("FAIL: 2 new secret(s)"), "{}", run.out);
}

#[test]
fn a_tree_that_holds_only_an_env_file_is_read() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write(".env.production", &format!("OPENAI_API_KEY={}\n", openai()));

    let run = tree.run(&["check"]);
    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says(".env.production:1"), "{}", run.out);
}

#[test]
fn an_escape_on_a_line_that_holds_a_key_names_the_fingerprint_and_no_part_of_the_key() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write(
        "src/lib.rs",
        &format!("fn f() {{\n    let k = \"{key}\".parse::<u8>().unwrap();\n}}\n"),
    );

    let text = tree.run(&["check", "escapes"]);
    let json = tree.run(&["check", "escapes", "--json"]);
    let stop = tree.stop();

    assert_eq!(text.code, 1, "{}", text.out);
    assert!(text.says("let k = \"sk-proj-..."), "{}", text.out);
    assert_eq!(
        json.json()["findings"][0]["check"],
        "escapes",
        "{}",
        json.out
    );
    for run in [&text, &json, &stop] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
    assert!(!holds(&tree.path(".git/klin"), &tail[..12]));
}

#[test]
fn an_escape_on_a_line_with_no_token_keeps_its_text_and_its_identity() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {\n    x.unwrap();\n}\n");

    let json = tree.run(&["check", "escapes", "--json"]);

    let finding = &json.json()["findings"][0];
    assert_eq!(finding["text"], "x.unwrap();", "{}", json.out);
    assert_eq!(finding["id"], "ff2f547c7c081f35", "{}", json.out);
}

#[test]
fn the_journal_prompt_excerpt_holds_no_part_of_a_key_typed_into_the_prompt() {
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let event = format!(
        r#"{{"hook_event_name": "UserPromptSubmit", "session_id": "s-1", "prompt": "use {key} here"}}"#
    );

    let run = harness::feed(tree.root(), harness::AGENT, &event);

    assert_eq!(run.code, 0, "{}", run.out);
    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(journal.contains("use sk-proj-..."), "{journal}");
    assert!(!journal.contains(&tail[..12]), "{journal}");
}

#[test]
fn an_accepted_entry_that_holds_a_key_matches_nothing_and_no_output_prints_the_key() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let line = format!("let k = \"{key}\".parse::<u8>().unwrap();");
    tree.write("src/lib.rs", &format!("fn f() {{\n    {line}\n}}\n"));
    let entry = serde_json::json!({
        "escapes": { "in": "src" },
        "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": line, "count": 1}]
    });
    tree.write("klin.json", &entry.to_string());

    let text = tree.run(&["check", "escapes"]);
    let json = tree.run(&["check", "escapes", "--json"]);

    assert_eq!(text.code, 1, "{}", text.out);
    assert!(text.says("sk-proj-..."), "{}", text.out);
    for run in [&text, &json] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

#[test]
fn policy_prints_an_accepted_entry_that_holds_a_key_with_the_fingerprint() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write("src/lib.rs", "fn f() {}\n");
    let entry = serde_json::json!({
        "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": format!("k = \"{key}\""),
                      "count": 1, "reason": format!("rotated {key}")}]
    });
    tree.write("klin.json", &entry.to_string());

    let text = tree.run(&["policy"]);
    let json = tree.run(&["policy", "--json"]);

    assert!(text.says("k = \"sk-proj-..."), "{}", text.out);
    for run in [&text, &json] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

#[test]
fn an_escape_on_a_line_with_a_placeholder_token_still_hides_the_token() {
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    let key = token("sk-proj-", &["test", &body(44)].concat());
    tree.write(
        "src/lib.rs",
        &format!("fn f() {{\n    let k = \"{key}\".parse::<u8>().unwrap();\n}}\n"),
    );

    let run = tree.run(&["check", "escapes"]);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("let k = \"sk-proj-..."), "{}", run.out);
    assert!(!run.says(&key["sk-proj-".len()..][..12]), "{}", run.out);
}

#[test]
fn an_accepted_entry_with_a_key_and_no_count_names_no_part_of_the_key_in_its_error() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write("src/lib.rs", "fn f() {}\n");
    let entry = serde_json::json!({
        "escapes": { "in": "src" },
        "accepted": [{"gate": "escapes", "file": format!("src/{key}.rs"), "text": format!("k = \"{key}\"")}]
    });
    tree.write("klin.json", &entry.to_string());

    let runs = [
        tree.run(&["check", "escapes"]),
        tree.run(&["check", "escapes", "--json"]),
        tree.run(&["policy"]),
        tree.run(&["policy", "--json"]),
    ];

    for run in &runs {
        assert_eq!(run.code, 2, "{}", run.out);
        assert!(run.says("sk-proj-..."), "{}", run.out);
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

#[test]
fn an_unmatched_accepted_entry_names_no_part_of_a_key_in_its_file_or_any_value() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let entry = serde_json::json!({
        "escapes": { "in": "src" },
        "accepted": [{"gate": "escapes", "file": format!("src/{key}.rs"), "text": "x.unwrap();",
                      "count": 1, "metadata": {"note": key.clone()}}]
    });
    tree.write("klin.json", &entry.to_string());

    let text = tree.run(&["check", "escapes"]);
    let json = tree.run(&["check", "escapes", "--json"]);
    let stop = tree.stop();

    assert!(text.says("sk-proj-..."), "{}", text.out);
    for run in [&text, &json, &stop] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
    assert!(!holds(&tree.path(".git/klin"), &tail[..12]));
}

#[test]
fn policy_names_no_part_of_a_key_used_as_a_property_name() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write("src/lib.rs", "fn f() {}\n");
    let entry = format!(
        r#"{{ "accepted": [{{"gate": "escapes", "file": "src/lib.rs", "text": "x", "count": 1, "{key}": 1}}] }}"#
    );
    tree.write("klin.json", &entry);

    for run in [tree.run(&["policy"]), tree.run(&["policy", "--json"])] {
        assert!(run.says("sk-proj-..."), "{}", run.out);
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

#[test]
fn a_convention_remedy_that_holds_a_key_names_no_part_of_it() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let entry = serde_json::json!({
        "conventions": {"no-unwrap": {"text": ".unwrap()", "in": "src", "remedy": format!("Ask for {key}.")}}
    });
    tree.write("klin.json", &entry.to_string());
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.write("src/lib.rs", "fn f() {\n    x.unwrap();\n}\n");

    let text = tree.run(&["check", "conventions"]);
    let json = tree.run(&["check", "conventions", "--json"]);
    let stop = tree.stop();

    assert_eq!(text.code, 1, "{}", text.out);
    assert!(text.says("Ask for sk-proj-..."), "{}", text.out);
    for run in [&text, &json, &stop] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
    assert!(!holds(&tree.path(".git/klin"), &tail[..12]));
}

#[test]
fn a_key_after_an_escaped_character_names_no_part_of_it_in_json() {
    let tree = Tree::new();
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    tree.write("src/lib.rs", "fn f() {}\n");
    let entry = serde_json::json!({
        "accepted": [{"gate": "escapes", "file": "src/lib.rs", "text": "x", "count": 1,
                      "reason": format!("line\n{key}\tand\"{key}")}]
    });
    tree.write("klin.json", &entry.to_string());

    for run in [tree.run(&["policy"]), tree.run(&["policy", "--json"])] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

/// A tree whose base holds `src/a.ts`, with the file changed since, so a Stop judges the turn.
fn changed_tree(config: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", config);
    tree.write("src/a.ts", "export const one = 1;\n");
    tree.base();
    tree.write("src/a.ts", "export const one = 2;\n");
    tree
}

#[test]
fn an_unplaced_sarif_result_that_holds_a_key_names_no_part_of_it_in_a_check() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = changed_tree(r#"{"sarif": [{"name": "eslint", "report": "eslint.sarif"}]}"#);
    tree.write(
        "eslint.sarif",
        &format!(
            r#"{{"version": "2.1.0", "runs": [{{"tool": {{"driver": {{"name": "eslint"}}}},
                "results": [{{"ruleId": "r", "message": {{"text": "leaked {key}"}}, "locations": []}}]}}]}}"#
        ),
    );

    let text = tree.run(&["check", "eslint"]);
    let json = tree.run(&["check", "eslint", "--json"]);

    assert!(text.says("leaked sk-proj-..."), "{}", text.out);
    assert!(json.says("leaked sk-proj-..."), "{}", json.out);
    for run in [&text, &json] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

#[test]
fn an_integration_run_command_that_holds_a_key_names_no_part_of_it_in_the_document() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let config = serde_json::json!({
        "sarif": [{"name": "eslint", "report": "eslint.sarif", "run": format!("scan --token {key}")}]
    });
    let tree = changed_tree(&config.to_string());
    tree.write(
        "eslint.sarif",
        r#"{"version": "2.1.0", "runs": [{"tool": {"driver": {"name": "eslint"}}, "results": []}]}"#,
    );

    let json = tree.run(&["check", "eslint", "--json"]);
    let (stop, report) = tree.stop_report();

    let basis = json.json()["measurements"].to_string();
    assert!(basis.contains("scan --token sk-proj-..."), "{}", json.out);
    for run in [&json, &stop] {
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
    assert!(!report.to_string().contains(&tail[..12]), "{report}");
    assert!(!holds(&tree.path(".git/klin"), &tail[..12]));
}

#[test]
fn a_finding_in_a_file_whose_name_holds_a_key_names_no_part_of_it_in_the_journal() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.write(&format!("src/{key}.rs"), "fn g() {\n    x.unwrap();\n}\n");

    let (stop, report) = tree.stop_report();

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(report.to_string().contains("escapes"), "{report}");
    assert!(journal.contains("src/sk-proj-..."), "{}", stop.out);
    assert!(!report.to_string().contains(&tail[..12]), "{report}");
    assert!(!journal.contains(&tail[..12]), "{journal}");
}

#[test]
fn a_command_line_that_holds_a_key_names_no_part_of_it_in_the_usage_error() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();

    for args in [
        vec![key.as_str()],
        vec!["check", "--sarif", "a", key.as_str()],
    ] {
        let run = tree.run(&args);
        assert_eq!(run.code, 2, "{}", run.out);
        assert!(!run.says(&tail[..12]), "{}", run.out);
    }
}

const PROMPT: &str =
    r#"{"hook_event_name": "UserPromptSubmit", "session_id": "s-1", "prompt": "go"}"#;

/// Every file under the state directory that holds the text.
fn state_holding(tree: &Tree, text: &str) -> Vec<String> {
    fn walk(dir: &Path, text: &str, out: &mut Vec<String>) {
        for entry in std::fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            match path.is_dir() {
                true => walk(&path, text, out),
                false => {
                    let body = String::from_utf8_lossy(&std::fs::read(&path).unwrap_or_default())
                        .into_owned();
                    if body.contains(text) {
                        out.push(path.display().to_string());
                    }
                }
            }
        }
    }
    let mut out = Vec::new();
    walk(&tree.path(".git/klin"), text, &mut out);
    out
}

#[test]
fn a_config_error_that_echoes_a_key_names_no_part_of_it_in_the_stop_or_its_state() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    assert_eq!(harness::feed(tree.root(), harness::AGENT, PROMPT).code, 0);
    tree.write("klin.json", &format!("{{\"{key}\": 1}}\n"));
    tree.write("src/lib.rs", "fn f() {}\nfn g() {}\n");

    let stop = tree.stop();

    assert!(stop.says("nothing judged"), "{}", stop.out);
    assert!(stop.says("sk-proj-..."), "{}", stop.out);
    assert!(!stop.says(&tail[..12]), "{}", stop.out);
    assert_eq!(state_holding(&tree, &tail[..12]), Vec::<String>::new());
}

#[test]
fn a_failing_build_that_prints_a_key_names_no_part_of_it_in_the_stop_or_its_state() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();
    let config =
        serde_json::json!({ "build": format!("echo token {key}; echo {key} >&2; exit 1") });
    tree.write("klin.json", &config.to_string());
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    assert_eq!(harness::feed(tree.root(), harness::AGENT, PROMPT).code, 0);
    tree.write("src/lib.rs", "fn f() {}\nfn g() {}\n");

    let stop = tree.stop();

    assert!(stop.says("token sk-proj-..."), "{}", stop.out);
    assert!(!stop.says(&tail[..12]), "{}", stop.out);
    assert_eq!(state_holding(&tree, &tail[..12]), Vec::<String>::new());
}

#[test]
fn a_plan_error_that_echoes_a_key_names_no_part_of_it_in_the_stop_or_its_state() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let config = serde_json::json!({
        "build": false,
        "sarif": [{"name": key.clone(), "report": "a.sarif"}, {"name": key.clone(), "report": "b.sarif"}]
    });
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    assert_eq!(harness::feed(tree.root(), harness::AGENT, PROMPT).code, 0);
    tree.write("klin.json", &config.to_string());
    tree.write("src/lib.rs", "fn f() {}\nfn g() {}\n");

    let stop = tree.stop();

    assert!(stop.says("sk-proj-..."), "{}", stop.out);
    assert!(!stop.says(&tail[..12]), "{}", stop.out);
    assert_eq!(state_holding(&tree, &tail[..12]), Vec::<String>::new());
}

#[test]
fn an_unknown_host_argument_that_holds_a_key_names_no_part_of_it() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    let host = format!("--host={key}");
    let args = [harness::AGENT, &[host.as_str()]].concat();

    let run = harness::feed(tree.root(), &args, PROMPT);

    assert!(run.says("sk-proj-..."), "{}", run.out);
    assert!(!run.says(&tail[..12]), "{}", run.out);
}

#[test]
fn a_key_that_the_prompt_excerpt_cuts_short_leaks_no_more_than_its_prefix() {
    let key = openai();
    let tail = &key["sk-proj-".len()..];
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let lead = "x".repeat(60);
    let event = format!(
        r#"{{"hook_event_name": "UserPromptSubmit", "session_id": "s-1", "prompt": "{lead} {key}"}}"#
    );

    assert_eq!(harness::feed(tree.root(), harness::AGENT, &event).code, 0);

    let journal = std::fs::read_to_string(tree.state("journal.jsonl")).unwrap_or_default();
    assert!(journal.contains("sk-proj-..."), "{journal}");
    assert!(!journal.contains(&tail[..8]), "{journal}");
}

#[test]
fn a_branch_name_shaped_like_a_key_reads_back_from_the_turn_stamp_unchanged() {
    let branch = token("feature/AKIA", "QWERTYUIOPASDFGH");
    let tree = Tree::new();
    tree.write("klin.json", "{}\n");
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.git(&["checkout", "-q", "-b", &branch]);

    assert_eq!(harness::feed(tree.root(), harness::AGENT, PROMPT).code, 0);

    let stamp = std::fs::read_to_string(tree.state("turn")).unwrap_or_default();
    assert!(stamp.contains(&branch), "{stamp}");
}

fn cursor_event(kind: &str, tree: &Tree, extra: serde_json::Value) -> String {
    let mut event = serde_json::json!({
        "hook_event_name": kind,
        "cursor_version": "3.20.21",
        "conversation_id": "s1",
        "session_id": "s1",
        "workspace_roots": [tree.root()],
    });
    if let (Some(event), Some(extra)) = (event.as_object_mut(), extra.as_object()) {
        event.extend(extra.clone());
    }
    event.to_string()
}

#[test]
fn a_cursor_followup_that_names_a_key_is_recognized_when_the_host_submits_it() {
    let key = openai();
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    let opened = cursor_event("sessionStart", &tree, serde_json::json!({}));
    assert_eq!(harness::feed(tree.root(), harness::AGENT, &opened).code, 0);
    tree.write(&format!("src/{key}.rs"), "fn g() {\n    x.unwrap();\n}\n");

    let stop = cursor_event("stop", &tree, serde_json::json!({ "loop_count": 0 }));
    let blocked = harness::feed(tree.root(), harness::AGENT, &stop);
    let answer: serde_json::Value = serde_json::from_str(blocked.printed.trim())
        .unwrap_or_else(|why| panic!("{why}: {}", blocked.out));
    let followup = answer["followup_message"].as_str().unwrap_or_default();
    assert!(followup.contains("src/sk-proj-..."), "{answer}");
    let prompts = tree.field("prompts");

    let submitted = cursor_event(
        "beforeSubmitPrompt",
        &tree,
        serde_json::json!({ "prompt": followup }),
    );
    let echoed = harness::feed(tree.root(), harness::AGENT, &submitted);

    assert_eq!(echoed.code, 0, "{}", echoed.out);
    assert_eq!(echoed.out, "", "the follow-up opened a turn");
    assert_eq!(
        tree.field("prompts"),
        prompts,
        "the follow-up raised the counter"
    );
}

#[test]
fn a_session_id_shaped_like_a_key_still_finds_its_prompt_event() {
    let session = openai();
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "doc_size": {"README.md": 10} }"#);
    tree.words("README.md", 5);
    tree.base();
    let prompt = serde_json::json!({
        "hook_event_name": "UserPromptSubmit", "session_id": session.clone(), "prompt": "go"
    });
    assert_eq!(
        harness::feed(tree.root(), harness::AGENT, &prompt.to_string()).code,
        0
    );
    tree.words("README.md", 30);

    let stop = |again: bool| {
        let event = serde_json::json!({
            "hook_event_name": "Stop", "session_id": session.clone(), "stop_hook_active": again
        });
        harness::feed(tree.root(), harness::AGENT, &event.to_string())
    };

    let first = stop(false);
    let second = stop(true);

    assert_eq!(first.code, 2, "{}", first.out);
    assert!(second.says("FAIL  doc-size"), "{}", second.out);
    assert!(!second.says("session and prompt hooks"), "{}", second.out);
}

#[test]
fn a_finding_in_a_file_whose_name_holds_a_key_is_accepted_as_the_report_names_it() {
    let key = openai();
    let tree = Tree::new();
    tree.write("klin.json", r#"{ "escapes": { "in": "src" } }"#);
    tree.write("src/lib.rs", "fn f() {}\n");
    tree.base();
    tree.write(&format!("src/{key}.rs"), "fn g() {\n    x.unwrap();\n}\n");
    let found = tree.run(&["check", "escapes", "--json"]).json();
    let finding = &found["findings"][0];
    let entry = serde_json::json!({
        "escapes": { "in": "src" },
        "accepted": [{"gate": "escapes", "file": finding["file"], "text": finding["text"], "count": 1}]
    });
    assert!(entry.to_string().contains("src/sk-proj-..."), "{found}");
    tree.write("klin.json", &entry.to_string());

    let check = tree.run(&["check", "escapes"]);
    let stop = tree.stop();

    assert_eq!(check.code, 0, "{}", check.out);
    assert!(check.says("all on the accepted list"), "{}", check.out);
    assert!(!check.says("nothing matched"), "{}", check.out);
    assert!(!stop.says("FAIL  escapes"), "{}", stop.out);
}
