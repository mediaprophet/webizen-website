import {TestRunner} from '/source/tests/test-runner.js';
import {registerSuites} from '/source/tests/suite-registry.js';
import {loadWasm,getWasmCoverage} from '/source/tests/wasm-loader.js';
import {NativeClient} from '/source/tests/native-client.js';
import {runGraphBenchmark} from './graph-benchmark.js';
import {runtimeEvidence} from './runtime-evidence.js';

const modes=new Set(['logic','wasm','native','both']);
function makeRunner(mode,ctx){if(!modes.has(mode))throw Error('Unknown test mode');const runner=new TestRunner();registerSuites(runner,{mode,wasm:null,native:null,isMobile:false,...ctx},mode);return runner;}
function catalogue(runner){const suites=[];const walk=(suite,path=[])=>{const full=[...path,suite.name];const tests=suite.tests.filter(x=>!x._isSuite).map(x=>x.name);suites.push({name:suite.name,path:full.join(' › '),tests,test_count:tests.length});for(const child of suite.tests.filter(x=>x._isSuite))walk(child.suite,full);};for(const s of runner._suites)walk(s);return {suites,test_count:suites.reduce((n,s)=>n+s.test_count,0),suite_count:suites.length};}
async function tests(options){
 const mode=options.mode||'wasm',ctx={};let nativeState='not-requested';let coverage=null;
 if(!modes.has(mode))throw Error('Unknown test mode');
 if(mode==='wasm'||mode==='both'){ctx.wasm=await loadWasm();coverage=await getWasmCoverage();}
 if(mode==='native'||mode==='both'){
  // Native requests only occur after an explicitly selected native/both run.
  const client=new NativeClient('http://127.0.0.1:4242',options.nativeToken||'');
  try{const health=await client.health(1500);if(health.ok){ctx.native=client;nativeState='reachable';}else nativeState='health-http-'+health.status;}catch{nativeState='unreachable';}
 }
 const runner=makeRunner(mode,ctx),events=[];const started=new Date().toISOString(),t0=performance.now();
 const counts=await runner.run(e=>{const clean={type:e.type,name:e.name,suite:e.suite?.name,reason:e.reason,error:e.error?.message,ms:e.ms,passed:e.passed,failed:e.failed,skipped:e.skipped};events.push(clean);postMessage({type:'event',event:clean});});
 const duration_ms=performance.now()-t0;
 return {schema:'webizen.actual-test-run/v1',mode,...counts,events,started_at:started,duration_ms,catalogue:catalogue(runner),coverage,native_state:nativeState,environment:{...await runtimeEvidence(ctx.wasm,{includeWasm:!!ctx.wasm}),native_daemon:!!ctx.native,native_state:nativeState},execution:'Original TestRunner and registerSuites, in isolated Web Worker',qualification:mode==='logic'?'Original JavaScript reference suites; not a native-engine result':'Original suites retain their own assertions and skips; inspect per-test outcomes',zero_assertion_policy:'skipped by original runner'};
}
self.onmessage=async({data})=>{try{let result;if(data.action==='tests')result=await tests(data.options);else if(data.action==='catalog')result=catalogue(makeRunner(data.options.mode||'both',{}));else if(data.action==='benchmark')result=await runGraphBenchmark(data.options,p=>postMessage({type:'progress',progress:p}));else throw Error('Unknown runner action');postMessage({type:'result',result});}catch(e){postMessage({type:'error',name:e.name,message:e.message});}};

