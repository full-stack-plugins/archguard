const assert = require('node:assert/strict');
const fs = require('node:fs');
const os = require('node:os');
const path = require('node:path');
const cp = require('node:child_process');
const root = path.resolve(__dirname, '../..');
const tool = path.join(root, 'fixtures/languages/typescript/node_modules/typescript');
const payload = {
  files: {'app/main.ts': 'import { Job as Alias } from "@core/job"; export class App { run(x: Alias): Alias { return x; } }', 'core/job.ts':'export class Job { run(x: string): string; run(x: number): number; run(x: string | number): string | number { return x; } }'},
  modules: {'app/main.ts':'app', 'core/job.ts':'core'},
  paths: {'@core/*':['core/*']},
};
const dir=fs.mkdtempSync(path.join(os.tmpdir(),'ag-ts-test-'));
try {
 const privateTool=path.join(dir,'tool');fs.mkdirSync(privateTool);fs.symlinkSync(path.join(tool,'lib'),path.join(privateTool,'lib'));fs.writeFileSync(path.join(privateTool,'libs.json'),JSON.stringify(fs.readdirSync(path.join(tool,'lib')).filter(f=>/^lib\.[a-z0-9.]+\.d\.ts$/.test(f))));
 const input=path.join(dir,'input.json');fs.writeFileSync(input,JSON.stringify(payload));
 const r=cp.spawnSync(process.execPath,['--max-old-space-size=256',path.join(root,'src/analysis/typescript/capture.cjs'),privateTool,input],{encoding:'utf8',timeout:20000,maxBuffer:1024*1024});
 assert.equal(r.status,0,r.stderr);
 const result=JSON.parse(r.stdout);
 assert.equal(result.version,'archguard.typescript-capture/v1');
 assert.equal(result.complete,true);
 assert.ok(result.edges.some(e=>e.from==='app/main.ts'&&e.to==='core/job.ts'));
 assert.equal(result.methods.filter(m=>m.name==='run'&&m.file==='core/job.ts').length,3);
 assert.ok(result.types.some(t=>t.name==='Job'&&t.file==='core/job.ts'));
 assert.ok(result.edges.every(e=>e.line>=1&&e.column>=1));
 console.log('actual compiler alias/overload capture passed');
} finally {fs.rmSync(dir,{recursive:true,force:true});}
