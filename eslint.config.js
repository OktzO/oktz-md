import js from "@eslint/js";

export default [
  {
    ignores: [
      "node_modules/",
      "storage/",
      "database/",
      "tmp/",
      "temp/",
      "native/",
      "**.min.js",
      "tests/",
    ],
  },
  js.configs.recommended,
  {
    languageOptions: {
      ecmaVersion: "latest",
      sourceType: "module",
      globals: {
        Buffer: "readonly",
        console: "readonly",
        process: "readonly",
        setTimeout: "readonly",
        clearTimeout: "readonly",
        setInterval: "readonly",
        clearInterval: "readonly",
        __dirname: "readonly",
        __filename: "readonly",
        require: "readonly",
        module: "readonly",
        URL: "readonly",
        URLSearchParams: "readonly",
        global: "readonly",
        AbortController: "readonly",
        structuredClone: "readonly",
        fetch: "readonly",
        TextEncoder: "readonly",
        TextDecoder: "readonly",
        performance: "readonly",
        queueMicrotask: "readonly",
        Blob: "readonly",
        FormData: "readonly",
        AbortSignal: "readonly",
      },
    },
  },
  {
    // Konvensi codebase: handler menerima `{ sock }` tapi tak semua memakainya;
    // catch error sering sengaja tak dipakai; catch kosong = swallow API third-party.
    rules: {
      "no-unused-vars": [
        "error",
        {
          args: "after-used",
          argsIgnorePattern: "^(sock|_)$",
          caughtErrorsIgnorePattern: "^(e|err|error)$",
        },
      ],
      "no-empty": ["error", { allowEmptyCatch: true }],
    },
  },
];