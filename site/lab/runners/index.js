// Public adapter API. Every run owns a fresh worker: AbortSignal terminates it.
let busy=false;
function invoke(action, options={}) {
 const {signal,onEvent,onProgress,...params}=options;
 if(signal?.aborted)return Promise.reject(new DOMException('Run cancelled','AbortError'));
 if(busy)return Promise.reject(new Error('A Lab run is already active. Cancel or finish it first.'));
 busy=true;
 return new Promise((resolve,reject)=>{
  const worker=new Worker(new URL('./worker.js',import.meta.url),{type:'module',name:'webizen-actual-runner'});
  const cleanup=()=>{worker.terminate();busy=false;signal?.removeEventListener('abort',abort);};
  const abort=()=>{cleanup();reject(new DOMException('Run cancelled; worker terminated','AbortError'));};
  signal?.addEventListener('abort',abort,{once:true});
  worker.onmessage=({data})=>{if(data.type==='event'){try{onEvent?.(data.event);}catch{}return;}if(data.type==='progress'){try{onProgress?.(data.progress);}catch{}return;}cleanup();if(data.type==='result')resolve(data.result);else{const e=new Error(data.message||'Runner failed');e.name=data.name||'Error';reject(e);}};
  worker.onerror=event=>{cleanup();reject(new Error(event.message||'Runner worker could not start'));};
  worker.postMessage({action,options:params});
 });
}
export const runTests=options=>invoke('tests',options);
export const runBenchmark=options=>invoke('benchmark',options);
export const catalogTests=options=>invoke('catalog',options);

