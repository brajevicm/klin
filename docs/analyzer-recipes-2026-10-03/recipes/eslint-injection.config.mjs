import tseslint from "typescript-eslint";

export default [
  { ignores: ["**/node_modules/**", "**/dist/**", "**/build/**", "**/.klin-recipes/**", "**/*.d.ts"] },
  {
    files: ["**/*.ts", "**/*.tsx", "**/*.mts", "**/*.cts"],
    languageOptions: { parser: tseslint.parser, parserOptions: { projectService: true } },
    linterOptions: { reportUnusedDisableDirectives: "off" },
    plugins: { "@typescript-eslint": tseslint.plugin },
    rules: {
      "no-eval": "error",
      "@typescript-eslint/no-implied-eval": "error",
      "no-new-func": "error",
    },
  },
];
