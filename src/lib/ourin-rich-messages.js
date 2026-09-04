import { proto, generateMessageIDV2 } from "ourin";
import { randomBytes, randomUUID, getRandomValues } from "crypto";
export const JS_KEYWORDS = new Set([
  "import",
  "export",
  "from",
  "default",
  "as",
  "const",
  "let",
  "var",
  "function",
  "class",
  "extends",
  "new",
  "return",
  "if",
  "else",
  "for",
  "while",
  "do",
  "switch",
  "case",
  "break",
  "continue",
  "try",
  "catch",
  "finally",
  "throw",
  "async",
  "await",
  "yield",
  "typeof",
  "instanceof",
  "in",
  "of",
  "delete",
  "void",
  "true",
  "false",
  "null",
  "undefined",
  "NaN",
  "Infinity",
  "this",
  "super",
  "static",
  "get",
  "set",
  "debugger",
  "with",
]);
export const PYTHON_KEYWORDS = new Set([
  "import",
  "from",
  "as",
  "def",
  "class",
  "return",
  "if",
  "elif",
  "else",
  "for",
  "while",
  "break",
  "continue",
  "try",
  "except",
  "finally",
  "raise",
  "with",
  "yield",
  "lambda",
  "pass",
  "del",
  "global",
  "nonlocal",
  "assert",
  "True",
  "False",
  "None",
  "and",
  "or",
  "not",
  "in",
  "is",
  "async",
  "await",
  "self",
  "print",
]);
export const GO_KEYWORDS = new Set([
  "func",
  "package",
  "import",
  "return",
  "if",
  "else",
  "for",
  "switch",
  "case",
  "break",
  "continue",
  "type",
  "struct",
  "interface",
  "map",
  "chan",
  "go",
  "defer",
  "const",
  "var",
  "range",
  "true",
  "false",
  "nil",
  "select",
  "default",
  "fallthrough",
]);
export const LUA_KEYWORDS = new Set([
  "function",
  "end",
  "if",
  "then",
  "else",
  "elseif",
  "for",
  "while",
  "do",
  "local",
  "return",
  "true",
  "false",
  "nil",
  "repeat",
  "until",
  "in",
  "not",
  "and",
  "or",
]);
export const BASH_KEYWORDS = new Set([
  "if",
  "then",
  "else",
  "elif",
  "fi",
  "for",
  "while",
  "do",
  "done",
  "case",
  "esac",
  "echo",
  "export",
  "return",
  "in",
  "function",
  "local",
  "read",
  "set",
  "unset",
  "true",
  "false",
  "exit",
  "source",
  "alias",
  "declare",
  "typeset",
]);
// Port baileys-new: tambahan keyword bahasa untuk tokenisasi code block
export const CPP_KEYWORDS = new Set([
  "alignas", "alignof", "and", "and_eq", "asm", "auto", "bitand", "bitor", "bool", "break", "case",
  "catch", "char", "class", "compl", "concept", "const", "consteval", "constexpr", "constinit",
  "const_cast", "continue", "co_await", "co_return", "co_yield", "decltype", "default", "delete",
  "do", "double", "dynamic_cast", "else", "enum", "explicit", "export", "extern", "false", "float",
  "for", "friend", "goto", "if", "inline", "int", "long", "mutable", "namespace", "new", "noexcept",
  "not", "not_eq", "nullptr", "operator", "or", "or_eq", "private", "protected", "public", "register",
  "reinterpret_cast", "requires", "return", "short", "signed", "sizeof", "static", "static_assert",
  "static_cast", "struct", "switch", "template", "this", "thread_local", "throw", "true", "try",
  "typedef", "typeid", "typename", "union", "unsigned", "using", "virtual", "void", "volatile",
  "wchar_t", "while", "xor", "xor_eq",
]);
export const CSS_KEYWORDS = new Set([
  "import", "media", "font-face", "keyframes", "supports", "charset",
  "important", "root", "hover", "active", "focus", "visited", "before", "after",
  "not", "nth-child", "first-child", "last-child", "only-child",
  "none", "inherit", "initial", "unset", "auto", "transparent", "currentcolor",
]);
export const HTML_KEYWORDS = new Set([
  "html", "head", "body", "title", "meta", "link", "script", "style",
  "header", "footer", "main", "section", "article", "aside", "nav",
  "div", "span", "h1", "h2", "h3", "h4", "h5", "h6", "p", "a", "img",
  "ul", "ol", "li", "table", "tr", "td", "th", "thead", "tbody",
  "form", "input", "button", "select", "textarea", "label", "option",
  "canvas", "svg", "iframe", "video", "audio", "source",
]);
export const RUST_KEYWORDS = new Set([
  "as", "break", "const", "continue", "crate", "else", "enum", "extern",
  "false", "fn", "for", "if", "impl", "in", "let", "loop", "match",
  "mod", "move", "mut", "pub", "ref", "return", "self", "Self",
  "static", "struct", "super", "trait", "true", "type", "unsafe",
  "use", "where", "while", "async", "await", "dyn",
  "abstract", "become", "box", "do", "final", "macro",
  "override", "priv", "typeof", "unsized", "virtual", "yield",
  "try",
]);
export const C_KEYWORDS = new Set([
  "auto", "break", "case", "char", "const", "continue",
  "default", "do", "double", "else", "enum", "extern",
  "float", "for", "goto", "if", "inline", "int",
  "long", "register", "restrict", "return", "short",
  "signed", "sizeof", "static", "struct", "switch",
  "typedef", "union", "unsigned", "void", "volatile",
  "while",
  "_Alignas", "_Alignof", "_Atomic", "_Bool",
  "_Complex", "_Generic", "_Imaginary",
  "_Noreturn", "_Static_assert", "_Thread_local",
]);
export const CSHARP_KEYWORDS = new Set([
  "abstract", "as", "base", "bool", "break", "byte",
  "case", "catch", "char", "checked", "class",
  "const", "continue", "decimal", "default",
  "delegate", "do", "double", "else", "enum",
  "event", "explicit", "extern", "false", "finally",
  "fixed", "float", "for", "foreach", "goto",
  "if", "implicit", "in", "int", "interface",
  "internal", "is", "lock", "long", "namespace",
  "new", "null", "object", "operator", "out",
  "override", "params", "private", "protected",
  "public", "readonly", "ref", "return", "sbyte",
  "sealed", "short", "sizeof", "stackalloc",
  "static", "string", "struct", "switch",
  "this", "throw", "true", "try", "typeof",
  "uint", "ulong", "unchecked", "unsafe",
  "ushort", "using", "virtual", "void",
  "volatile", "while",
  "async", "await", "record", "init",
  "required", "file", "global", "nameof",
  "var", "dynamic", "partial", "yield",
  "from", "where", "select", "group",
  "orderby", "join", "let", "into",
  "equals", "by", "ascending", "descending",
]);
export const CMD_KEYWORDS = new Set([
  "echo", "set", "if", "else",
  "for", "in", "do", "goto",
  "call", "exit", "shift",
  "pause", "start", "title",
  "cls", "rem",
  "dir", "copy", "move", "del",
  "mkdir", "rmdir", "type",
  "ren", "tasklist", "taskkill",
  "ping", "ipconfig", "netstat",
  "shutdown",
]);
export const POWERSHELL_KEYWORDS = new Set([
  "function", "filter", "param",
  "begin", "process", "end",
  "if", "else", "elseif",
  "switch", "foreach", "for",
  "while", "do", "until",
  "break", "continue", "return",
  "throw", "trap", "try",
  "catch", "finally",
  "$true", "$false", "$null",
  "Write-Host", "Write-Output",
  "Get-Item", "Set-Item",
  "Get-ChildItem", "Remove-Item",
  "Copy-Item", "Move-Item",
  "Test-Path", "Invoke-Command",
]);
export const LANGUAGE_KEYWORDS = {
  css: CSS_KEYWORDS,
  html: HTML_KEYWORDS,
  javascript: JS_KEYWORDS,
  typescript: JS_KEYWORDS,
  js: JS_KEYWORDS,
  ts: JS_KEYWORDS,
  python: PYTHON_KEYWORDS,
  py: PYTHON_KEYWORDS,
  go: GO_KEYWORDS,
  golang: GO_KEYWORDS,
  cpp: CPP_KEYWORDS,
  "c++": CPP_KEYWORDS,
  rust: RUST_KEYWORDS,
  rs: RUST_KEYWORDS,
  c: C_KEYWORDS,
  h: C_KEYWORDS,
  csharp: CSHARP_KEYWORDS,
  cs: CSHARP_KEYWORDS,
  bash: BASH_KEYWORDS,
  sh: BASH_KEYWORDS,
  shell: BASH_KEYWORDS,
  cmd: CMD_KEYWORDS,
  bat: CMD_KEYWORDS,
  powershell: POWERSHELL_KEYWORDS,
  ps1: POWERSHELL_KEYWORDS,
};
export var CodeHighlightType;
(function (CodeHighlightType) {
  CodeHighlightType[(CodeHighlightType["DEFAULT"] = 0)] = "DEFAULT";
  CodeHighlightType[(CodeHighlightType["KEYWORD"] = 1)] = "KEYWORD";
  CodeHighlightType[(CodeHighlightType["METHOD"] = 2)] = "METHOD";
  CodeHighlightType[(CodeHighlightType["STRING"] = 3)] = "STRING";
  CodeHighlightType[(CodeHighlightType["NUMBER"] = 4)] = "NUMBER";
  CodeHighlightType[(CodeHighlightType["COMMENT"] = 5)] = "COMMENT";
})(CodeHighlightType || (CodeHighlightType = {}));
export var RichSubMessageType;
(function (RichSubMessageType) {
  RichSubMessageType[(RichSubMessageType["UNKNOWN"] = 0)] = "UNKNOWN";
  RichSubMessageType[(RichSubMessageType["GRID_IMAGE"] = 1)] = "GRID_IMAGE";
  RichSubMessageType[(RichSubMessageType["TEXT"] = 2)] = "TEXT";
  RichSubMessageType[(RichSubMessageType["INLINE_IMAGE"] = 3)] = "INLINE_IMAGE";
  RichSubMessageType[(RichSubMessageType["TABLE"] = 4)] = "TABLE";
  RichSubMessageType[(RichSubMessageType["CODE"] = 5)] = "CODE";
  RichSubMessageType[(RichSubMessageType["DYNAMIC"] = 6)] = "DYNAMIC";
  RichSubMessageType[(RichSubMessageType["MAP"] = 7)] = "MAP";
  RichSubMessageType[(RichSubMessageType["LATEX"] = 8)] = "LATEX";
  RichSubMessageType[(RichSubMessageType["CONTENT_ITEMS"] = 9)] =
    "CONTENT_ITEMS";
})(RichSubMessageType || (RichSubMessageType = {}));
export const tokenizeCode = (codeStr, language = "javascript") => {
  const keywords = LANGUAGE_KEYWORDS[language] || JS_KEYWORDS;
  const blocks = [];
  const lines = codeStr.split("\n");
  for (let li = 0; li < lines.length; li++) {
    const line = lines[li];
    const isLast = li === lines.length - 1;
    const nl = isLast ? "" : "\n";
    if (!line.trim()) {
      blocks.push({
        highlightType: CodeHighlightType.DEFAULT,
        codeContent: line + nl,
      });
      continue;
    }
    if (line.trim().startsWith("//") || line.trim().startsWith("#")) {
      blocks.push({
        highlightType: CodeHighlightType.COMMENT,
        codeContent: line + nl,
      });
      continue;
    }
    const regex =
      /(\/\/.*$|#.*$)|(["'`](?:[^"'`\\]|\\.)*["'`])|(\b\d+(?:\.\d+)?\b)|(\b[a-zA-Z_$][\w$]*\b)|([^\s\w$"'`]+)|(\s+)/g;
    let match;
    const tokens = [];
    while ((match = regex.exec(line)) !== null) {
      const val = match[0];
      if (match[1]) {
        tokens.push({
          highlightType: CodeHighlightType.COMMENT,
          codeContent: val,
        });
      } else if (match[2]) {
        tokens.push({
          highlightType: CodeHighlightType.STRING,
          codeContent: val,
        });
      } else if (match[3]) {
        tokens.push({
          highlightType: CodeHighlightType.NUMBER,
          codeContent: val,
        });
      } else if (match[4]) {
        if (keywords.has(val)) {
          tokens.push({
            highlightType: CodeHighlightType.KEYWORD,
            codeContent: val,
          });
        } else {
          const after = line.slice(regex.lastIndex).trimStart();
          if (after.startsWith("(")) {
            tokens.push({
              highlightType: CodeHighlightType.METHOD,
              codeContent: val,
            });
          } else {
            tokens.push({
              highlightType: CodeHighlightType.DEFAULT,
              codeContent: val,
            });
          }
        }
      } else {
        tokens.push({
          highlightType: CodeHighlightType.DEFAULT,
          codeContent: val,
        });
      }
    }
    if (tokens.length === 0) {
      blocks.push({
        highlightType: CodeHighlightType.DEFAULT,
        codeContent: line + nl,
      });
      continue;
    }
    const merged = [];
    for (const t of tokens) {
      const prev = merged.length > 0 ? merged[merged.length - 1] : undefined;
      if (prev && prev.highlightType === t.highlightType) {
        prev.codeContent += t.codeContent;
      } else {
        merged.push({ ...t });
      }
    }
    if (merged.length > 0) {
      merged[merged.length - 1].codeContent += nl;
    }
    blocks.push(...merged);
  }
  return blocks;
};
export const buildRichContextInfo = (quoted) => {
  const ctxInfo = {
    forwardingScore: 1,
    isForwarded: true,
    forwardedAiBotMessageInfo: { botJid: "867051314767696@bot" },
    forwardOrigin: 4,
  };
  if (quoted?.key) {
    ctxInfo.stanzaId = quoted.key.id;
    ctxInfo.participant =
      quoted.key.participant || quoted.sender || quoted.key.remoteJid;
    ctxInfo.quotedMessage = quoted.message;
  }
  return ctxInfo;
};
export const buildBotForwardedMessage = (
  submessages,
  contextInfo,
  unifiedResponse,
) => {
  const richResponse = {
    messageType: 1,
    submessages,
    contextInfo,
  };
  if (unifiedResponse) {
    richResponse.unifiedResponse = unifiedResponse;
  }
  return {
    botForwardedMessage: {
      message: {
        richResponseMessage: richResponse,
      },
    },
  };
};
export const generateTableContent = (
  title,
  headers,
  rows,
  quoted,
  options = {},
) => {
  const { footer, headerText } = options;
  const tableRows = [
    { items: headers, isHeading: true },
    ...rows.map((row) => ({ items: row.map(String) })),
  ];
  const submessages = [];
  if (headerText) {
    submessages.push({ messageType: 2, messageText: headerText });
  }
  submessages.push({
    messageType: 4,
    tableMetadata: { title, rows: tableRows },
  });
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const generateListContent = (title, items, quoted, options = {}) => {
  const { footer, headerText } = options;
  const tableRows = items.map((item) => ({
    items: Array.isArray(item) ? item.map(String) : [String(item)],
  }));
  const submessages = [];
  if (headerText) {
    submessages.push({ messageType: 2, messageText: headerText });
  }
  submessages.push({
    messageType: 4,
    tableMetadata: { title, rows: tableRows },
  });
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const generateCodeBlockContent = (code, quoted, options = {}) => {
  const { title, footer, language = "javascript" } = options;
  const submessages = [];
  if (title) {
    submessages.push({ messageType: 2, messageText: title });
  }
  submessages.push({
    messageType: 5,
    codeMetadata: {
      codeLanguage: language,
      codeBlocks: tokenizeCode(code, language),
    },
  });
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const generateLatexContent = (quoted, options) => {
  const { text, expressions, headerText, footer } = options;
  const submessages = [];
  if (headerText) {
    submessages.push({ messageType: 2, messageText: headerText });
  }
  const latexExpressions = expressions.map((expr) => {
    const entry = {
      latexExpression: expr.latexExpression,
      url: expr.url,
      width: expr.width,
      height: expr.height,
    };
    if (expr.fontHeight !== undefined) entry.fontHeight = expr.fontHeight;
    if (expr.imageTopPadding !== undefined)
      entry.imageTopPadding = expr.imageTopPadding;
    if (expr.imageLeadingPadding !== undefined)
      entry.imageLeadingPadding = expr.imageLeadingPadding;
    if (expr.imageBottomPadding !== undefined)
      entry.imageBottomPadding = expr.imageBottomPadding;
    if (expr.imageTrailingPadding !== undefined)
      entry.imageTrailingPadding = expr.imageTrailingPadding;
    return entry;
  });
  submessages.push({
    messageType: 8,
    latexMetadata: {
      text: text || "",
      expressions: latexExpressions,
    },
  });
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const generateLatexImageContent = async (
  quoted,
  options,
  uploadFn,
  renderLatexToPng,
) => {
  const { text, expressions, headerText, footer } = options;
  const submessages = [];
  if (headerText) {
    submessages.push({ messageType: 2, messageText: headerText });
  }
  const latexExpressions = await Promise.all(
    expressions.map(async (expr) => {
      const { buffer, width, height } = await renderLatexToPng(
        expr.latexExpression,
      );
      const uploadResult = await uploadFn(buffer, "image");
      const imageUrl = uploadResult.url || uploadResult.directPath;
      return {
        latexExpression: expr.latexExpression,
        url: imageUrl,
        width,
        height,
      };
    }),
  );
  submessages.push({
    messageType: 8,
    latexMetadata: {
      text: text || "",
      expressions: latexExpressions,
    },
  });
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const generateLatexInlineImageContent = async (
  quoted,
  options,
  uploadFn,
  renderLatexToPng,
) => {
  const { text, expressions, headerText, footer } = options;
  const submessages = [];
  if (headerText) {
    submessages.push({ messageType: 2, messageText: headerText });
  }
  if (text) {
    submessages.push({ messageType: 2, messageText: text });
  }
  for (const expr of expressions) {
    const { buffer, width, height } = await renderLatexToPng(
      expr.latexExpression,
    );
    const uploadResult = await uploadFn(buffer, "image");
    const imageUrl = uploadResult.url || uploadResult.directPath;
    submessages.push({
      messageType: 3,
      imageMetadata: {
        imageUrl: {
          imagePreviewUrl: imageUrl,
          imageHighResUrl: imageUrl,
        },
        imageText: expr.latexExpression,
        alignment: 2,
      },
    });
  }
  if (footer) {
    submessages.push({ messageType: 2, messageText: footer });
  }
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
export const captureUnifiedResponse = (msg) => {
  const botFwd = msg?.botForwardedMessage?.message;
  if (!botFwd) return null;
  const rich = botFwd.richResponseMessage;
  if (!rich?.unifiedResponse?.data) return null;
  return {
    unifiedResponse: { data: rich.unifiedResponse.data },
    submessages: rich.submessages || [],
    contextInfo: rich.contextInfo || {},
  };
};
export const generateUnifiedResponseContent = (quoted, captured) => {
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(
      captured.submessages,
      ctxInfo,
      captured.unifiedResponse,
    ),
    messageId: generateMessageIDV2(),
  };
};
export const generateRichMessageContent = (submessages, quoted) => {
  const ctxInfo = buildRichContextInfo(quoted);
  return {
    message: buildBotForwardedMessage(submessages, ctxInfo),
    messageId: generateMessageIDV2(),
  };
};
const HIGHLIGHT_TYPE_MAP = {
  0: "DEFAULT",
  1: "KEYWORD",
  2: "METHOD",
  3: "STR",
  4: "NUMBER",
  5: "COMMENT",
};
export const tokenizeCodeV2 = (code, language = "javascript") => {
  const keywords = LANGUAGE_KEYWORDS[language] || JS_KEYWORDS;
  const tokens = [];
  let i = 0;
  const n = code.length;
  const push = (codeContent, highlightType) => {
    if (!codeContent) return;
    const last = tokens[tokens.length - 1];
    if (last && last.highlightType === highlightType) {
      last.codeContent += codeContent;
    } else {
      tokens.push({ codeContent, highlightType });
    }
  };
  const isWordStart = (c) => /[a-zA-Z_$]/.test(c);
  const isWord = (c) => /[a-zA-Z0-9_$]/.test(c);
  const isNum = (c) => /[0-9]/.test(c);
  while (i < n) {
    const c = code[i];
    if (c === "\n" || c === "\t" || c === " " || /\s/.test(c)) {
      let s = i;
      while (i < n && /\s/.test(code[i])) i++;
      push(code.slice(s, i), 0);
      continue;
    }
    if (c === "/" && code[i + 1] === "/") {
      let s = i;
      i += 2;
      while (i < n && code[i] !== "\n") i++;
      push(code.slice(s, i), 5);
      continue;
    }
    if (c === "/" && code[i + 1] === "*") {
      let s = i;
      i += 2;
      while (i < n - 1 && !(code[i] === "*" && code[i + 1] === "/")) i++;
      i += 2;
      push(code.slice(s, i), 5);
      continue;
    }
    if (
      c === "#" &&
      (language === "python" ||
        language === "py" ||
        language === "bash" ||
        language === "sh" ||
        language === "shell" ||
        language === "lua")
    ) {
      let s = i;
      i++;
      while (i < n && code[i] !== "\n") i++;
      push(code.slice(s, i), 5);
      continue;
    }
    if (c === '"' || c === "'" || c === "`") {
      let s = i;
      const q = c;
      i++;
      while (i < n) {
        if (code[i] === "\\" && i + 1 < n) {
          i += 2;
        } else if (code[i] === q) {
          i++;
          break;
        } else i++;
      }
      push(code.slice(s, i), 3);
      continue;
    }
    if (isNum(c)) {
      let s = i;
      while (i < n && /[0-9.xXa-fA-FeEbBoO_]/.test(code[i])) i++;
      push(code.slice(s, i), 4);
      continue;
    }
    if (isWordStart(c)) {
      let s = i;
      while (i < n && isWord(code[i])) i++;
      const word = code.slice(s, i);
      let type = 0;
      if (keywords.has(word)) {
        type = 1;
      } else {
        let j = i;
        while (j < n && /\s/.test(code[j])) j++;
        if (code[j] === "(") type = 2;
      }
      push(word, type);
      continue;
    }
    push(c, 0);
    i++;
  }
  return {
    codeBlock: tokens,
    unified_codeBlock: tokens.map((t) => ({
      content: t.codeContent,
      type: HIGHLIGHT_TYPE_MAP[t.highlightType] || "DEFAULT",
    })),
  };
};
export const toTableMetadataV2 = (arr) => {
  if (!Array.isArray(arr) || arr.length === 0) {
    throw new Error("Input must be a non-empty array");
  }
  const [title, headerStr, ...rest] = arr;
  const splitCols = (str) => {
    if (typeof str !== "string") return [];
    return str.includes("|")
      ? str.split("|").map((s) => s.trim())
      : str.split(",").map((s) => s.trim());
  };
  const splitRows = (str) => {
    if (typeof str !== "string") return [];
    return str.split(";;").map((row) => splitCols(row));
  };
  const header = splitCols(headerStr);
  const parsedRows = rest.flatMap(splitRows);
  const maxLen = Math.max(header.length, ...parsedRows.map((r) => r.length));
  const unified_rows = [
    {
      is_header: true,
      cells: [...header, ...Array(maxLen - header.length).fill("")],
    },
    ...parsedRows.map((cells) => ({
      is_header: false,
      cells: [...cells, ...Array(maxLen - cells.length).fill("")],
    })),
  ];
  const rows = unified_rows.map((r) => ({
    items: r.cells,
    ...(r.is_header ? { isHeading: true } : {}),
  }));
  return { title, rows, unified_rows };
};
export const generateTableContentV2 = (table, quoted, options = {}) => {
  const { title, footer, headerText, text } = options;
  const { unified_rows } = toTableMetadataV2(table);
  const sections = [];
  if (headerText || title) {
    const headingText = headerText || title;
    sections.push({
      view_model: {
        primitive: {
          text: headingText,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  if (text) {
    sections.push({
      view_model: {
        primitive: {
          text,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  sections.push({
    view_model: {
      primitive: {
        rows: unified_rows,
        __typename: "GenATableUXPrimitive",
      },
      __typename: "GenAISingleLayoutViewModel",
    },
  });
  if (footer) {
    sections.push({
      view_model: {
        primitive: {
          text: footer,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  const responseId = randomUUID();
  const unifiedData = {
    response_id: responseId,
    sections,
  };
  const base64Data = Buffer.from(JSON.stringify(unifiedData)).toString(
    "base64",
  );
  const ctxInfo = {
    forwardingScore: 2,
    isForwarded: true,
    forwardedAiBotMessageInfo: { botJid: "259786046210223@bot" },
    forwardOrigin: 4,
    botMessageSharingInfo: {
      botEntryPointOrigin: 1,
      forwardScore: 2,
    },
  };
  if (quoted?.key) {
    ctxInfo.stanzaId = quoted.key.id;
    ctxInfo.participant =
      quoted.key.participant || quoted.sender || quoted.key.remoteJid;
    ctxInfo.quotedMessage = quoted.message;
  }
  const content = {
    messageContextInfo: {
      threadId: [],
      deviceListMetadata: {
        senderKeyIndexes: [],
        recipientKeyIndexes: [],
        recipientKeyHash: "",
        recipientTimestamp: Math.floor(Date.now() / 1000),
      },
      deviceListMetadataVersion: 2,
      messageSecret: randomBytes(32),
    },
    botForwardedMessage: {
      message: {
        richResponseMessage: {
          submessages: [],
          messageType: 1,
          unifiedResponse: { data: base64Data },
          contextInfo: ctxInfo,
        },
      },
    },
  };
  return {
    message: content,
    messageId: generateMessageIDV2(),
  };
};
export const generateCodeBlockContentV2 = (code, quoted, options = {}) => {
  const { title, footer, language = "javascript", text } = options;
  const { unified_codeBlock } = tokenizeCodeV2(code, language);
  const sections = [];
  if (text) {
    sections.push({
      view_model: {
        primitive: {
          text,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  sections.push({
    view_model: {
      primitive: {
        language,
        code_blocks: unified_codeBlock,
        __typename: "GenAICodeUXPrimitive",
      },
      __typename: "GenAISingleLayoutViewModel",
    },
  });
  if (footer) {
    sections.push({
      view_model: {
        primitive: {
          text: footer,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  const responseId = randomUUID();
  const unifiedData = {
    response_id: responseId,
    sections,
  };
  const base64Data = Buffer.from(JSON.stringify(unifiedData)).toString(
    "base64",
  );
  const ctxInfo = {
    mentionedJid: [],
    groupMentions: [],
    statusAttributions: [],
    forwardingScore: 2,
    isForwarded: true,
    forwardedAiBotMessageInfo: { botJid: "259786046210223@bot" },
    forwardOrigin: 4,
    botMessageSharingInfo: {
      botEntryPointOrigin: 1,
      forwardScore: 2,
    },
  };
  if (quoted?.key) {
    ctxInfo.stanzaId = quoted.key.id;
    ctxInfo.participant =
      quoted.key.participant || quoted.sender || quoted.key.remoteJid;
    ctxInfo.quotedMessage = quoted.message;
  }
  const content = {
    messageContextInfo: {
      threadId: [],
      deviceListMetadata: {
        senderKeyIndexes: [],
        recipientKeyIndexes: [],
        recipientKeyHash: "",
        recipientTimestamp: Math.floor(Date.now() / 1000),
      },
      deviceListMetadataVersion: 2,
      messageSecret: randomBytes(32),
    },
    botForwardedMessage: {
      message: {
        richResponseMessage: {
          submessages: [],
          messageType: 1,
          unifiedResponse: { data: base64Data },
          contextInfo: ctxInfo,
        },
      },
    },
  };
  return {
    message: content,
    messageId: generateMessageIDV2(),
  };
};
export const generateLinkContent = (text, links, quoted, options = {}) => {
  const {
    footer,
    botJid = "867051314767696@bot",
    forwardingScore = 3,
    citations = [],
    proofs = [],
  } = options;
  const submessages = [];
  const fullText = footer ? `${text}${footer}` : text;
  submessages.push({ messageType: 2, messageText: fullText });
  const sections = [];
  const inlineEntities = links.map((link, i) => {
    const url = typeof link === "string" ? link : link.url;
    const displayName =
      typeof link === "object" && link.displayName
        ? link.displayName
        : citations[i]?.sourceTitle || `Link ${i + 1}`;
    return {
      key: `IE_${i}`,
      metadata: {
        display_name: displayName,
        is_trusted: false,
        url,
        __typename: "GenAIInlineLinkItem",
      },
    };
  });
  sections.push({
    view_model: {
      primitive: {
        text,
        inline_entities: inlineEntities,
        __typename: "GenAIMarkdownTextUXPrimitive",
      },
      __typename: "GenAISingleLayoutViewModel",
    },
  });
  if (footer) {
    sections.push({
      view_model: {
        primitive: {
          text: footer,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  const responseId = randomUUID();
  const unifiedData = {
    response_id: responseId,
    sections,
  };
  const base64Data = Buffer.from(JSON.stringify(unifiedData)).toString(
    "base64",
  );
  const ctxInfo = {
    forwardingScore,
    isForwarded: true,
    forwardedAiBotMessageInfo: { botJid },
    forwardOrigin: 4,
    botMessageSharingInfo: {
      forwardScore: forwardingScore,
    },
  };
  if (quoted?.key) {
    ctxInfo.stanzaId = quoted.key.id;
    ctxInfo.participant =
      quoted.key.participant || quoted.sender || quoted.key.remoteJid;
    ctxInfo.quotedMessage = quoted.message;
  }
  const messageContextInfo = {
    messageSecret: randomBytes(32),
  };
  if (citations.length > 0 || proofs.length > 0) {
    const botMetadata = {};
    if (citations.length > 0) {
      botMetadata.richResponseSourcesMetadata = {
        sources: citations.map((c, i) => ({
          provider: 1,
          thumbnailCdnUrl: "",
          sourceProviderUrl:
            typeof links[i] === "string" ? links[i] : links[i]?.url || "",
          sourceQuery: c.sourceQuery || "",
          faviconCdnUrl: c.faviconCdnUrl || "",
          citationNumber: c.citationNumber ?? i + 1,
          sourceTitle: c.sourceTitle || "",
        })),
      };
    }
    if (proofs.length > 0) {
      botMetadata.verificationMetadata = {
        proofs: proofs.map((p) => ({
          version: p.version || 1,
          useCase: p.useCase || 1,
          signature: p.signature || "",
          certificateChain: p.certificateChain || [],
        })),
      };
    }
    messageContextInfo.botMetadata = botMetadata;
  }
  const content = {
    messageContextInfo,
    botForwardedMessage: {
      message: {
        richResponseMessage: {
          messageType: 1,
          submessages,
          unifiedResponse: { data: base64Data },
          contextInfo: ctxInfo,
        },
      },
    },
  };
  return {
    message: content,
    messageId: generateMessageIDV2(),
  };
};
export const generateLinkContentV2 = (text, links, quoted, options = {}) => {
  const { footer, searchEngine = "MAME" } = options;
  const submessages = [];
  const fullText = footer ? `${text}${footer}` : text;
  submessages.push({ messageType: 2, messageText: fullText });
  const sections = [];
  const inlineEntities = links.map((link, i) => {
    const url = typeof link === "string" ? link : link.url;
    const displayName =
      typeof link === "object" && link.displayName
        ? link.displayName
        : `Link ${i + 1}`;
    const sourceDisplayName =
      typeof link === "object" && link.sourceDisplayName
        ? link.sourceDisplayName
        : `Source ${i + 1}`;
    const sourceSubtitle =
      typeof link === "object" && link.sourceSubtitle
        ? link.sourceSubtitle
        : "";
    return {
      key: `IE_${i}`,
      metadata: {
        reference_id: i + 1,
        reference_url: url,
        reference_title: displayName,
        reference_display_name: displayName,
        sources: [
          {
            source_type: "THIRD_PARTY",
            source_display_name: sourceDisplayName,
            source_subtitle: sourceSubtitle,
            source_url: url,
          },
        ],
        __typename: "GenAISearchCitationItem",
      },
    };
  });
  sections.push({
    view_model: {
      primitive: {
        text,
        inline_entities: inlineEntities,
        __typename: "GenAIMarkdownTextUXPrimitive",
      },
      __typename: "GenAISingleLayoutViewModel",
    },
  });
  const searchSources = links.map((link, i) => {
    const url = typeof link === "string" ? link : link.url;
    const sourceDisplayName =
      typeof link === "object" && link.sourceDisplayName
        ? link.sourceDisplayName
        : `Source ${i + 1}`;
    const sourceSubtitle =
      typeof link === "object" && link.sourceSubtitle
        ? link.sourceSubtitle
        : "";
    return {
      source_type: "THIRD_PARTY",
      source_display_name: sourceDisplayName,
      source_subtitle: sourceSubtitle,
      source_url: url,
    };
  });
  sections.push({
    view_model: {
      primitive: {
        sources: searchSources,
        search_engine: searchEngine,
        __typename: "GenAISearchResultPrimitive",
      },
      __typename: "GenAISingleLayoutViewModel",
    },
  });
  if (footer) {
    sections.push({
      view_model: {
        primitive: {
          text: footer,
          __typename: "GenAIMarkdownTextUXPrimitive",
        },
        __typename: "GenAISingleLayoutViewModel",
      },
    });
  }
  const responseId = randomUUID();
  const unifiedData = {
    response_id: responseId,
    sections,
  };
  const base64Data = Buffer.from(JSON.stringify(unifiedData)).toString(
    "base64",
  );
  const ctxInfo = {
    isForwarded: true,
    forwardOrigin: 4,
  };
  if (quoted?.key) {
    ctxInfo.participant =
      quoted.key.participant || quoted.sender || quoted.key.remoteJid;
    ctxInfo.quotedMessage = quoted.message;
  }
  const content = {
    messageContextInfo: {
      threadId: [],
      messageSecret: randomBytes(32),
    },
    botForwardedMessage: {
      message: {
        richResponseMessage: {
          messageType: 1,
          submessages,
          unifiedResponse: { data: base64Data },
          contextInfo: ctxInfo,
        },
      },
    },
  };
  return {
    message: content,
    messageId: generateMessageIDV2(),
  };
};
// Port dari @sairidev/baileys-new (rich-message-utils.js) — diselaraskan dengan builder oktz.
// Shortcut content { code, table, links, richResponse } di sendMessage() → botForwardedMessage.richResponseMessage
export const toUnified = (submessages) => ({
  response_id: randomUUID(),
  sections: submessages.map((submessage) => {
    switch (submessage.messageType) {
      case RichSubMessageType.CODE: {
        const codeMetadata = submessage.codeMetadata;
        return {
          view_model: {
            primitive: {
              language: codeMetadata.codeLanguage,
              code_blocks: codeMetadata.codeBlocks.map((block) => ({
                content: block.codeContent,
                type: CodeHighlightType[block.highlightType],
              })),
              __typename: "GenAICodeUXPrimitive",
            },
            __typename: "GenAISingleLayoutViewModel",
          },
        };
      }
      case RichSubMessageType.TABLE: {
        const tableMetadata = submessage.tableMetadata;
        return {
          view_model: {
            primitive: {
              title: tableMetadata.title,
              rows: tableMetadata.rows.map((row) => ({
                is_header: row.isHeading,
                cells: row.items,
                markdown_cells: row.items.map((item) => ({ text: item })),
              })),
              __typename: "GenATableUXPrimitive",
            },
            __typename: "GenAISingleLayoutViewModel",
          },
        };
      }
      case RichSubMessageType.TEXT:
        return {
          view_model: {
            primitive: {
              text: submessage.messageText,
              inline_entities: submessage.inlineEntities || [],
              __typename: "GenAIMarkdownTextUXPrimitive",
            },
            __typename: "GenAISingleLayoutViewModel",
          },
        };
    }
    return submessage;
  }),
});
export const botMetadataSignature = () => {
  const signature = new Uint8Array(64);
  getRandomValues(signature);
  return signature;
};
export const botMetadataCertificate = (length = 685) => {
  const certificate = new Uint8Array(length);
  certificate[0] = 48;
  certificate[1] = 130;
  getRandomValues(certificate.subarray(2));
  return certificate;
};
export const wrapToBotForwardedMessage = (richResponseMessage) => ({
  messageContextInfo: {
    botMetadata: {
      verificationMetadata: {
        proofs: [
          {
            certificateChain: [
              botMetadataCertificate(),
              botMetadataCertificate(892),
            ],
            version: 1,
            useCase: 1,
            signature: botMetadataSignature(),
          },
        ],
      },
    },
  },
  botForwardedMessage: {
    message: { richResponseMessage },
  },
});
export const prepareRichResponseMessage = (content) => {
  const {
    code,
    contentText,
    disclaimerText,
    footerText,
    headerText,
    language,
    links,
    noHeading,
    richResponse,
    table,
    title,
  } = content;
  let submessages = [];
  if (Array.isArray(richResponse)) {
    submessages = richResponse.map((submessage) => {
      if (submessage.text) {
        return {
          messageType: RichSubMessageType.TEXT,
          messageText: submessage.text,
          inlineEntities: submessage.inlineEntities,
        };
      } else if (submessage.code) {
        return {
          messageType: RichSubMessageType.CODE,
          codeMetadata: {
            codeLanguage: submessage.language,
            codeBlocks: submessage.code,
          },
        };
      } else if (submessage.table) {
        return {
          messageType: RichSubMessageType.TABLE,
          tableMetadata: {
            title: submessage.title,
            rows: submessage.table,
          },
        };
      }
      return submessage;
    });
  } else {
    if (headerText) {
      submessages.push({
        messageType: RichSubMessageType.TEXT,
        messageText: headerText,
      });
    }
    if (contentText) {
      submessages.push({
        messageType: RichSubMessageType.TEXT,
        messageText: contentText,
      });
    }
    if (code) {
      const lang = language || "javascript";
      submessages.push({
        messageType: RichSubMessageType.CODE,
        codeMetadata: {
          codeLanguage: lang,
          codeBlocks: tokenizeCode(code, lang),
        },
      });
    } else if (links) {
      links.forEach((linkField, index) => {
        const prefix = "SS_" + index;
        const url = linkField.url;
        const sources = linkField.sources?.map((sourceField) => ({
          source_type: "THIRD_PARTY",
          source_display_name: sourceField.displayName,
          source_subtitle: sourceField.subtitle,
          source_url: sourceField.url || url,
        }));
        submessages.push({
          messageType: RichSubMessageType.TEXT,
          messageText:
            linkField.text + ` {{${prefix}}}¹{{/${prefix}}} `,
          inlineEntities: [
            {
              key: prefix,
              metadata: {
                reference_id: index + 1,
                reference_url: url,
                reference_title: linkField.title,
                reference_display_name: linkField.displayName,
                sources: sources || [],
                __typename: "GenAISearchCitationItem",
              },
            },
          ],
        });
      });
    } else if (table) {
      submessages.push({
        messageType: RichSubMessageType.TABLE,
        tableMetadata: {
          title,
          rows: table.map((items, index) => ({
            isHeading: !noHeading && index == 0,
            items,
          })),
        },
      });
    }
    if (footerText) {
      submessages.push({
        messageType: RichSubMessageType.TEXT,
        messageText: footerText,
      });
    }
  }
  const unified = toUnified(submessages);
  const richResponseMessage = proto.AIRichResponseMessage.create({
    submessages,
    messageType:
      proto.AIRichResponseMessageType.AI_RICH_RESPONSE_TYPE_STANDARD,
    unifiedResponse: {
      data: Buffer.from(JSON.stringify(unified), "utf-8"),
    },
    contextInfo: {
      isForwarded: true,
      forwardingScore: 1,
      forwardedAiBotMessageInfo: { botJid: "867051314767696@bot" },
      forwardOrigin: 4,
    },
  });
  const message = wrapToBotForwardedMessage(richResponseMessage);
  const botMetadata = message.messageContextInfo.botMetadata;
  if (disclaimerText) {
    botMetadata.messageDisclaimerText = disclaimerText;
  }
  botMetadata.botResponseId = unified.response_id;
  return message;
};
//# sourceMappingURL=rich-messages.js.map
