import typescriptEslint from "@typescript-eslint/eslint-plugin";
import tsParser from "@typescript-eslint/parser";
import react from "eslint-plugin-react";
import js from "@eslint/js";
import globals from "globals";

export default [
  {
    ignores: ["dist", "node_modules"],
  },
  {
    files: ["**/*.{ts,tsx,js,jsx}"],
    languageOptions: {
      parser: tsParser,
      parserOptions: {
        ecmaVersion: 2022,
        sourceType: "module",
        ecmaFeatures: { jsx: true },
        // if you enable type-aware rules, set "project": "./apps/web/tsconfig.json"
      },
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
    plugins: {
      "@typescript-eslint": typescriptEslint,
      react: react,
    },
    rules: {
      ...js.configs.recommended.rules,
      ...typescriptEslint.configs.recommended.rules,
      ...react.configs.recommended.rules,
      "react/react-in-jsx-scope": "off", // Not needed with React 17+ JSX transform
    },
    settings: {
      react: {
        version: "detect",
        runtime: "automatic", // Use automatic JSX runtime
      },
    },
  },
];
