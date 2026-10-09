'use strict';
// Trusted helper: only the fixed compiler module is executable. Candidate text
// enters a closed in-memory CompilerHost and is never required, emitted or run.
const fs=require('node:fs');
const path=require('node:path');
const tool=process.argv[2], input=process.argv[3];
const ts=require(path.join(tool,'lib/typescript.js'));
if(ts.version!=='5.9.3') throw Error('unsupported compiler');
if(fs.statSync(input).size>8*1024*1024) throw Error('input byte budget');
const data=JSON.parse(fs.readFileSync(input,'utf8'));
const keys=Object.keys(data).sort();
if(JSON.stringify(keys)!==JSON.stringify(['files','modules','paths'])) throw Error('unknown input options');
const entries=Object.entries(data.files).sort(([a],[b])=>a.localeCompare(b,'en'));
if(entries.length>64||Object.keys(data.modules).length>64) throw Error('file count budget');
const sources=new Map();let total=0,nodes=0;
const portable=p=>typeof p==='string'&&p.length<=256&&p.split('/').every(s=>/^[A-Za-z0-9_$.-]+$/.test(s)&&s!=='.'&&s!=='..');
for(const [file,text] of entries){
 if(!portable(file)||!file.endsWith('.ts')||typeof text!=='string'||Buffer.byteLength(text)>512*1024)throw Error('input file boundary');
 total+=Buffer.byteLength(text);if(total>4*1024*1024)throw Error('total byte budget');
 sources.set('/src/'+file,text);
}
// This list is written only by the Rust toolchain after hashing the exact
// installed standard libraries; helper cannot discover any additional files.
const whitelist=JSON.parse(fs.readFileSync(path.join(tool,'libs.json'),'utf8'));
for(const file of whitelist){
 if(!/^lib\.[a-z0-9.]+\.d\.ts$/.test(file))throw Error('library path');
 sources.set('/lib/'+file,fs.readFileSync(path.join(tool,'lib',file),'utf8'));
}
const options={target:ts.ScriptTarget.ES2022,module:ts.ModuleKind.ESNext,moduleResolution:ts.ModuleResolutionKind.Bundler,strict:true,noEmit:true,skipLibCheck:false,baseUrl:'/src',paths:data.paths,types:[],allowJs:false,resolveJsonModule:false,allowImportingTsExtensions:true};
const parsed=new Map();
const host={
 getSourceFile(file,version){
  if(!sources.has(file))return undefined;
  if(!parsed.has(file)){
   const sf=ts.createSourceFile(file,sources.get(file),version,true,ts.ScriptKind.TS);
   if(file.startsWith('/src/')){
    const stack=[sf];while(stack.length){const node=stack.pop();if(++nodes>65536)throw Error('AST node budget');ts.forEachChild(node,c=>{stack.push(c);});}
   }
   parsed.set(file,sf);
  }
  return parsed.get(file);
 },
 getDefaultLibFileName:()=>'/lib/lib.es2022.full.d.ts',writeFile(){throw Error('emit forbidden');},
 getCurrentDirectory:()=>'/src',getDirectories:()=>[],getCanonicalFileName:f=>f,
 useCaseSensitiveFileNames:()=>true,getNewLine:()=> '\n',fileExists:f=>sources.has(f),readFile:f=>sources.get(f),
 directoryExists:d=>[...sources.keys()].some(f=>f.startsWith(d+'/')),realpath:f=>f,
};
const required=Object.keys(data.modules).sort();
for(const f of required)if(!portable(f)||!f.endsWith('.ts'))throw Error('required path');
const program=ts.createProgram(required.map(f=>'/src/'+f),options,host);
const checker=program.getTypeChecker();
const out={version:'archguard.typescript-capture/v1',complete:true,files:[],types:[],methods:[],edges:[],unknowns:[]};let records=0;
function emit(kind,value){
 if(++records>4096||JSON.stringify(value).length>8192)throw Error('record budget');
 out[kind].push(value);
}
function location(node){const sf=node.getSourceFile();const a=sf.getLineAndCharacterOfPosition(node.getStart(sf));const b=sf.getLineAndCharacterOfPosition(node.end);return {file:sf.fileName.slice(5),line:a.line+1,column:a.character+1,endLine:b.line+1,endColumn:b.character+1};}
function gap(file,reason,node){out.complete=false;emit('unknowns',{...(node?location(node):{file,line:1,column:1,endLine:1,endColumn:2}),reason});}
for(const file of required){
 if(!sources.has('/src/'+file)){gap(file,'required source missing');continue;}
 emit('files',{file});
}
const diagnostics=ts.getPreEmitDiagnostics(program);
if(diagnostics.length>256)throw Error('diagnostic budget');
for(const d of diagnostics){
 const file=d.file&&d.file.fileName.startsWith('/src/')?d.file.fileName.slice(5):required[0];
 gap(file,'compiler diagnostic '+d.code);
}
const edgeKeys=new Set();
function target(node,symbol){
 if(!symbol){gap(node.getSourceFile().fileName.slice(5),'unresolved semantic reference',node);return;}
 if(symbol.flags&ts.SymbolFlags.Alias)symbol=checker.getAliasedSymbol(symbol);
 const declarations=symbol.declarations||[];
 if(!declarations.length){gap(node.getSourceFile().fileName.slice(5),'reference has no declaration',node);return;}
 for(const declaration of declarations){
  const dest=declaration.getSourceFile().fileName;
  if(dest.startsWith('/lib/'))continue;
  const source=location(node),to=dest.slice(5);
  if(!Object.hasOwn(data.modules,to)){gap(source.file,'target outside protected file scope',node);continue;}
  if(to===source.file)continue;
  const edge={...source,from:source.file,to};const key=JSON.stringify(edge);
  if(!edgeKeys.has(key)){edgeKeys.add(key);emit('edges',edge);}
 }
}
for(const file of required){
 const sf=program.getSourceFile('/src/'+file);if(!sf)continue;
 const stack=[sf];
 while(stack.length){
  const node=stack.pop();
  if(ts.isClassDeclaration(node)||ts.isInterfaceDeclaration(node)||ts.isTypeAliasDeclaration(node)||ts.isEnumDeclaration(node)){
   if(!ts.isSourceFile(node.parent))gap(file,'nested type scope unsupported',node);
   else if(node.name)emit('types',{...location(node),name:node.name.text});else gap(file,'anonymous type identity unsupported',node);
  }
  if(ts.isMethodDeclaration(node)||ts.isMethodSignature(node)||ts.isFunctionDeclaration(node)){
   const topLevel=ts.isSourceFile(node.parent)||((ts.isClassDeclaration(node.parent)||ts.isInterfaceDeclaration(node.parent))&&ts.isSourceFile(node.parent.parent));
   if(!topLevel)gap(file,'nested callable scope unsupported',node);
   else if(node.name&&!ts.isComputedPropertyName(node.name)){
    const signature=checker.getSignatureFromDeclaration(node);
    if(signature)emit('methods',{...location(node),owner:node.parent.name?node.parent.name.getText(sf):'',name:node.name.getText(sf),signature:checker.signatureToString(signature,node,ts.TypeFormatFlags.NoTruncation)});
    else gap(file,'method signature unavailable',node);
   }else gap(file,'computed or anonymous callable unsupported',node);
  }
  // Value-position identifiers also carry static declaration dependencies:
  // ambient variables/functions/enums need not have an import or type node.
  // Shorthand property symbols name the new local property, so ask the checker
  // for its value symbol explicitly rather than guessing from identifier text.
  if(ts.isIdentifier(node)) {
   const symbol=ts.isShorthandPropertyAssignment(node.parent)&&node.parent.name===node
    ?checker.getShorthandAssignmentValueSymbol(node.parent):checker.getSymbolAtLocation(node);
   target(node,symbol);
  }
  if(ts.isImportDeclaration(node)||ts.isExportDeclaration(node)){
   if(node.moduleSpecifier)target(node.moduleSpecifier,checker.getSymbolAtLocation(node.moduleSpecifier));
  }
  if(ts.isTypeReferenceNode(node)||ts.isExpressionWithTypeArguments(node)||ts.isNewExpression(node)||ts.isTypeQueryNode(node))target(node,checker.getSymbolAtLocation(node.typeName||node.expression||node.exprName));
  if(ts.isCallExpression(node)&&node.expression.kind===ts.SyntaxKind.ImportKeyword){
   const arg=node.arguments[0];
   if(arg&&ts.isStringLiteral(arg))target(arg,checker.getSymbolAtLocation(arg));else gap(file,'nonliteral dynamic import unresolved',node);
  }
  if(ts.isImportEqualsDeclaration(node)||ts.isImportTypeNode(node))gap(file,'unsupported import syntax',node);
  if(ts.isCallExpression(node)&&ts.isIdentifier(node.expression)&&['require','eval','Function'].includes(node.expression.text))gap(file,'dynamic execution target unresolved',node);
  ts.forEachChild(node,c=>{stack.push(c);});
 }
}
for(const values of Object.values(out))if(Array.isArray(values))values.sort((a,b)=>JSON.stringify(a)<JSON.stringify(b)?-1:JSON.stringify(a)>JSON.stringify(b)?1:0);
const encoded=JSON.stringify(out);if(Buffer.byteLength(encoded)>1024*1024)throw Error('output byte budget');
process.stdout.write(encoded);
