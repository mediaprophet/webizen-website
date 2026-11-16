// Test-only browser automation; no part of this file is shipped as application code.
const {chromium}=require('C:/Users/Hp/AppData/Local/npm-cache/_npx/9833c18b2d85bc59/node_modules/playwright-core');
const fs=require('node:fs');
const path=require('node:path');
const crypto=require('node:crypto');
const assert=require('node:assert/strict');
const base=process.env.WEBIZEN_TEST_ORIGIN||'http://127.0.0.1:8802';
const entry=process.env.WEBIZEN_TEST_PAGE||'/aaa-instanced-proof.html';
const out=path.resolve('evidence/aaa-production-browser');fs.mkdirSync(out,{recursive:true});
const sha=b=>crypto.createHash('sha256').update(b).digest('hex');
const safe=s=>s.replace(/[^a-z0-9._-]/gi,'_');
const writes=[];
async function bounded(promise){let timer;try{return await Promise.race([promise,new Promise((_,reject)=>{timer=setTimeout(()=>reject(Error("Upstream production response timed out")),120000);})]);}finally{clearTimeout(timer);}}
async function verifyResponse(page,response,id){
  const request=response.request().postDataJSON();
  assert.equal(request.profile,'files');assert.equal(request.include_html,false);
  assert.equal(request.include_resources,true);assert.equal(request.include_package,true);
  const body=await response.json();assert.equal(body.ok,true,JSON.stringify(body));
  assert.equal(body.strict_replay_verified,true);assert.equal(body.resource_count,body.resources.length);
  assert.equal(Object.hasOwn(body,'cards_html'),false);
  const sourceSha=sha(Buffer.from(request.source));
  await page.waitForFunction(s=>window.__webizenNativeInstrument?.schema==='webizen.production-run/v1'&&window.__webizenNativeInstrument?.source_sha256===s,sourceSha,{timeout:120000});
  const receipt=await page.evaluate(()=>window.__webizenNativeInstrument);
  assert.equal(receipt.package_sha256,body.package_sha256);assert.equal(receipt.browser_verified_all_digests,true);
  const links=page.locator('#native-production-output a[download]');
  for(let i=0;i<body.resources.length;i++){
    const resource=body.resources[i],bytes=Buffer.from(resource.base64,'base64');
    assert.equal(bytes.length,resource.byte_length);assert.equal(sha(bytes),resource.sha256);
    const link=links.nth(i),href=await link.getAttribute('href');
    assert.equal(sha(Buffer.from(href.split(',')[1],'base64')),resource.sha256);
    const downloading=page.waitForEvent('download');await link.click();
    const downloaded=await downloading, destination=path.join(out,`${id}-${safe(resource.key)}`);
    await downloaded.saveAs(destination);assert.equal(sha(fs.readFileSync(destination)),resource.sha256);
  }
  const packageBytes=Buffer.from(body.package_base64,'base64');
  assert.equal(packageBytes.length,body.package_byte_length);assert.equal(sha(packageBytes),body.package_sha256);
  const downloading=page.waitForEvent('download');await page.locator('#native-production-package').click();
  const downloaded=await downloading,destination=path.join(out,`${id}.hmc`);
  await downloaded.saveAs(destination);assert.equal(sha(fs.readFileSync(destination)),body.package_sha256);
  await page.waitForFunction(()=>[...document.querySelectorAll('#native-production-output img')].every(i=>i.complete&&i.naturalWidth>0));
  assert.equal(await page.locator('#native-production-output audio[autoplay]').count(),0);
  assert.equal(await page.locator('#native-production-output audio').evaluateAll(a=>a.every(e=>e.paused&&e.currentTime===0)),true);
  if(id==='tone-default'){const audio=page.locator('#native-production-output audio');await audio.click({position:{x:24,y:27}});await page.waitForFunction(()=>document.querySelector('#native-production-output audio').currentTime>0);await audio.evaluate(a=>a.pause());}
  await page.screenshot({path:path.join(out,`${id}.png`)});
  const result={id,receipt,request_source_sha256:sourceSha,package_bytes:packageBytes.length,resources:body.resources.map(({base64,...r})=>r)};
  writes.push(result);fs.writeFileSync(path.join(out,'production-runs.json'),JSON.stringify(writes,null,2));
  return result;
}
(async()=>{
  const browser=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:['--enable-unsafe-webgpu','--ignore-gpu-blocklist']});
  let page;
  try{
    page=await browser.newPage({viewport:{width:1600,height:1000},locale:'en-AU',reducedMotion:'reduce'});page.setDefaultTimeout(120000);
    const errors=[],scripts=[];page.on('pageerror',e=>errors.push(e.message));page.on('request',r=>{if(r.resourceType()==='script')scripts.push(r.url());});
    await page.goto(base+entry);await page.waitForFunction(()=>window.__webizenNativeWorld||document.getElementById('native-fallback'),null,{timeout:180000});
    assert.ok(await page.evaluate(()=>window.__webizenNativeWorld),'Native world did not boot');
    await page.locator('#native-proof-lab>summary').click();await page.locator('#native-lab-tab-production').click();
    const catalogue=await (await page.request.get(base+'/data/publications/aaa-production/catalogue.json')).json();
    const expected={catchment:2,lettering:1,tone:1,surface:3};
    for(const recipe of catalogue.recipes){
      await page.locator('#native-production-recipe').selectOption(recipe.id);
      let response=page.waitForResponse(r=>r.url()===base+'/api/produce'&&r.request().method()==='POST');
      await page.locator('#native-production-run').click();const original=await verifyResponse(page,await response,recipe.id+'-default');
      assert.equal(original.resources.length,expected[recipe.id]);
      const parameter=recipe.parameters[0],changed=parameter.kind==='number'?String(parameter.max):'A shared living world';
      await page.locator('#native-production-param-'+parameter.key).fill(changed);
      response=page.waitForResponse(r=>r.url()===base+'/api/produce'&&r.request().method()==='POST');
      await page.locator('#native-production-run').click();const updated=await verifyResponse(page,await response,recipe.id+'-changed');
      assert.notEqual(updated.receipt.source_sha256,original.receipt.source_sha256);
      assert.notEqual(updated.receipt.package_sha256,original.receipt.package_sha256);
      assert.ok(updated.resources.some((r,i)=>r.sha256!==original.resources[i]?.sha256),"Changed parameter did not change any actual output bytes");
    }
    // Real invalid source reaches Synth. This is not an intercepted or fabricated refusal.
    await page.locator('#native-lab-production > details > summary').click();
    await page.locator('#native-production-custom').check();
    await page.locator('#native-production-source').fill('requires [capability("capability.invoke")]; let asset = capability.invoke("hypermedia.invalid.not_an_operation", {});');
    let response=page.waitForResponse(r=>r.url()===base+'/api/produce'&&r.request().method()==='POST');
    await page.locator('#native-production-run').click();const refused=await (await response).json();assert.equal(refused.ok,false);
    await page.waitForFunction(()=>!document.getElementById('native-production-status').textContent.includes('strictly replaying'));
    const refusalText=await page.locator('#native-production-status').innerText();assert.ok(refusalText.length>5);
    fs.writeFileSync(path.join(out,'refusal.json'),JSON.stringify({response:refused,display:refusalText},null,2));
    // Hold delivery of a genuine upstream response to exercise discarding deterministically.
    // No output bytes, status or headers are fabricated; the server still runs normally.
    await page.locator('#native-production-recipe').selectOption('tone');
    const before=await page.evaluate(()=>window.__webizenNativeInstrument.package_sha256);
    let release,upstreamReady;const held=new Promise(r=>release=r),ready=new Promise(r=>upstreamReady=r);
    await page.route('**/api/produce',async route=>{const upstream=await route.fetch();upstreamReady();await held;await route.fulfill({response:upstream});},{times:1});
    response=page.waitForResponse(r=>r.url()===base+'/api/produce'&&r.request().method()==='POST');
    await page.locator('#native-production-run').click();await bounded(ready);
    await page.locator('#native-production-cancel').click();assert.match(await page.locator('#native-production-status').innerText(),/discarded.*server may finish/i);
    release();await response;await page.waitForTimeout(500);
    assert.equal(await page.evaluate(()=>window.__webizenNativeInstrument.package_sha256),before);
    // A real subsequent job still works after discarding; camera input remains native.
    response=page.waitForResponse(r=>r.url()===base+'/api/produce'&&r.request().method()==='POST');
    await page.locator('#native-production-run').click();await verifyResponse(page,await response,'tone-after-discard');
    await page.locator('#native-proof-lab>summary').click();
    const yaw=await page.evaluate(()=>window.__webizenNativeWorld.camera.yaw);
    await page.locator('#native-world').focus();await page.keyboard.press('ArrowRight');
    await page.waitForFunction(y=>window.__webizenNativeWorld.camera.yaw!==y,yaw);
    const world=await page.evaluate(()=>window.__webizenNativeWorld);
    assert.ok(world.frame_timing.sample_count>0);assert.equal(world.frame_timing.gpu_duration_measured,false);
    const identity=await (await page.request.get(base+'/pkg/native-world/runtime-identity.json')).json();
    const wasm=await (await page.request.get(base+'/pkg/native-world/webizen_world_bg.wasm')).body();assert.equal(sha(wasm),identity.wasm.sha256);
    assert.equal(world.served_build_identity.wasm.sha256,identity.wasm.sha256,"Served runtime changed during browser qualification");
    assert.equal(new Set(scripts).size,1);assert.ok(scripts.every(s=>s.endsWith('/pkg/native-world/webizen_world.js')));assert.deepEqual(errors,[]);
    await page.screenshot({path:path.join(out,'world-after-production.png')});
    const result={recipes:catalogue.recipes.length,completed_runs:writes.length,refusal_verified:true,discard_verified:true,world_navigation:true,world,served_wasm_sha256:sha(wasm),errors,scripts};
    fs.writeFileSync(path.join(out,'results.json'),JSON.stringify(result,null,2));console.log(JSON.stringify({recipes:result.recipes,completed_runs:result.completed_runs,refusal:true,discard:true,world_navigation:true,served_wasm_sha256:sha(wasm)}));
  }catch(e){if(page){fs.writeFileSync(path.join(out,'failure.json'),JSON.stringify({error:String(e),state:await page.evaluate(()=>({body:document.body.innerText,world:window.__webizenNativeWorld,instrument:window.__webizenNativeInstrument})).catch(()=>null)},null,2));await page.screenshot({path:path.join(out,'failure.png')}).catch(()=>{});}throw e;}
  finally{await browser.close();}
})().catch(e=>{console.error(e);process.exitCode=1});
