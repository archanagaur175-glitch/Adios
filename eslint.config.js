import js from "@eslint/js";
import tseslint from "typescript-eslint";

export default tseslint.config(
  {
    ignores: [
      "dist/**",
      "node_modules/**",
      "src-tauri/**",
      "target/**",
      "packages/**",
      "*.config.ts",
    ],
  },
  js.configs.recommended,
  ...tseslint.configs.recommended,
  {
    rules: {
      // Warn (not error) so a commented-out `any` (allowed by Rule 3) doesn't
      // block the gate; any `any` usage is still surfaced for review.
      "@typescript-eslint/no-explicit-any": "warn",
    },
  },
);
