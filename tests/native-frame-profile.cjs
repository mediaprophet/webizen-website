// Read-only browser performance sampling; never shipped as website code.
const {chromium}=require('C:/Users/Hp/AppData/Local/npm-cache/_npx/9833c18b2d85bc59/node_modules/playwright-core');
const fs=require('node:fs');
const label=process.argv[2]||'baked-pinned';const base=process.argv[3]||'http://127.0.0.1:8801';const route=process.argv[4]||'aaa-proof.html';
const out='evidence/aaa-performance';fs.mkdirSync(out,{recursive:true});
(async()=>{const browser=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:['--enable-unsafe-webgpu','--ignore-gpu-blocklist']});try{
 const page=await browser.newPage({viewport:{width:1600,height:1000},reducedMotion:'reduce'});
 const errors=[];page.on('pageerror',e=>errors.push(e.message));
 const start=Date.now();await page.goto(base+'/'+route);
 await page.waitForFunction(()=>window.__webizenNativeWorld||document.getElementById('native-fallback'),null,{timeout:180000});
 const before=await page.evaluate(()=>window.__webizenNativeWorld);if(!before)throw Error('Native boot refused');
 const bootMs=Date.now()-start;console.log(JSON.stringify({stage:'boot',label,bootMs,triangles:before.upload.total_triangles}));
 const session=await page.context().newCDPSession(page);await session.send('Profiler.enable');await session.send('Profiler.setSamplingInterval',{interval:1000});await session.send('Profiler.start');
 const observations=await page.evaluate(async()=>{
  const frames=[];let previous=performance.now();const begun=previous;
  await new Promise(resolve=>{function sample(stamp){frames.push(stamp-previous);previous=stamp;if(stamp-begun>=6000)resolve();else requestAnimationFrame(sample);}requestAnimationFrame(sample);});
  const adapter=await navigator.gpu?.requestAdapter();
  return {elapsed_ms:performance.now()-begun,raf_intervals_ms:frames,world:window.__webizenNativeWorld,adapter_probe:adapter?.info?{vendor:adapter.info.vendor,architecture:adapter.info.architecture,device:adapter.info.device,description:adapter.info.description}:null,adapter_note:'Separate default adapter probe; not a claim that this is the native selected adapter',resources:performance.getEntriesByType('resource').filter(r=>r.name.endsWith('.wasm')||r.name.endsWith('.10d')).map(r=>({url:r.name,duration_ms:r.duration,encoded_bytes:r.encodedBodySize}))};
 });
 const profile=(await session.send('Profiler.stop')).profile;
 fs.writeFileSync(out+'/'+label+'.cpuprofile',JSON.stringify(profile));
 const nodes=new Map(profile.nodes.map(n=>[n.id,n]));const counts=new Map();for(const id of profile.samples||[])counts.set(id,(counts.get(id)||0)+1);
 const hotspots=[...counts].sort((a,b)=>b[1]-a[1]).slice(0,25).map(([id,hits])=>({hits,function:nodes.get(id)?.callFrame.functionName,url:nodes.get(id)?.callFrame.url,line:nodes.get(id)?.callFrame.lineNumber}));
 const sorted=observations.raf_intervals_ms.slice(1).sort((a,b)=>a-b);const quantile=p=>sorted[Math.min(sorted.length-1,Math.ceil(sorted.length*p)-1)];
 const measured=observations.raf_intervals_ms.slice(1);
 await page.goto('about:blank');
 const control=await page.evaluate(async()=>{const stamps=[];await new Promise(resolve=>{function next(t){stamps.push(t);if(stamps.length===30)resolve();else requestAnimationFrame(next);}requestAnimationFrame(next);});return {intervals_ms:stamps.slice(1).map((v,i)=>v-stamps[i]),callbacks_per_second:29000/(stamps.at(-1)-stamps[0])};});
 const result={schema:'webizen.browser-frame-sample/v1',label,route,viewport:{width:1600,height:1000},boot_ms:bootMs,before,observations,raf_median_ms:quantile(.5),raf_p95_ms:quantile(.95),observed_raf_callbacks_per_second:1000*measured.length/measured.reduce((a,b)=>a+b,0),fps_interval:'Consecutive sampled RAF timestamps; excludes first callback and subsequent adapter probe',empty_page_control:control,hotspots,errors};
 fs.writeFileSync(out+'/'+label+'.json',JSON.stringify(result,null,2));console.log(JSON.stringify({label,boot_ms:bootMs,raf_median_ms:result.raf_median_ms,raf_p95_ms:result.raf_p95_ms,raf_callbacks_per_second:result.observed_raf_callbacks_per_second,hotspots:hotspots.slice(0,8),errors}));
 }finally{await browser.close();}})().catch(e=>{console.error(e);process.exitCode=1});
