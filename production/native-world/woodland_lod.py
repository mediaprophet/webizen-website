"""Native QEM woodland LOD. Source faces are simplified only by Synth morphisms."""
from pathlib import Path
import json,hashlib,sys,shutil,subprocess,time,struct
HERE=Path(__file__).resolve().parent
sys.path.insert(0,str(HERE.parent))
from build_art import parse_dxf,lit
from native_bridge import SynthBridge

def build_lod(bridge,source,target):
 data=json.loads(source.read_text());name='eucalyptus-'+str(target);folder=HERE/'lod';folder.mkdir(exist_ok=True);cache=folder/(name+'.json');source_sha=hashlib.sha256(source.read_bytes()).hexdigest()
 if cache.exists():
  prior=json.loads(cache.read_text())
  if prior.get('source_sha256')==source_sha:return prior
 total=sum(len(p['indices'])//3 for p in data['parts']);lines=['requires [capability("capability.invoke")];'];inputs=[];files=[]
 for i,p in enumerate(data['parts']):
  obj=folder/(name+'-'+p['material']+'.obj');pos=p['positions'];ind=p['indices'];obj.write_text('\n'.join(['v '+' '.join(str(x) for x in pos[k:k+3]) for k in range(0,len(pos),3)]+['f '+' '.join(str(x+1) for x in ind[k:k+3]) for k in range(0,len(ind),3)]));key=str(i)+'.obj';inputs.append({'key':key,'kind':'obj','path':str(obj)});goal=max(8,int(target*(len(ind)//3)/total))
  lines.extend([f'let input{i}=capability.invoke("hypermedia.manifold.import_mesh",{{input:"{key}",format:"obj"}});',f'let mesh{i}=capability.invoke("hypermedia.manifold.simplify",{{mesh:input{i},mode:"decimate",target_triangles:{goal},max_error:10.0,attribute_policy:"discard"}});',f'let dxf{i}=capability.invoke("hypermedia.manifold.project_dxf",{{mesh:mesh{i},layer:"{p["material"]}"}});'])
  files.append(f'{{key:"{p["material"]}.dxf",resource:dxf{i}}}')
 lines.append('let mut asset={files:['+','.join(files)+']};');recipe='\n'.join(lines);(folder/(name+'.vibe')).write_text(recipe);result=bridge.produce(recipe,name,inputs=inputs);assert result['receipt']['strict_replay_verified'];resources={r['key']:r for r in result['receipt']['resources']};parts=[]
 for p in data['parts']:
  res=resources[p['material']+'.dxf'];pos,ind=parse_dxf(Path(res['path']));parts.append({'name':p['material'],'material':p['material'],'positions':pos,'indices':ind,'source_sha256':res['sha256']})
 record={'id':name,'schema':'webizen.native-lod/v1','source_sha256':source_sha,'source_triangles':total,'target_triangles':target,'triangles':sum(len(p['indices'])//3 for p in parts),'parts':parts,'palette':data['palette'],'production':result,'method':'hypermedia.manifold.simplify mode decimate, max_error10; originals retained'};cache.write_text(json.dumps(record,separators=(',',':')));print('NATIVE_LOD',name,record['triangles'],flush=True);return record

if __name__=='__main__':
 bridge=SynthBridge(HERE)
 try:
  for target in ([int(sys.argv[1])] if len(sys.argv)>1 else [40000,2000]):build_lod(bridge,HERE.parent/'campsite/woodland/assets/eucalyptus.json',target)
 finally:bridge.close()


