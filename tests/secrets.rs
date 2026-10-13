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
