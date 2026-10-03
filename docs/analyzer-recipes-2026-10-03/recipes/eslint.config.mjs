import tseslint from "typescript-eslint";
import sonarjs from "eslint-plugin-sonarjs";

export default [
  { ignores: ["**/node_modules/**", "**/dist/**", "**/build/**", "**/.klin-recipes/**", "**/*.d.ts"] },
  {
    files: ["**/*.ts", "**/*.tsx", "**/*.mts", "**/*.cts"],
    languageOptions: { parser: tseslint.parser, parserOptions: { projectService: true } },
    linterOptions: { reportUnusedDisableDirectives: "off" },
    plugins: { "@typescript-eslint": tseslint.plugin, sonarjs },
    rules: {
      "no-eval": "error",
      "@typescript-eslint/no-implied-eval": "error",
      "no-new-func": "error",
      "no-empty": "error",
      "@typescript-eslint/no-empty-function": "error",
      "sonarjs/no-ignored-exceptions": "error",
      "@typescript-eslint/no-unused-vars": "error",
      "no-unreachable": "error",
      "sonarjs/no-commented-code": "error",
      "no-console": "error",
      "no-debugger": "error",
      "no-alert": "error",
      "@typescript-eslint/no-unnecessary-condition": "error",
    },
  },
];
