// Browser verification only; this file is not shipped as application code.
const {chromium}=require('C:/Users/Hp/AppData/Local/npm-cache/_npx/9833c18b2d85bc59/node_modules/playwright-core');
const fs=require('node:fs');
const base='http://127.0.0.1:'+(process.argv.includes('--repository')?'8802':'8801');
const out='evidence/'+(process.argv.includes('--repository')?'aaa-repository-browser':process.argv.includes('--instanced')?'aaa-instanced-browser':process.argv.includes('--full')?'aaa-full-browser':process.argv.includes('--terrain')?'aaa-terrain-browser':process.argv.includes('--lab')?'aaa-lab-browser':'aaa-browser');fs.mkdirSync(out,{recursive:true});
(async()=>{const b=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:['--enable-unsafe-webgpu','--ignore-gpu-blocklist']});try{
 const p=await b.newPage({viewport:{width:1600,height:1000},locale:'en-US',reducedMotion:'reduce'});const errors=[],scripts=[];
 p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(r.resourceType()==='script')scripts.push(r.url());});
 await p.goto(base+'/'+(process.argv.includes('--instanced')?'aaa-instanced-proof.html':process.argv.includes('--full')?'aaa-full-proof.html':process.argv.includes('--terrain')?'terrain-proof.html':process.argv.includes('--lab')?'lab-proof.html#observatory':'aaa-proof.html'));
 await p.waitForFunction(()=>window.__webizenNativeWorld||document.getElementById('native-fallback'),null,{timeout:120000});
 await p.screenshot({path:out+'/world.png'});
 const world=await p.evaluate(()=>({receipt:window.__webizenNativeWorld,upload:window.__webizenNativeUpload,initialisation:window.__webizenNativeInit,status:document.getElementById('native-world-status')?.textContent,lang:document.documentElement.lang}));
 fs.writeFileSync(out+'/admission.json',JSON.stringify({world,errors,scripts},null,2));
 if(!world.receipt)throw Error(world.status);
 const manifestPath=process.argv.includes('--instanced')?'production/aaa-instancing/scene.json':process.argv.includes('--full')?'production/aaa-proof/scene-full.json':process.argv.includes('--terrain')?'data/publications/aaa/terrain-model-v2/scene.json':process.argv.includes('--lab')?'production/native-world/scene.json':'production/aaa-proof/scene.json';
 const manifest=JSON.parse(fs.readFileSync(manifestPath,'utf8'));
 if(world.upload.assets_loaded!==manifest.assets.length||world.upload.assets_refused!==0||world.upload.total_triangles!==manifest.totals.triangles||world.upload.vertex_count!==manifest.totals.vertices||!world.upload.material_pipeline)throw Error('Native scene differs from exact authored manifest');
 if(process.argv.includes('--instanced')){
 const inst=world.receipt.instances;
 if(!inst||inst.prototype_groups!==91||inst.source_instances!==449||inst.ecology_placements!==367||inst.expanded_triangles!==2443920)throw Error('Native instance admission differs from all authored placements');
 }
 console.log(JSON.stringify({stage:'native-admission',assets:world.upload.assets_loaded,triangles:world.upload.total_triangles,material_pipeline:world.upload.material_pipeline}));
 if(await p.locator('#native-panel-toggle').count()){
 await p.locator('#native-panel-toggle').click();
 if(await p.locator('#native-world-ui').isVisible())throw Error('Native panel hiding failed');
 await p.screenshot({path:out+'/world-unobstructed.png'});
 await p.locator('#native-panel-toggle').click();
 }
 if(process.argv.includes('--visual-only')){fs.writeFileSync(out+'/visual-receipt.json',JSON.stringify({world,errors,scripts},null,2));console.log(JSON.stringify({visual_only:true,textures:world.receipt.textures,errors}));return;}
 await p.locator('#native-proof-lab>summary').click();
 try{await p.waitForFunction(()=>document.querySelectorAll('#native-dataset-table tbody tr').length===25&&document.getElementById('native-query-run')?.getAttribute('disabled')===null,null,{timeout:60000});}
 catch(e){await p.screenshot({path:out+'/dataset-loading-failure.png'});fs.writeFileSync(out+'/dataset-loading-failure.json',JSON.stringify(await p.evaluate(()=>({status:document.getElementById('native-lab-status')?.textContent,body:document.body.innerText})),null,2));throw e;}
 await p.locator('#native-query-run').click();
 const query=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.screenshot({path:out+'/graph-query.png'});
 await p.locator('#native-lab-tab-rights').click();await p.locator('#native-rights-run').click();
 const permitted=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.locator('#native-rights-withdraw').check();await p.locator('#native-rights-run').click();
 const withdrawn=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.screenshot({path:out+'/rights-withdrawn.png'});
 await p.locator('#native-lab-tab-space').click();await p.locator('#native-space-run').click();
 const space=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.screenshot({path:out+'/earth-sky.png'});
 await p.locator('#native-lab-tab-checks').click();await p.locator('#native-checks-run').click();
 const checks=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.locator('#native-benchmark-run').click();
 await p.waitForFunction(()=>window.__webizenNativeInstrument?.schema==='webizen.native-query-benchmark/v1',null,{timeout:60000});
 const benchmark=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.locator('#native-lab-results').scrollIntoViewIfNeeded();await p.screenshot({path:out+'/benchmark.png'});
 await p.locator('#native-benchmark-repeats').fill('200');
 // Dispatch the cancellation while the short workload is still pending;
 // Playwright's visual-stability wait can outlast a completed measurement.
 await p.evaluate(()=>{document.getElementById('native-benchmark-run').click();setTimeout(()=>document.getElementById('native-benchmark-cancel').click(),10);});
 await p.waitForFunction(()=>window.__webizenNativeInstrument?.state==='cancelled',null,{timeout:60000});
 const cancelled=await p.evaluate(()=>window.__webizenNativeInstrument);
 if(cancelled.completed_repeats>=200)throw Error('Cancellation did not stop a pending job');
 await p.locator('#native-lab-tab-data').click();
 await p.locator('#native-dataset').selectOption('ecosystem-module-narratives-en-au');
 await p.waitForFunction(()=>document.querySelector('#native-query-preset option[value="all-modules"]')&&document.getElementById('native-query-run')?.getAttribute('disabled')===null,null,{timeout:60000});
 await p.locator('#native-query-preset').selectOption('all-modules');await p.locator('#native-query-run').click();
 const modules=await p.evaluate(()=>window.__webizenNativeInstrument);
 if(modules.match_count!==16)throw Error('Native ecosystem inventory is incomplete');
 await p.locator('#native-query-preset').selectOption('human-purpose');await p.locator('#native-query-run').click();
 const purposes=await p.evaluate(()=>window.__webizenNativeInstrument);
 if(purposes.match_count!==16)throw Error('Missing native human-purpose explanations');
 await p.screenshot({path:out+'/ecosystem-purposes.png'});
 let field=null;
 if(process.argv.includes('--fields')){
 await p.locator('#native-lab-tab-fields').click();await p.locator('#native-field-load').click();
 await p.waitForFunction(()=>window.__webizenNativeInstrument?.schema==='webizen.field-instrument-run/v1',null,{timeout:60000});
 field=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.locator('#native-field-canvas').screenshot({path:out+'/field-gain-one.png'});
 await p.locator('#native-field-gain').fill('4');await p.locator('#native-field-apply').click();
 const adjusted=await p.evaluate(()=>window.__webizenNativeInstrument);
 await p.locator('#native-field-canvas').screenshot({path:out+'/field-gain-four.png'});
 if(fs.readFileSync(out+'/field-gain-one.png').equals(fs.readFileSync(out+'/field-gain-four.png')))throw Error('Field gain changed receipt but not rendered pixels');
 const downloading=p.waitForEvent('download');await p.locator('#native-lab-export').click();
 const download=await downloading;await download.saveAs(out+'/field-export.json');
 const exported=JSON.parse(fs.readFileSync(out+'/field-export.json','utf8'));
 if(exported.derived_cells.length!==13824||exported.source_sha256!==adjusted.source_sha256)throw Error('Field export differs from admitted source');
 await p.locator('#native-field-next').click();
 if(!/25 to 48/.test(await p.locator('#native-field-summary').innerText()))throw Error('Field paging failed');
 await p.locator('#native-field-toggle').click();if(await p.locator('#native-field-canvas').isVisible())throw Error('Field hiding failed');await p.locator('#native-field-toggle').click();
 if(field.derived_cells.length!==13824||JSON.stringify(field.derived_cells)!==JSON.stringify(adjusted.derived_cells))throw Error('Field data changed with diagnostic gain');
 fs.writeFileSync(out+'/field.json',JSON.stringify({initial:field,adjusted},null,2));
 field={cells:field.derived_cells.length,source_sha256:field.source_sha256,draw:field.draw,adjusted_draw:adjusted.draw};
 }
 const crypto=require('node:crypto');
 fs.writeFileSync(out+'/instruments.json',JSON.stringify({world,query,permitted,withdrawn,space,checks,benchmark,cancelled,modules,purposes,field,errors,scripts},null,2));
 const identity=JSON.parse(fs.readFileSync('site/pkg/native-world/runtime-identity.json','utf8'));
 const served=await p.request.get(base+'/pkg/native-world/webizen_world_bg.wasm');
 const servedSha=crypto.createHash('sha256').update(await served.body()).digest('hex');
 if(servedSha!==identity.wasm.sha256)throw Error('Served native runtime identity mismatch');
 await p.locator('#native-proof-lab>summary').click();
 const routes={};for(const purpose of ['learn','research','create','participate']){
 await p.locator('#native-purpose').selectOption(purpose);
 try{await p.waitForFunction(x=>window.__webizenNativeWorld?.purpose===x,purpose);}
 catch(e){fs.writeFileSync(out+'/interaction-failure.json',JSON.stringify(await p.evaluate(()=>({world:window.__webizenNativeWorld,body:document.body.innerText})),null,2));throw e;}
 routes[purpose]=await p.evaluate(()=>window.__webizenNativeWorld.path);
 if(new Set(routes[purpose]).size!==7)throw Error('Observer purpose removed a place');
 }
 if(new Set(Object.values(routes).map(JSON.stringify)).size!==4)throw Error('Observer paths do not vary');
 const before=await p.evaluate(()=>window.__webizenNativeWorld.camera.yaw);
 await p.locator('#native-world').focus();await p.keyboard.press('ArrowRight');
 await p.waitForFunction(v=>window.__webizenNativeWorld.camera.yaw!==v,before);
 await p.setViewportSize({width:390,height:844});await p.screenshot({path:out+'/mobile-world.png'});
 await p.locator('#native-proof-lab>summary').click();await p.locator('#native-lab-tab-data').click();
 await p.screenshot({path:out+'/mobile-data.png'});
 const overflow=await p.evaluate(()=>document.documentElement.scrollWidth>innerWidth);
 if(overflow)throw Error('Mobile horizontal overflow');
 const result={world,query,permitted,withdrawn,space,checks,benchmark,cancelled,modules,purposes,field,servedSha,routes,mobile:{width:390,horizontal_overflow:overflow},errors,scripts};
 fs.writeFileSync(out+'/results.json',JSON.stringify(result,null,2));
 console.log(JSON.stringify({language:world.lang,query_matches:query.match_count,permitted:permitted.permitted,withdrawn:withdrawn.permitted,checks:checks.assertions,benchmark_median_ms:benchmark.median_ms,benchmark_repeats:benchmark.completed_repeats,cancelled:cancelled.state,field,errors,scripts}));
 if(world.lang!=='en-AU'||query.match_count!==8||permitted.permitted!==true||withdrawn.permitted!==false||benchmark.completed_repeats!==30||benchmark.warmup_runs!==10||!checks.cases.every(c=>c.passed)||cancelled.state!=='cancelled'||errors.length||scripts.some(s=>!s.endsWith('/pkg/native-world/webizen_world.js')))process.exitCode=1;
 }finally{await b.close();}})().catch(e=>{console.error(e);process.exitCode=1});
