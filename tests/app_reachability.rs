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
fn a_kit_file_the_components_json_ui_alias_names_gets_neither_rule() {
    let tree = app();
    tree.write("tsconfig.json", ALIASED);
    tree.write("components.json", r#"{"aliases":{"ui":"@/ui"}}"#);
    tree.write(
        "ui/button.tsx",
        "export function Button() {\n  return null;\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "import { Button } from \"@/ui/button\";\n\nexport function Header() {\n  return Button();\n}\n",
    );
    tree.base();
    tree.write(
        "ui/button.tsx",
        "export function Button() {\n  return null;\n}\n\nexport const buttonVariants = {};\n",
    );
    tree.write(
        "ui/card.tsx",
        "export function Card() {\n  return null;\n}\n",
    );
    tree.write(
        "components/ui/badge.tsx",
        "export function Badge() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("ui/card.tsx"), "{}", run.out);
    assert!(!run.says("buttonVariants"), "{}", run.out);
    assert!(run.says("components/ui/badge.tsx"), "{}", run.out);
}

#[test]
fn with_no_components_json_a_kit_under_components_ui_gets_neither_rule() {
    let tree = app();
    tree.base();
    tree.write(
        "components/ui/card.tsx",
        "export function Card() {\n  return null;\n}\n",
    );
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("components/ui/card.tsx"), "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn a_config_file_a_tool_reads_at_the_package_root_is_an_entry() {
    let tree = app();
    tree.base();
    tree.write(
        "tailwind.config.ts",
        "const config = { content: [] };\n\nexport default config;\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_exported_type_is_no_finding_and_a_value_an_exported_type_names_is_used() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export const sizes = [\"sm\", \"lg\"] as const;\nexport type Size = (typeof sizes)[number];\nexport interface Props {\n  size: Size;\n}\nexport const unused = 1;\n\nexport function Header() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("export unused"), "{}", run.out);
    for held in ["export sizes", "export Size", "export Props"] {
        assert!(!run.says(held), "{held}\n{}", run.out);
    }
}

#[test]
fn a_file_the_base_held_unreached_keeps_its_exports_held_once_it_is_reached() {
    let tree = app();
    tree.write(
        "components/footer.tsx",
        "export const legacy = 1;\n\nexport function Footer() {\n  return null;\n}\n",
    );
    tree.base();
    tree.write(
        "components/footer.tsx",
        "export const legacy = 1;\nexport const extra = 2;\n\nexport function Footer() {\n  return null;\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "import { Footer } from \"./footer\";\n\nexport function Header() {\n  return Footer();\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("export extra"), "{}", run.out);
    assert!(!run.says("export legacy"), "{}", run.out);
}

#[test]
fn an_export_only_an_unreached_file_imports_is_no_finding_of_its_own() {
    let tree = app();
    tree.base();
    tree.write(
        "lib/icons.ts",
        "export const PlusIcon = \"+\";\nexport const MinusIcon = \"-\";\n",
    );
    tree.write(
        "components/header.tsx",
        "import { PlusIcon } from \"../lib/icons\";\n\nexport function Header() {\n  return PlusIcon;\n}\n",
    );
    tree.write(
        "components/old.tsx",
        "import { MinusIcon } from \"../lib/icons\";\n\nexport function Old() {\n  return MinusIcon;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/old.tsx"), "{}", run.out);
    assert!(!run.says("export MinusIcon"), "{}", run.out);
}

#[test]
fn a_value_an_exported_interface_names_only_as_a_property_is_still_unused() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export const status = \"ready\";\nexport interface Props {\n  status: string;\n}\n\nexport function Header() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("export status"), "{}", run.out);
}

#[test]
fn a_value_a_type_a_local_export_clause_exposes_reads_through_typeof_is_used() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export const sizes = [\"sm\", \"lg\"] as const;\ntype Size = (typeof sizes)[number];\nexport type { Size };\n\nexport function Header() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn an_alias_klin_cannot_prove_in_a_test_leaves_the_app_judged() {
    let tree = app();
    tree.write(
        "tsconfig.json",
        r#"{"compilerOptions":{"paths":{"@/*":["./*","./src/*"]}}}"#,
    );
    tree.write(
        "components/header.test.tsx",
        "import { Header } from \"@/components/header\";\n\nHeader();\n",
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
fn an_unused_export_blocks_no_stop_and_fails_at_check() {
    let tree = app();
    tree.base();
    tree.write(
        "components/header.tsx",
        "export function Header() {\n  return null;\n}\n\nexport function Banner() {\n  return null;\n}\n",
    );

    let stop = tree.stop();
    let check = reachability(&tree);

    assert_eq!(stop.code, 0, "{}", stop.out);
    assert_eq!(check.code, 1, "{}", check.out);
    assert!(check.says("export Banner"), "{}", check.out);
}

#[test]
fn a_route_the_change_renames_private_is_not_judged_and_a_private_folder_is() {
    let route = "export async function GET() {\n  return new Response(\"auth\");\n}\n";
    let tree = app();
    tree.write("app/api/auth/route.ts", route);
    tree.base();
    tree.remove("app/api/auth/route.ts");
    tree.write("app/api/auth/_route.ts", route);
    tree.write(
        "app/_components/card.tsx",
        "export function Card() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("app/_components/card.tsx"), "{}", run.out);
    assert!(!run.says("_route.ts:0  unreached"), "{}", run.out);
}

#[test]
fn a_new_file_named_like_a_disabled_route_is_judged() {
    let tree = app();
    tree.base();
    tree.write(
        "app/chat/_page.tsx",
        "export default function Page() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("app/chat/_page.tsx"), "{}", run.out);
}

#[test]
fn a_kit_alias_the_change_writes_leaves_nothing_out() {
    let tree = app();
    tree.write("tsconfig.json", ALIASED);
    tree.write("components.json", r#"{"aliases":{"ui":"@/ui"}}"#);
    tree.base();
    tree.write("components.json", r#"{"aliases":{"ui":"@/lib"}}"#);
    tree.write(
        "lib/format.ts",
        "export function formatNumber(n: number) {\n  return `${n}`;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("lib/format.ts"), "{}", run.out);
}

#[test]
fn a_paths_rule_the_change_writes_moves_no_kit() {
    let tree = app();
    tree.write("tsconfig.json", ALIASED);
    tree.write("components.json", r#"{"aliases":{"ui":"@/ui"}}"#);
    tree.base();
    tree.write(
        "tsconfig.json",
        r#"{"compilerOptions":{"paths":{"@/*":["./lib/*"]}}}"#,
    );
    tree.write(
        "lib/ui/orphan.tsx",
        "export function Orphan() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("lib/ui/orphan.tsx"), "{}", run.out);
}

#[test]
fn a_script_the_change_writes_makes_no_entry() {
    let tree = app();
    tree.base();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"scripts":{"x":"tsx lib/orphan.ts"},"dependencies":{"next":"15.0.0"}}"#,
    );
    tree.write("lib/orphan.ts", "console.log(\"orphan\");\n");

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("lib/orphan.ts"), "{}", run.out);
}

#[test]
fn a_vendored_mark_the_change_writes_leaves_nothing_out() {
    let tree = app();
    tree.base();
    tree.write(".gitattributes", "* linguist-vendored\n");
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn a_renamed_gitattributes_marks_only_where_the_base_held_it() {
    let tree = app();
    tree.write("vendor/.gitattributes", "*.tsx linguist-vendored\n");
    tree.base();
    tree.git(&["mv", "vendor/.gitattributes", "components/.gitattributes"]);
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn a_renamed_vendored_directory_stays_left_out() {
    let tree = app();
    tree.write("vendor/.gitattributes", "*.tsx linguist-vendored\n");
    tree.write(
        "vendor/kit.tsx",
        "export function Kit() {\n  return null;\n}\n",
    );
    tree.base();
    tree.git(&["mv", "vendor", "third-party"]);

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_bin_or_workspaces_field_leaves_a_next_app_judged() {
    let tree = app();
    tree.base();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"bin":"./cli.js","workspaces":["packages/*"],"dependencies":{"next":"15.0.0"}}"#,
    );
    tree.write(
        "components/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("components/footer.tsx"), "{}", run.out);
}

#[test]
fn a_file_gitattributes_marks_vendored_gets_neither_rule() {
    let tree = app();
    tree.write(
        ".gitattributes",
        "components/ai-elements/** linguist-vendored\n",
    );
    tree.write(
        "components/ai-elements/message.tsx",
        "export function Message() {\n  return null;\n}\n",
    );
    tree.write(
        "components/header.tsx",
        "import { Message } from \"./ai-elements/message\";\n\nexport function Header() {\n  return Message();\n}\n",
    );
    tree.base();
    tree.write(
        "components/ai-elements/message.tsx",
        "export function Message() {\n  return null;\n}\n\nexport const messageVariants = {};\n",
    );
    tree.write(
        "components/ai-elements/tool.tsx",
        "export function Tool() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn a_package_that_is_no_app_gets_no_note() {
    let tree = app();
    tree.write(
        "tools/package.json",
        r#"{"name":"tools","private":true,"devDependencies":{"typescript":"5.9.3"}}"#,
    );
    tree.write("tools/run.ts", "console.log(\"tools\");\n");
    tree.write(
        "cli/package.json",
        r#"{"name":"cli","bin":"./main.ts","dependencies":{"react":"19.0.0"}}"#,
    );
    tree.write("cli/main.ts", "console.log(\"cli\");\n");
    tree.base();

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("could not prove"), "{}", run.out);
}

#[test]
fn a_file_any_package_script_names_is_an_entry_even_one_a_linter_only_reads() {
    let tree = app();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"scripts":{"lint":"eslint src/legacy.ts"},"dependencies":{"next":"15.0.0"}}"#,
    );
    tree.base();
    tree.write("src/legacy.ts", "console.log(\"legacy\");\n");

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
}

#[test]
fn the_judged_count_includes_the_entries() {
    let tree = app();
    tree.base();

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(run.says("3 file(s) judged"), "{}", run.out);
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

const VITE: &str =
    r#"{"name":"web","private":true,"type":"module","devDependencies":{"vite":"6.0.0"}}"#;

fn vite() -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write("package.json", VITE);
    tree.write(
        "index.html",
        "<!doctype html>\n<html>\n  <body>\n    <div id=\"root\"></div>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>\n  </body>\n</html>\n",
    );
    tree.write("src/main.tsx", "import { App } from \"./app\";\n\nApp();\n");
    tree.write(
        "src/app.tsx",
        "export function App() {\n  return null;\n}\n",
    );
    tree
}

#[test]
fn a_vite_app_is_judged_from_its_index_html_module_script() {
    let tree = vite();

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("could not prove"), "{}", run.out);
    assert!(run.says("1 export(s) judged, 0 unused"), "{}", run.out);
}

#[test]
fn a_new_file_nothing_imports_in_a_vite_app_fails() {
    let tree = vite();
    tree.base();
    tree.write(
        "src/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/footer.tsx"), "{}", run.out);
}

fn vite_page(tree: &Tree, body: &str) {
    tree.write(
        "index.html",
        &format!("<!doctype html>\n<html>\n  <body>\n{body}\n  </body>\n</html>\n"),
    );
}

#[test]
fn a_new_file_under_pages_nothing_imports_in_a_vite_app_fails() {
    let tree = vite();
    tree.base();
    tree.write(
        "src/pages/about.tsx",
        "export default function About() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/pages/about.tsx"), "{}", run.out);
}

#[test]
fn a_module_script_a_change_adds_makes_its_file_an_entry() {
    let tree = vite();
    tree.base();
    tree.write("src/admin.tsx", "console.log(\"admin\");\n");
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx\"></script>\n    <script src='./src/admin.tsx?v=1' type=module></script>",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("src/admin.tsx"), "{}", run.out);
}

#[test]
fn a_module_script_inside_an_html_comment_makes_no_entry() {
    let tree = vite();
    tree.base();
    tree.write("src/admin.tsx", "console.log(\"admin\");\n");
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx\"></script>\n    <!-- <script type=\"module\" src=\"/src/admin.tsx\"></script> -->",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/admin.tsx"), "{}", run.out);
}

fn a_vite_hole(tree: &Tree, why: &str) {
    tree.write(
        "src/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(
        run.says("app package(s) klin could not prove"),
        "{}",
        run.out
    );
    assert!(run.says(why), "{}", run.out);
    assert!(!run.says("src/footer.tsx"), "{}", run.out);
}

#[test]
fn a_vite_app_whose_index_html_names_no_held_module_is_a_hole() {
    let tree = vite();
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.jsx\"></script>",
    );
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app whose index.html names no TypeScript module the package holds",
    );
}

#[test]
fn a_vite_app_that_holds_a_vue_file_is_a_hole() {
    let tree = vite();
    tree.write("src/Widget.vue", "<template><div /></template>\n");
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app that holds .vue, .svelte or .astro files klin does not read",
    );
}

#[test]
fn a_vite_app_with_an_inline_module_script_is_a_hole() {
    let tree = vite();
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx\"></script>\n    <script type=\"module\">\n      import \"/src/footer.tsx\";\n    </script>",
    );
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app whose index.html holds an inline module script klin does not read",
    );
}

#[test]
fn a_vite_app_whose_module_script_names_a_placeholder_is_a_hole() {
    let tree = vite();
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/%VITE_ENTRY%.tsx\"></script>",
    );
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app whose index.html names a module through a %...% placeholder klin does not expand",
    );
}

#[test]
fn a_package_that_names_next_and_vite_is_judged_as_a_next_app() {
    let tree = app();
    tree.write(
        "package.json",
        r#"{"name":"web","private":true,"dependencies":{"next":"15.0.0","vite":"6.0.0"}}"#,
    );
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/components/footer.tsx\"></script>",
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
fn an_inline_module_script_gives_the_hole_text_before_a_placeholder() {
    let tree = vite();
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/%VITE_ENTRY%.tsx\"></script>\n    <script type=\"module\">\n      import \"/src/footer.tsx\";\n    </script>",
    );
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app whose index.html holds an inline module script klin does not read",
    );
}

#[test]
fn a_percent_encoded_src_is_no_placeholder() {
    let tree = vite();
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx?values=%20%2F\"></script>",
    );
    tree.base();
    tree.write(
        "src/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("could not prove"), "{}", run.out);
    assert!(run.says("src/footer.tsx"), "{}", run.out);
}

#[test]
fn a_vite_app_whose_index_html_names_only_a_test_is_a_hole() {
    let tree = vite();
    tree.write(
        "src/main.test.tsx",
        "import { App } from \"./app\";\n\nApp();\n",
    );
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.test.tsx\"></script>",
    );
    tree.base();

    a_vite_hole(
        &tree,
        "a Vite app whose index.html names no TypeScript module the package holds",
    );
}

#[test]
fn a_module_script_inside_a_template_or_raw_text_makes_no_entry() {
    let tree = vite();
    tree.base();
    tree.write("src/admin.tsx", "console.log(\"admin\");\n");
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx\"></script>\n    <template><script type=\"module\" src=\"/src/admin.tsx\"></script></template>\n    <style>/* <script type=\"module\" src=\"/src/admin.tsx\"></script> */</style>",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/admin.tsx"), "{}", run.out);
}

#[test]
fn script_text_in_a_quoted_attribute_is_no_inline_module_script() {
    let tree = vite();
    vite_page(
        &tree,
        "    <div data-snippet='<script type=\"module\">import \"./x\";</script>'></div>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>",
    );
    tree.base();
    tree.write(
        "src/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("could not prove"), "{}", run.out);
    assert!(run.says("src/footer.tsx"), "{}", run.out);
}

#[test]
fn a_template_tag_in_a_comment_inside_a_template_hides_no_entry() {
    let tree = vite();
    vite_page(
        &tree,
        "    <template>\n      <!-- <template> -->\n    </template>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>",
    );
    tree.base();
    tree.write(
        "src/footer.tsx",
        "export function Footer() {\n  return null;\n}\n",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(!run.says("could not prove"), "{}", run.out);
    assert!(run.says("src/footer.tsx"), "{}", run.out);
}

#[test]
fn a_template_end_tag_in_a_quoted_attribute_keeps_the_template_inert() {
    let tree = vite();
    tree.base();
    tree.write("src/admin.tsx", "console.log(\"admin\");\n");
    vite_page(
        &tree,
        "    <script type=\"module\" src=\"/src/main.tsx\"></script>\n    <template><div title=\"</template>\"></div><script type=\"module\" src=\"/src/admin.tsx\"></script></template>",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/admin.tsx"), "{}", run.out);
}

#[test]
fn an_end_tag_whose_name_only_starts_with_the_raw_text_name_keeps_it_raw() {
    let tree = vite();
    tree.base();
    tree.write("src/admin.tsx", "console.log(\"admin\");\n");
    vite_page(
        &tree,
        "    <style>\n      /* </styles>\n      <script type=\"module\" src=\"/src/admin.tsx\"></script>\n      */\n    </style>\n    <script type=\"module\" src=\"/src/main.tsx\"></script>",
    );

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(run.says("src/admin.tsx"), "{}", run.out);
}

/// A Yarn workspace whose root lists `packages/*` and holds a Next app named `web` under
/// `packages/web`, whose page imports `importing` from its header.
fn workspace(importing: &str) -> Tree {
    let tree = Tree::new();
    tree.write("klin.json", "{}");
    tree.write(
        "package.json",
        r#"{"name":"root","private":true,"workspaces":["packages/*"]}"#,
    );
    tree.write("packages/web/package.json", NEXT);
    tree.write(
        "packages/web/app/layout.tsx",
        "export default function Layout() {\n  return null;\n}\n",
    );
    tree.write(
        "packages/web/app/page.tsx",
        &format!("import {{ Header }} from \"{importing}\";\n\nexport default function Page() {{\n  return Header();\n}}\n"),
    );
    tree
}

fn header(tree: &Tree, path: &str) {
    tree.write(path, "export function Header() {\n  return null;\n}\n");
}

#[test]
fn a_subpath_import_through_the_package_own_workspace_name_reaches_its_file() {
    let tree = workspace("web/components/header");
    header(&tree, "packages/web/components/header.tsx");

    let run = reachability(&tree);

    assert_eq!(run.code, 0, "{}", run.out);
    assert!(!run.says("components/header.tsx"), "{}", run.out);
}

#[test]
fn an_import_of_a_workspace_package_that_writes_exports_is_external_and_the_app_stays_judged() {
    let tree = workspace("ui/header");
    tree.write(
        "packages/ui/package.json",
        r#"{"name":"ui","exports":{"./*":"./src/*.tsx"}}"#,
    );
    header(&tree, "packages/ui/header.tsx");
    header(&tree, "packages/web/components/footer.tsx");

    let run = reachability(&tree);

    assert_eq!(run.code, 1, "{}", run.out);
    assert!(
        run.says("packages/web/components/footer.tsx"),
        "{}",
        run.out
    );
    assert!(!run.says("ui/header"), "{}", run.out);
}

#[test]
fn the_app_hole_note_counts_packages_not_the_aliases_inside_them() {
    let tree = app();
    tree.write(
        "tsconfig.json",
        r#"{"extends":"package/config","compilerOptions":{"paths":{"@/*":["./*"]}}}"#,
    );
    tree.write(
        "app/page.tsx",
        "import { a } from \"@/a\";\nimport { b } from \"@/b\";\nimport { c } from \"@/c\";\n\nexport default function Page() {\n  return [a, b, c];\n}\n",
    );
    for name in ["a", "b", "c"] {
        tree.write(
            &format!("{name}.ts"),
            &format!("export const {name} = 1;\n"),
        );
    }

    let run = reachability(&tree);

    assert!(
        run.says("1 app package(s) klin could not prove"),
        "{}",
        run.out
    );
    assert!(run.says("app/page.tsx:3"), "{}", run.out);
}
