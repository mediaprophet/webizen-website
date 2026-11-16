"""Assemble reviewed native assets and broad journey chapters in the website."""
from pathlib import Path
import json,hashlib,sys
root=Path(__file__).resolve().parents[1]
camp=root/'production/campsite'
def local(value):return Path(str(value).removeprefix('\\\\?\\'))
def url(value):return '/'+local(value).relative_to(root).as_posix()
assets=[]
for folder,name,instance_id,title,pos,angle,anchor,panel_count in [
 ('car','car','car-passat-b7','Gold 2012–13 Volkswagen Passat wagon with roof box',[-3.5,0,1.3],-.12,'ground',None),
 ('camper','vintage-windup-camper','vintage-windup-camper','1980-era Jayco wind-up camper',[1,0,0],0,'ground',None),
 ('equipment','solar-array','solar-array','Three-panel solar array — first',[-.4,0,5.6],0,'ground',3),
 ('equipment','solar-array','solar-array-second','Three-panel solar array — second',[1.8,0,5.6],0,'ground',3),
 ('equipment','solar-array-two','solar-array-two','Two-panel solar array',[3.7,0,5.6],0,'ground',2),
 ('equipment','starlink-mini','starlink-mini','Starlink Mini roof connection',[1.7,2.72,-.65],0,'camper',None)]:
 m=json.loads((camp/folder/'assets'/f'{name}.manifest.json').read_text(encoding='utf-8'))
 native=m['native']; assert hashlib.sha256(local(native['path']).read_bytes()).hexdigest()==native['sha256']
 for k in ['production','sealing']:assert m[k]['strict_replay_verified']
 asset={'id':m['id'],'title':title,'mesh_url':url(m['mesh_json']),'mesh_sha256':hashlib.sha256(local(m['mesh_json']).read_bytes()).hexdigest(),'native_url':url(native['path']),'native_sha256':native['sha256'],'recipe_url':url(m['recipe']),'receipt_url':'/production/campsite/'+folder+m['sealing']['receipt_url'],'position':pos,'rotation_y':angle,'anchor':anchor,'qualification':m.get('qualification','Authored dimensional interpretation; not scanned manufacturer CAD.'),'strict_replay_verified':True}
 asset['geometry_id']=m['id'];asset['id']=instance_id
 if panel_count is not None:asset['panel_count']=panel_count
 assets.append(asset)
manifest={'schema':'webizen.campsite/v1','title':'The field studio','default_locale':'en-AU','description':'A working campsite: shelter, energy and connection in a living landscape. Authored from the creator’s description, with broad journey chapters drawn from a private timeline.','geometry':'Native Synth loft/revolve geometry, exported losslessly for the browser and sealed as .10d. Three.js presents the scene.','coordinates':'Metres in a fictional scene; these are not geographic coordinates.','assets':assets}
(camp/'campsite.json').write_text(json.dumps(manifest,ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
if '--assets-only' in sys.argv:
 print('Native campsite asset manifest updated: revised car, roof box, 3+3+2 solar arrays.')
 raise SystemExit(0)
p=root/'site/world/scene.js';s=p.read_text(encoding='utf-8')
s=s.replace('scene.add(new T.HemisphereLight(0xe2f0ea,0x526250,2.2));','const hemisphere=new T.HemisphereLight(0xe2f0ea,0x526250,1.3);scene.add(hemisphere);')
s=s.replace('new T.DirectionalLight(0xffdc9d,3.5)','new T.DirectionalLight(0xffdc9d,2.5)')
s=s.replace('return{go(id)',"let chapterId=null;\n return{chapter(chapter){chapterId=chapter?.id||null;if(!chapter)return;const[earth,leaf,light,skyColour]=chapter.artistic_palette;scene.fog.color.set(skyColour);scene.background.set(skyColour);sky.material.uniforms.top.value.set(skyColour);sky.material.uniforms.bottom.value.set(light);hemisphere.groundColor.set(earth);hemisphere.color.set(light);sun.color.set(light);},go(id)")
s=s.replace('return{nativeAssets,meshes:','return{nativeAssets,chapter:chapterId,meshes:')
p.write_text(s,encoding='utf-8')
p=root/'site/world/app.js';s=p.read_text(encoding='utf-8')
if 'let journey=null' not in s:
 s=s.replace('const $=id=>','let journey=null,activeChapter=null;\nconst $=id=>',1)
 s=s.replace("$('location').hidden=false;","if(current==='field-studio'&&activeChapter){$('locationText').textContent=activeChapter.title+' · '+activeChapter.landscape_copy;}$('location').hidden=false;",1)
 s=s.replace("if(id==='field-studio'){const journey=node('a');", "if(id==='field-studio'){chapterControls(body);const journey=node('a');")
 s=s.replace('function showMap(){',"""function chapterControls(parent){if(!journey)return;const section=node('section','experiment');section.lang='en-AU';const heading=node('h3');heading.textContent='Landscapes along the way';const label=node('label');label.textContent='Choose a landscape chapter';const select=node('select');select.id='journey-chapter';for(const c of journey.chapters){const option=node('option');option.value=c.id;option.textContent=c.title;select.append(option);}select.value=activeChapter?.id||journey.chapters[0].id;const copy=node('p');const dates=node('p','muted');function apply(){activeChapter=journey.chapters.find(c=>c.id===select.value);world.chapter(activeChapter);copy.textContent=activeChapter.landscape_copy;dates.textContent=activeChapter.month_bands.map(m=>new Intl.DateTimeFormat(localeState().requested,{month:'short',year:'numeric',timeZone:'UTC'}).format(new Date(m+'-01T00:00:00Z'))).join(' · ');const url=new URL(location.href);url.searchParams.set('chapter',activeChapter.id);history.replaceState(null,'',url);updateLocation();}select.onchange=apply;label.append(select);section.append(heading,label,copy,dates);const note=node('small','muted');note.textContent='Broad memories of place, expressed through light and colour. A shared artistic setting; no exact campsite locations.';section.append(note);parent.append(section);apply();}
function showMap(){""")
 s=s.replace('setMotion(motion);\n try{',"setMotion(motion);\n try{journey=await fetch('/data/journey.json').then(r=>r.json());activeChapter=journey.chapters.find(c=>c.id===new URLSearchParams(location.search).get('chapter'))||null;if(activeChapter)world.chapter(activeChapter);}catch(error){console.warn('Journey chapters unavailable',error);}\n try{",1)
 s=s.replace("a.href='/reading.html';a.textContent='HTML reading route · English'","a.href='/publication/reading.html';a.textContent='HTML reading route · Australian English'")
p.write_text(s,encoding='utf-8')
p=root/'publication/publish.py';s=p.read_text(encoding='utf-8').replace("'reading_route':'/reading.html#'","'reading_route':'/publication/reading.html#'");p.write_text(s,encoding='utf-8')
p=root/'tests/publication/test_publication.py';s=p.read_text(encoding='utf-8').replace("len(r['places']),6","len(r['places']),7").replace("len(set(r['places'])),6","len(set(r['places'])),7");p.write_text(s,encoding='utf-8')
print('Six campsite instances assembled, including three solar arrays with 3+3+2 panels.')
