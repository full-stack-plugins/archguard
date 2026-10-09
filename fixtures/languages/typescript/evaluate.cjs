// Dependency evaluation only: this is not a production fact provider.
const assert = require('node:assert/strict');
const path = require('node:path');
const ts = require('typescript');
assert.equal(ts.version, '5.9.3');
const configPath = path.join(__dirname, 'tsconfig.json');
const config = ts.readConfigFile(configPath, ts.sys.readFile);
assert.equal(config.error, undefined);
const parsed = ts.parseJsonConfigFileContent(config.config, ts.sys, __dirname);
assert.deepEqual(parsed.errors, []);
const program = ts.createProgram(parsed.fileNames, parsed.options);
assert.deepEqual(ts.getPreEmitDiagnostics(program), []);
const checker = program.getTypeChecker();
const app = program.getSourceFile(path.join(__dirname, 'app/main.ts'));
const core = program.getSourceFile(path.join(__dirname, 'core/job.ts'));
let importedDeclaration;
const overloads = [];
const unknownDynamicImports = [];
function visit(node) {
  if (ts.isImportSpecifier(node)) {
    const alias = checker.getSymbolAtLocation(node.name);
    const symbol = checker.getAliasedSymbol(alias);
    importedDeclaration = symbol.declarations[0].getSourceFile().fileName;
  }
  if (ts.isMethodDeclaration(node) && node.body) {
    for (const signature of checker.getTypeAtLocation(node).getCallSignatures()) {
      overloads.push(checker.signatureToString(signature));
    }
  }
  if (ts.isCallExpression(node) && node.expression.kind === ts.SyntaxKind.ImportKeyword
      && !ts.isStringLiteral(node.arguments[0])) {
    const start = node.getSourceFile().getLineAndCharacterOfPosition(node.getStart());
    unknownDynamicImports.push({ file: path.relative(__dirname, node.getSourceFile().fileName), line: start.line + 1,
      reason: 'nonliteral import target; no deterministic dependency edge' });
  }
  ts.forEachChild(node, visit);
}
visit(app); visit(core);
assert.equal(importedDeclaration, core.fileName);
assert.deepEqual(overloads.sort(), ['(input: number): number', '(input: string): string']);
assert.equal(unknownDynamicImports.length, 1);
assert.equal(unknownDynamicImports[0].line, 3);
console.log(JSON.stringify({ evaluationOnly: true, compiler: ts.version,
  aliasTarget: path.relative(__dirname, importedDeclaration), overloads, unknownDynamicImports,
  productionCoverage: 'unsupported; no adapter published' }, null, 2));
