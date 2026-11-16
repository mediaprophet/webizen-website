from pathlib import Path
import shutil,json
root=Path(__file__).resolve().parents[1];repo=Path('C:/github/webizen-website');world=root/'site/world';world.mkdir(parents=True,exist_ok=True)
p=world/'scene.js'
if not p.exists():shutil.copy2(repo/'site/world/scene.js',p)
s=p.read_text(encoding='utf-8')
s=s.replace("import {registerEnvironment,getEnvironment,allExhibits} from './registry.js';","import {registerEnvironment,getEnvironment,allExhibits} from './registry.js';\nconst nativeAssets=[];\nasync function nativeWork(id,parent,x,z,scale){const data=await fetch('/production/assets/'+id+'.mesh.json').then(r=>{if(!r.ok)throw Error('Native artwork unavailable');return r.json();});const group=new T.Group();for(const p of data.parts){const geo=new T.BufferGeometry();geo.setAttribute('position',new T.Float32BufferAttribute(p.positions,3));geo.setIndex(p.indices);geo.computeVertexNormals();const m=new T.Mesh(geo,new T.MeshStandardMaterial({...data.palette[p.material],side:T.DoubleSide}));m.castShadow=true;m.receiveShadow=true;group.add(m);}group.position.set(x,heightAt(x,z)+.65*scale,z);group.scale.setScalar(scale);parent.add(group);nativeAssets.push({id,parts:data.parts.length,source:'Synth-exported DXF geometry'});}")
s=s.replace('pavilion(-17,6);pavilion(20,-7,.8);',"nativeWork('canopy',root,-17,6,.45).catch(console.error);nativeWork('tidal-vessel',root,20,-7,.8).catch(console.error);nativeWork('memory-frond',root,-20,-16,.6).catch(console.error);")
s=s.replace("return{meshes:renderer.info.render.calls","return{nativeAssets,meshes:renderer.info.render.calls")
p.write_text(s,encoding='utf-8')
for name in ['en.json','en-AU.json']:
 target=root/'site/locales'/name;target.parent.mkdir(parents=True,exist_ok=True)
 data=json.loads((repo/'site/locales'/name).read_text(encoding='utf-8'));data['aboutGraph']='The public website graph is published as a genuine .q42 volume and verified by the native QualiaDB reader. Declared-purpose anchors and typed relationships select the route; Vibe packages the resulting sequence. Authentication, private graph views, delegation and contestability remain separate host integration work.'
 target.write_text(json.dumps(data,ensure_ascii=False,indent=2),encoding='utf-8')
p=root/'tooling/world_server.py';s=p.read_text(encoding='utf-8');start=s.index("    predicate='?p'",s.index("   if kind=='query':"));end=s.index("   if kind=='rights':",start)
s=s[:start]+'''    result=public_graph.relationships(value)
    return self.json_response({'query':'Native Q42 relationship selection: '+value,'result':{'returned':len(result['rows'])},'readable':result['rows'],'receipt':result,'execution':'Native QualiaDB Q42 reader','source_graph':result['graph_uri']})
'''+s[end:];p.write_text(s,encoding='utf-8')
print('Actual Synth meshes integrated into the world; native Q42 path and relationship descriptions updated.')
