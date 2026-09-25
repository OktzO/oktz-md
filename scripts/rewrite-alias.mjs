import fs from "node:fs";
import path from "node:path";
import { parse } from "espree";

const targets = process.argv.slice(2);
const extensions = new Set([".js", ".mjs", ".cjs"]);

if (targets.length === 0) throw new Error("No targets provided");

function collectFiles(target) {
  const stat = fs.statSync(target);
  if (stat.isFile()) return extensions.has(path.extname(target)) ? [target] : [];
  if (!stat.isDirectory()) return [];
  return fs
    .readdirSync(target, { withFileTypes: true })
    .sort((a, b) => a.name.localeCompare(b.name))
    .flatMap((entry) => collectFiles(path.join(target, entry.name)));
}

function sourceValue(node) {
  if (node?.type === "Literal" && typeof node.value === "string") return node.value;
  if (
    node?.type === "TemplateLiteral" &&
    node.expressions.length === 0 &&
    node.quasis.length === 1
  ) {
    return node.quasis[0].value.cooked;
  }
  return null;
}

function collectOurinRanges(node, ranges = []) {
  if (!node || typeof node !== "object") return ranges;
  if (
    (node.type === "ImportDeclaration" ||
      node.type === "ExportNamedDeclaration" ||
      node.type === "ExportAllDeclaration" ||
      node.type === "ImportExpression") &&
    sourceValue(node.source) === "ourin"
  ) {
    ranges.push(node.source.range);
  }
  for (const value of Object.values(node)) {
    if (Array.isArray(value)) {
      for (const child of value) collectOurinRanges(child, ranges);
    } else {
      collectOurinRanges(value, ranges);
    }
  }
  return ranges;
}

const files = [...new Set(targets.flatMap(collectFiles))];
let rewrittenFiles = 0;
for (const file of files) {
  const source = fs.readFileSync(file, "utf8");
  const ast = parse(source, {
    ecmaVersion: "latest",
    sourceType: "module",
    range: true,
  });
  const ranges = collectOurinRanges(ast).sort((a, b) => b[0] - a[0]);
  if (ranges.length === 0) continue;
  let rewritten = source;
  for (const [start, end] of ranges) {
    const literal = source.slice(start, end);
    const quote = literal[0];
    rewritten = `${rewritten.slice(0, start)}${quote}onigis${quote}${rewritten.slice(end)}`;
  }
  fs.writeFileSync(file, rewritten);
  console.log(`rewrote ${file}`);
  rewrittenFiles += 1;
}
console.log(`rewrote ${rewrittenFiles} files`);
