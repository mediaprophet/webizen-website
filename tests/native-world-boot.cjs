// Browser test harness only; never shipped as world implementation.
const {chromium}=require('C:/Users/Hp/AppData/Local/npm-cache/_npx/9833c18b2d85bc59/node_modules/playwright-core');
const fs=require('node:fs');
const route=process.argv[2]||'/native-prototype.html',mobile=process.argv.includes('--mobile');
const out='evidence/native-world/browser/'+(route.includes('prototype')?'prototype':'world')+(mobile?'-mobile':'');fs.mkdirSync(out,{recursive:true});
(async()=>{const b=await chromium.launch({executablePath:'C:/Program Files/Google/Chrome/Application/chrome.exe',headless:true,args:process.env.NATIVE_GPU==='hardware'?['--enable-unsafe-webgpu','--ignore-gpu-blocklist']:['--enable-unsafe-webgpu','--enable-unsafe-swiftshader','--use-angle=swiftshader','--disable-gpu-sandbox']});try{
const p=await b.newPage({viewport:mobile?{width:390,height:844}:{width:1440,height:1000},reducedMotion:'reduce',locale:'en-US',hasTouch:mobile});const errors=[],scripts=[],failed=[];
p.on('pageerror',e=>errors.push(e.message));p.on('request',r=>{if(r.resourceType()==='script')scripts.push(r.url());});p.on('response',r=>{if(r.status()>=400&&!r.url().endsWith('/favicon.ico'))failed.push({url:r.url(),status:r.status()});});p.on('console',m=>{if(m.type()==='error')errors.push(m.text());});
await p.goto('http://127.0.0.1:8800'+route);try{await p.waitForFunction(()=>window.__webizenNativeWorld||document.getElementById('native-fallback'),null,{timeout:90000});}catch(e){errors.push(e.message);}
const result=await p.evaluate(()=>({receipt:window.__webizenNativeWorld||null,status:document.getElementById('native-world-status')?.textContent,language:document.documentElement.lang,body:document.body.innerText.slice(0,3000),canvas:{width:document.querySelector('canvas').width,height:document.querySelector('canvas').height}}));
await p.screenshot({path:out+'/desktop.png',timeout:30000});Object.assign(result,{route,mobile,scripts,errors,failed});fs.writeFileSync(out+'/boot.json',JSON.stringify(result,null,2));console.log(JSON.stringify({receipt:result.receipt?{renderer:result.receipt.renderer,frames:result.receipt.frames,upload:result.receipt.upload,scene:result.receipt.scene}:null,status:result.status,scripts,errors,failed,evidence:out}));
if(!result.receipt||result.language!=='en-AU'||failed.length||scripts.some(s=>!s.endsWith('/pkg/native-world/webizen_world.js')))process.exitCode=1;
}finally{await b.close();}})().catch(e=>{console.error(e);process.exitCode=1;});
