mod harness;

use harness::Tree;

const NEXT: &str = r#"{"name":"web","private":true,"dependencies":{"next":"15.0.0"}}"#;
const ALIASED: &str = r#"{"compilerOptions":{"paths":{"@/*":["./*"]}}}"#;

fn app() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("package.json", NEXT);
    tree.write(
        "app/layout.tsx",
        "export default function Layout() {\n  return null;\n}\n",
    );
    tree.write(
        "app/page.tsx",
        "import { Header } from \"../components/header\";\n\nexport default function Page() {\n  return Header();\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "export function Header() {\n  return null;\n}\n",
    );
    tree
}

fn reachability(tree: &Tree) -> harness::Run {
    tree.run(&["check", "reachability"])
}

#[test]
fn a_new_component_nothing_imports_fails() {
    let tree = app();
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
    assert!(!run.says("export Footer"), "{}", run.out);
}

#[test]
fn a_new_ts_file_nothing_imports_fails_and_one_a_reached_file_imports_passes() {
    let tree = app();
    tree.base();
    tree.write(
        "lib/format.ts",
        "export function formatNumber(n: number) {\n  return `${n}`;\n}\n",
    );

    let orphan = reachability(&tree);
    tree.write(
        "components/header.tsx",
        "import { formatNumber } from \"../lib/format\";\n\nexport function Header() {\n  return formatNumber(1);\n}\n",
    );
    let wired = reachability(&tree);

    assert_eq!(orphan.code, 1, "{}", orphan.out);
    assert!(orphan.says("lib/format.ts"), "{}", orphan.out);
    assert_eq!(wired.code, 0, "{}", wired.out);
    assert!(wired.says("4 export(s) judged, 0 unused"), "{}", wired.out);
}

#[test]
fn a_new_export_no_reached_file_imports_fails() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export function Header() {\n  return null;\n}\n\nexport function Banner() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/header.tsx:5"), "{}", run.out);
    assert!(run.says("export Banner"), "{}", run.out);
    assert!(run.says("drop the `export`"), "{}", run.out);
}

#[test]
fn a_use_inside_the_exporting_file_does_not_count() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export const title = \"Chat\";\n\nexport function Header() {\n  return title;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("export title"), "{}", run.out);
}

#[test]
fn an_entry_export_of_its_file_kind_passes_and_one_of_another_kind_fails() {
    let tree = app();
    tree.base();
    tree.write(
        "app/page.tsx",
        "import { Header } from \"../components/header\";\n\nexport const metadata = { title: \"Chat\" };\nexport const revalidate = 60;\nexport const GET = 1;\n\nexport default function Page() {\n  return Header();\n}\n",
    );
    tree.write(
        "app/api/chat/route.ts",
        "async function handler() {\n  return new Response(\"ok\");\n}\n\nexport { handler as GET, handler as POST };\nexport const maxDuration = 30;\n",
    );
    tree.write(
        "middleware.ts",
        "export function middleware() {}\n\nexport const config = { matcher: [] };\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("export GET"), "{}", run.out);
    assert!(
        run.says("1 new unreached file(s) or unused export(s)"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_next_dynamic_imports_is_reached() {
    let tree = app();
    tree.base();
    tree.write(
        "app/page.tsx",
        "import dynamic from \"next/dynamic\";\nimport { Header } from \"../components/header\";\n\nconst Chart = dynamic(() => import(\"../components/chart\"));\n\nexport default function Page() {\n  return [Header(), Chart];\n}\n",
    );
    tree.write(
        "components/chart.tsx",
        "export default function Chart() {\n  return null;\n}\n\nexport const height = 4;\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_package_with_no_entry_point_and_no_framework_klin_knows_is_a_hole_and_no_fail() {
    let tree = app();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"dependencies":{"react-router":"7.0.0"}}"#,
    );
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("app package(s) klin could not prove"),
        "{}",
        run.out
    );
    assert!(
        run.says("whose framework klin does not recognize"),
        "{}",
        run.out
    );
    assert!(!run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn an_import_of_a_file_the_tree_does_not_hold_turns_no_judgement_off() {
    let tree = app();
    tree.write(
        "app/page.tsx",
        "import type { Route } from \"./+types/page\";\nimport { Header } from \"../components/header\";\n\nexport default function Page(_: Route) {\n  return Header();\n}\n",
    );
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn an_alias_klin_cannot_prove_turns_the_whole_package_off() {
    let tree = app();
    tree.write(
        "tsconfig.json",
        r#"{"compilerOptions":{"paths":{"@/*":["./*","./src/*"]}}}"#,
    );
    tree.write(
        "app/page.tsx",
        "import { Header } from \"@/components/header\";\n\nexport default function Page() {\n  return Header();\n}\n",
    );
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("@/components/header"), "{}", run.out);
    assert!(!run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn an_import_that_names_several_files_reaches_each_of_them() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "import { Footer } from \"./footer\";\n\nexport function Header() {\n  return Footer();\n}\n",
    );
    tree.write(
        "components/footer.ts",
        "export function Footer() {\n  return null;\n}\n",
    );
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_re_export_reaches_its_target_only_from_a_reached_file() {
    let tree = app();
    tree.base();
    tree.write(
        "components/index.ts",
        "export { Footer } from \"./footer\";\n",
    );
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/index.ts"), "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn an_export_is_judged_through_the_graph_and_not_by_its_name() {
    let tree = app();
    tree.write(
        "lib/utils.ts",
        "export function generateId() {\n  return \"id\";\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "import { generateId } from \"../lib/utils\";\nimport { slug } from \"../lib/ids\";\n\nexport function Header() {\n  return [generateId(), slug];\n}\n",
    );
    tree.write("lib/ids.ts", "export const slug = \"chat\";\n");
    tree.base();
    tree.write(
        "lib/ids.ts",
        "export const slug = \"chat\";\n\nexport function generateId() {\n  return slug;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("lib/ids.ts:3"), "{}", run.out);
    assert!(run.says("export generateId"), "{}", run.out);
}

#[test]
fn a_namespace_import_uses_every_export_of_its_module() {
    let tree = app();
    tree.write(
        "app/page.tsx",
        "import * as header from \"../components/header\";\n\nexport default function Page() {\n  return header.Header();\n}\n",
    );
    tree.base();
    tree.write(
        "components/header.tsx",
        "export function Header() {\n  return null;\n}\n\nexport const height = 4;\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_module_only_its_test_imports_fails_and_the_test_is_no_finding() {
    let tree = app();
    tree.base();
    tree.write(
        "lib/format.ts",
        "export function formatNumber(n: number) {\n  return `${n}`;\n}\n",
    );
    tree.write(
        "lib/format.test.ts",
        "import { formatNumber } from \"./format\";\n\nformatNumber(1);\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("lib/format.ts"), "{}", run.out);
    assert!(!run.says("lib/format.test.ts"), "{}", run.out);
}

#[test]
fn a_kit_file_gets_the_file_rule_and_not_the_export_rule() {
    let tree = app();
    tree.write("tsconfig.json", ALIASED);
    tree.write(
        "components.json",
        r#"{"aliases":{"components":"@/components","ui":"@/components/ui"}}"#,
    );
    tree.write(
        "components/ui/button.tsx",
        "export function Button() {\n  return null;\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "import { Button } from \"@/components/ui/button\";\n\nexport function Header() {\n  return Button();\n}\n",
    );
    tree.base();
    tree.write(
        "components/ui/button.tsx",
        "export function Button() {\n  return null;\n}\n\nexport const buttonVariants = {};\n",
    );
    tree.write(
        "components/ui/card.tsx",
        "export function Card() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/ui/card.tsx"), "{}", run.out);
    assert!(!run.says("buttonVariants"), "{}", run.out);
}

#[test]
fn a_file_a_package_script_runs_is_an_entry() {
    let tree = app();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"scripts":{"seed":"tsx scripts/seed.ts"},"dependencies":{"next":"15.0.0"}}"#,
    );
    tree.base();
    tree.write("scripts/seed.ts", "console.log(\"seeded\");\n");

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_file_the_section_excepts_is_excluded_and_not_judged() {
    let tree = app();
    tree.write(
        "klin.json",
        r#"{"reachability":{"except":["components/footer.tsx"]}}"#,
    );
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("1 excluded"), "{}", run.out);
}

#[test]
fn an_orphan_the_base_held_stays_a_note() {
    let tree = app();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );
    tree.base();
    tree.write(
        "components/header.tsx",
        "export function Header() {\n  return \"header\";\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("1 unreached file(s) the base already held"),
        "{}",
        run.out
    );
}

#[test]
fn a_file_that_loses_its_last_import_is_worsened() {
    let tree = app();
    tree.base();
    tree.write(
        "app/page.tsx",
        "export default function Page() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("got worse"), "{}", run.out);
    assert!(run.says("components/header.tsx"), "{}", run.out);
}

#[test]
fn the_stop_blocks_a_turn_that_adds_an_orphan_component() {
    let tree = app();
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = tree.stop();

    assert_eq!(run.code, 2, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}
