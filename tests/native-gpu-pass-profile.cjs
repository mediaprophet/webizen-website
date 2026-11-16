// Test-only Playwright instrumentation; never shipped as website logic.
const {chromium}=require('C:/Users/Hp/AppData/Local/npm-cache/_npx/9833c18b2d85bc59/node_modules/playwright-core');
const fs=require('node:fs');
const path=require('node:path');
const label=process.argv[2]||'instanced-gpu-pass';
if(!/^[a-zA-Z0-9_-]+$/.test(label))throw Error('Invalid evidence label');
const base=process.argv[3]||'http://127.0.0.1:8801';
const route=process.argv[4]||'aaa-instanced-proof.html';
const url=new URL(route,base+'/');url.searchParams.set('gpu-timing','1');
const out=path.resolve(__dirname,'../evidence/aaa-performance');fs.mkdirSync(out,{recursive:true});
const result={schema:'webizen.gpu-pass-profile/v1',label,url:url.href,viewport:{width:1600,height:1000},segments:[],errors:[],console_errors:[],scope:'Native GPU query ranges from AO through postprocessing; excludes earlier uniform/instance queue writes, CPU submission, presentation and readback. RAF is measured independently; GPU time is never inferred from CPU duration.'};
const summary=values=>{const v=[...values].sort((a,b)=>a-b);return {samples:v.length,p50_ms:v[Math.ceil(v.length*.5)-1]??null,p95_ms:v[Math.ceil(v.length*.95)-1]??null,callbacks_per_second:v.length?1000*v.length/v.reduce((a,b)=>a+b,0):null};};
const save=()=>fs.writeFileSync(path.join(out,label+'.json'),JSON.stringify(result,null,2));
const log=(stage,extra={})=>console.log(JSON.stringify({stage,label,...extra}));
(async()=>{
 const browser=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:['--enable-unsafe-webgpu','--ignore-gpu-blocklist']});
 result.browser_version=await browser.version();
 let page;
 try {
  page=await browser.newPage({viewport:result.viewport,deviceScaleFactor:1,reducedMotion:'reduce'});
  page.on('pageerror',e=>result.errors.push(e.message));
  page.on('console',m=>{if(m.type()==='error'||m.type()==='warning')result.console_errors.push({type:m.type(),text:m.text()});});
  const began=Date.now();await page.goto(url.href);
  await page.waitForFunction(()=>window.__webizenNativeWorld||document.getElementById('native-fallback'),null,{timeout:240000});
  result.admission=await page.evaluate(()=>window.__webizenNativeWorld);result.boot_ms=Date.now()-began;
  if(!result.admission)throw Error('Native scene refused admission');
  await page.waitForFunction(()=>window.__webizenNativeWorld?.gpu_pass_timing!==undefined,null,{timeout:45000});
  const first=await page.evaluate(()=>window.__webizenNativeWorld.gpu_pass_timing);
  result.initial_gpu_timing=first;log('boot',{boot_ms:result.boot_ms,adapter:first.selected_adapter,available:first.available});
  if(!first.available||!first.enabled){result.status='gpu-timestamps-unavailable';save();process.exitCode=2;return;}
  // Warmup explicitly excludes load/upload frames and waits for genuine completed query samples.
  const warmupStart=Date.now();
  await page.waitForFunction(()=>{const w=window.__webizenNativeWorld,g=w?.gpu_pass_timing;return g?.completed_frames>=8&&w.frames>=12;},null,{timeout:180000});
  if(Date.now()-warmupStart<10000)await page.waitForTimeout(10000-(Date.now()-warmupStart));
  result.warmup={elapsed_ms:Date.now()-warmupStart,receipt:await page.evaluate(()=>({frames:window.__webizenNativeWorld.frames,camera:window.__webizenNativeWorld.camera,gpu:window.__webizenNativeWorld.gpu_pass_timing}))};
  log('warmup-complete',{completed:result.warmup.receipt.gpu.completed_frames});
  await page.locator('#native-world').focus();
  async function segment(name,moving) {
   const start=await page.evaluate(()=>{
    const w=window.__webizenNativeWorld;
    window.__gpuPassQa={active:true,stamps:[],snapshots:[],lastCompleted:-1};
    function sample(t){const q=window.__gpuPassQa;if(!q.active)return;if(q.stamps.length<4096)q.stamps.push(t);const r=window.__webizenNativeWorld,g=r?.gpu_pass_timing;
     if(g&&g.completed_frames!==q.lastCompleted){q.lastCompleted=g.completed_frames;q.snapshots.push({at:t,frames:r.frames,camera:r.camera,gpu:g,cpu_raf:r.frame_timing});}
     requestAnimationFrame(sample);
    }requestAnimationFrame(sample);
    return {frames:w.frames,camera:w.camera,gpu:w.gpu_pass_timing,wall:performance.now()};
   });
   const cutoff=start.gpu.observed_frame+1, count=start.gpu.completed_frames;
   let final,commands=0;const deadline=Date.now()+180000;
   while(Date.now()<deadline){
    if(moving){await page.keyboard.press(commands%12<6?'ArrowRight':'ArrowLeft');commands++;}
    await page.waitForTimeout(moving?600:1000);
    final=await page.evaluate(()=>({frames:window.__webizenNativeWorld.frames,camera:window.__webizenNativeWorld.camera,gpu:window.__webizenNativeWorld.gpu_pass_timing}));
    // A fresh 16-slot native window ensures no startup/previous-segment samples contaminate quantiles.
    if(final.gpu.completed_frames-count>=16&&final.gpu.sample_count>=8&&final.gpu.oldest_sample_frame>=cutoff)break;
   }
   const observed=await page.evaluate(()=>{window.__gpuPassQa.active=false;return {elapsed_ms:performance.now(),...window.__gpuPassQa};});
   const fresh=final.gpu.completed_frames-count>=16&&final.gpu.sample_count>=8&&final.gpu.oldest_sample_frame>=cutoff;
   const intervals=observed.stamps.slice(1).map((t,i)=>t-observed.stamps[i]);
   const cameras=new Set(observed.snapshots.map(s=>JSON.stringify(s.camera)));
   const record={name,passed:fresh&&(!moving||cameras.size>1),elapsed_ms:observed.elapsed_ms-start.wall,start_frame:start.frames,end_frame:final.frames,query_frame_cutoff:cutoff,completed_query_delta:final.gpu.completed_frames-count,query_window_fully_after_cutoff:final.gpu.oldest_sample_frame>=cutoff,retained_gpu_sample_count:final.gpu.sample_count,gpu_final:final.gpu,raf:summary(intervals),raf_intervals_ms:intervals,receipt_snapshots:observed.snapshots,camera_motion:{requested:moving,real_keyboard_commands:commands,distinct_receipt_cameras:cameras.size,before:start.camera,after:final.camera}};
   result.segments.push(record);save();
   await page.screenshot({path:path.join(out,label+'-'+name+'.png'),animations:'disabled'});
   log(name,{passed:record.passed,completed_queries:record.completed_query_delta,raf:record.raf,gpu_p50_ms:record.gpu_final.p50_ms,gpu_p95_ms:record.gpu_final.p95_ms});
   if(!record.passed)throw Error(name+': insufficient completed native GPU samples or camera movement');
  }
  await segment('stationary',false);
  await segment('camera-motion',true);
  result.final_world=await page.evaluate(()=>window.__webizenNativeWorld);
  result.resources=await page.evaluate(()=>performance.getEntriesByType('resource').filter(r=>/\.(wasm|10d)(\?|$)/.test(r.name)).map(r=>({url:r.name,duration_ms:r.duration,encoded_bytes:r.encodedBodySize})));
  await page.goto('about:blank');
  result.empty_page_control=summary(await page.evaluate(async()=>{const s=[];await new Promise(resolve=>{function f(t){s.push(t);if(s.length>=31)resolve();else requestAnimationFrame(f);}requestAnimationFrame(f);});return s.slice(1).map((t,i)=>t-s[i]);}));
  result.validation_errors=result.console_errors.filter(m=>/GPUValidationError|validation error|uncaptured|device lost/i.test(m.text));
  result.status=(result.errors.length||result.validation_errors.length)?'failed-browser-errors':'completed';save();log('finished',{status:result.status,errors:result.errors,console_errors:result.console_errors});
  if(result.errors.length||result.validation_errors.length)process.exitCode=1;
 }catch(e){result.status='failed';result.failure=String(e.stack||e);if(page&&!page.isClosed()){try{result.failure_world=await page.evaluate(()=>window.__webizenNativeWorld);await page.screenshot({path:path.join(out,label+'-failure.png')});}catch{}}save();throw e;}
 finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1;});

