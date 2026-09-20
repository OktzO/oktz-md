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
    ],
  },
  js.configs.recommended,
  {
    languageOptions: {
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
      },
    },
  },
  {
    ignores: ["tests/"],
  },
];