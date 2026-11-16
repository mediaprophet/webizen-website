from pathlib import Path
root=Path(__file__).resolve().parents[1];p=root/'site/lab/lab.js';s=p.read_text(encoding='utf-8')
s=s.replace("$('export-tests').disabled=false;}catch", "$('test-summary').append(document.createTextNode(' · engine '+(testResult.environment?.engine_version||'JavaScript logic')));$('export-tests').disabled=false;}catch")
s=s.replace("browser WASM · ${number", "browser WASM ${data.environment?.engine_version||'version not reported'} · ${number")
s=s.replace("scene=await createLab($('lab-scene'),gallery);", "scene=await createLab($('lab-scene'),gallery);$('scene-motion').textContent=scene.state().motion?'Pause movement':'Resume movement';$('scene-motion').setAttribute('aria-pressed',String(!scene.state().motion));")
p.write_text(s,encoding='utf-8');print('Runtime identity and reduced-motion control are explicit.')
