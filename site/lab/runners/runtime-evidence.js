export async function sha256(bytes){return [...new Uint8Array(await crypto.subtle.digest('SHA-256',bytes))].map(x=>x.toString(16).padStart(2,'0')).join('');}
async function fileIdentity(url){const r=await fetch(url);if(!r.ok)throw Error('Cannot identify runtime resource '+url+': '+r.status);const b=await r.arrayBuffer();return {url,bytes:b.byteLength,sha256:await sha256(b)};}
export async function runtimeEvidence(wasm,{includeWasm=true}={}){
 const urls=['/source/tests/test-runner.js','/source/tests/suite-registry.js','/source/benchmark-dataset-loader.js'];
 if(includeWasm)urls.push('/source/playground/qualia_core_db.js','/source/playground/qualia_core_db_bg.wasm');
 let version=null,info=null;try{version=wasm?.get_engine_version?.()??null;info=wasm?.get_engine_info?.()??null;}catch{}
 const sources=[];for(const url of urls){try{sources.push(await fileIdentity(url));}catch(e){sources.push({url,error:e.message});}}
 return {engine:'qualia-core-db',engine_version:version,engine_info:info,runtime:includeWasm?'browser WebAssembly in dedicated worker':'JavaScript reference suites in dedicated worker',native_daemon:false,gpu_used:false,hardware:{architecture:'unknown',npu:'not probed',cpu_model:'unknown'},cross_origin_isolated:self.crossOriginIsolated,wasm_available:typeof WebAssembly==='object',source_identity:sources,identity_scope:'Content hashes captured after execution, outside measured intervals; source-only failures remain visible'};
}

