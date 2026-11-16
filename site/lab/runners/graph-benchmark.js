import {loadWasm} from '/source/tests/wasm-loader.js';
import {loadDataset,queriesForManifest,qHash} from '/source/benchmark-dataset-loader.js';
import {runtimeEvidence,sha256} from './runtime-evidence.js';

// Source-backed compatibility repair for the original synthetic IRI-only loader.
// Rust q_hash (qualia-core-db/src/lib.rs) reserves the top four bits and returns
// FNV-1a & 0x0fffffffffffffff. The legacy browser loader masks only Object.
// Keep the original dataset generator and textual queries; normalise these IRI
// records before querying, and publish both input hashes. Never apply to typed
// literals or modality-tagged records: this adapter accepts synthetic IRIs only.
export function canonicalSyntheticIris(data){
 const mask=0x0fffffffffffffffn,view=new DataView(data.db.buffer,data.db.byteOffset,data.db.byteLength);let changed=0;
 for(let i=0;i<data.triples.length;i++){const t=data.triples[i],off=i*48;
  if(![t.subject,t.predicate,t.object].every(x=>typeof x==='string'&&x.startsWith('http://q.test/')))throw Error('Canonicalisation only accepts original synthetic IRI records');
  const s=t.subjectHash&mask,p=t.predicateHash&mask,o=t.objectHash&mask;
  if(s!==t.subjectHash||p!==t.predicateHash||o!==t.objectHash)changed++;
  t.subjectHash=s;t.predicateHash=p;t.objectHash=o;
  view.setBigUint64(off,s,true);view.setBigUint64(off+8,p,true);view.setBigUint64(off+16,o,true);view.setBigUint64(off+40,s^p^o,true);
 }
 return {operation:'IRI identity normalisation to canonical q_hash 60-bit space',changed_records:changed,source_contract:'qualia-core-db/src/lib.rs::q_hash',legacy_issue:'Original JS dataset loader retained type/modality bits in subject and predicate',scope:'synthetic IRI-only dataset; not a general Quin or literal rewriting rule'};
}

const percentile=(samples,p)=>{const a=[...samples].sort((x,y)=>x-y);const index=(a.length-1)*p,lo=Math.floor(index),hi=Math.ceil(index);return a[lo]+(a[hi]-a[lo])*(index-lo);};
function stats(id,label,samples){const mean=samples.reduce((n,x)=>n+x,0)/samples.length;return {id,label,unit:'ms',mean,p50:percentile(samples,.5),p95:percentile(samples,.95),min:Math.min(...samples),max:Math.max(...samples),sample_standard_deviation:Math.sqrt(samples.reduce((n,x)=>n+(x-mean)**2,0)/(samples.length-1)),samples};}
const key=m=>[m.s,m.p,m.o].map(x=>BigInt(x).toString()).join('/');
function expected(triples,subject,predicate){return triples.filter(t=>(!subject||t.subject===subject)&&(!predicate||t.predicate===predicate)).map(t=>[t.subjectHash,t.predicateHash,t.objectHash].map(x=>x.toString()).join('/')).sort();}
function verify(actual,reference,label){if(actual.error)throw Error(label+': '+actual.error);if(!Array.isArray(actual.matches))throw Error(label+': missing native matches array');const matches=actual.matches.map(key).sort();if(matches.length!==reference.length||matches.some((x,i)=>x!==reference[i]))throw Error(label+': correctness mismatch ('+matches.length+' actual / '+reference.length+' expected)');return {id:label,passed:true,match_count:matches.length};}

export async function runGraphBenchmark(options={},onProgress=()=>{}){
 const size=Number(options.size??10000);if(!Number.isSafeInteger(size)||size<100||size>50000)throw Error('Dataset size must be an integer from 100 to 50000');
 const started_at=new Date().toISOString();onProgress({phase:'loading',message:'Loading the actual QualiaDB WASM'});
 const loadStart=performance.now(),wasm=await loadWasm(),wasm_load_ms=performance.now()-loadStart;if(typeof wasm.execute_ntriples_query!=='function')throw Error('This WASM build does not expose execute_ntriples_query; no benchmark substituted');
 const response=await fetch('/source/benchmark-datasets/synthetic-10k.json');if(!response.ok)throw Error('Original dataset manifest unavailable');const manifest=await response.json();manifest.synthetic_n=size;manifest.n_triples=size;manifest._manifestUrl=response.url;
 onProgress({phase:'dataset',message:'Generating original synthetic N-Triples and canonical IRI Quin inputs'});const data=await loadDataset(manifest,'synthetic');const original_dataset_hash=await sha256(data.db),normalisation=canonicalSyntheticIris(data);const dataset_hash=await sha256(data.db),queries=queriesForManifest(manifest,'synthetic');
 const query=q=>{const raw=wasm.execute_ntriples_query(q,data.db,size);const result=JSON.parse(raw);if(result.error)throw Error(result.error);return result;};
 const references={point:expected(data.triples,queries.pointSubject,null),filter:expected(data.triples,null,queries.filterPredicate),first:expected(data.triples,queries.twohopStart,null),second:expected(data.triples,queries.twohopSecond,null)};
 const operations=[{id:'point',label:'Point query',run:()=>query(queries.point),check:r=>verify(r,references.point,'point')},{id:'filter',label:'Predicate filter',run:()=>query(queries.filter),check:r=>verify(r,references.filter,'filter')},{id:'twohop',label:'Two sequential linked point queries',run:()=>[query(queries.twohop1),query(queries.twohop2)],check:r=>({id:'twohop',passed:verify(r[0],references.first,'twohop-first').passed&&verify(r[1],references.second,'twohop-second').passed,match_count:r[0].matches.length+r[1].matches.length})}];
 // Check the full dataset and every operation before collecting timed samples.
 const correctness=[verify(query('?s ?p ?o .'),expected(data.triples,null,null),'full-dataset'),...operations.map(op=>op.check(op.run()))];
 const warmups=10,sampleCount=30,metrics=[];
 for(const op of operations){onProgress({phase:'warmup',operation:op.id,count:warmups});for(let i=0;i<warmups;i++)op.check(op.run());const samples=[];for(let i=0;i<sampleCount;i++){const t0=performance.now();const result=op.run();const elapsed=performance.now()-t0;op.check(result);samples.push(elapsed);onProgress({phase:'sample',operation:op.id,completed:i+1,total:sampleCount});}metrics.push(stats(op.id,op.label,samples));}
 const environment=await runtimeEvidence(wasm);
 return {schema:'webizen.actual-benchmark-run/v1',started_at,workload:'synthetic-ntriples-v1/browser-canonical-quin-query-v1',metrics,correctness:{passed:correctness.every(x=>x.passed),checks:correctness,policy:'Full dataset and each operation checked before timing; every timed sample validated after its clock stops'},environment,dataset:{source_manifest:'/source/benchmark-datasets/synthetic-10k.json',generator:'/source/benchmark-dataset-loader.js',triples:size,flat_quin_bytes:data.db.byteLength,sha256:dataset_hash,original_input_sha256:original_dataset_hash,normalisation,format:data.format},method:{warmups_per_operation:warmups,samples_per_operation:sampleCount,clock:'performance.now',measurement:'Synchronous execute_ntriples_query plus JSON decoding; verification, dataset preparation, resource hashes and scene rendering excluded from timed intervals',twohop:'Two sequential point queries using the actual first-edge destination encoded by the original synthetic generator; not a single SPARQL join',execution:'Dedicated worker; caller must suspend scenic animation for comparative measurement',series_compatibility:'Separate corrected browser series; not comparable with legacy full-64-bit input, daemon/CPU/GPU/historical proxy paths'},preparation:{wasm_load_ms,dataset_load_ms:data.loadMs},failures:[],exclusions:[],source:'actual browser WASM execution; no mock values'};
}
