from pathlib import Path
import sys,json,hashlib,collections,shutil
P=Path(__file__).resolve().parent;sys.path.insert(0,str(P.parent))
from native_bridge import SynthBridge
from build_art import lit
R=P/'review/habitat-field';R.mkdir(parents=True,exist_ok=True)
world=(P/'inputs/fictional-world-frame.vibe').read_text();field=(P.parent/'aaa-terrain/material-field.vibe').read_text();size=64
samples=[[-90+(x+.5)*180/size,-90+(z+.5)*180/size] for z in range(size) for x in range(size)]
b=SynthBridge(P)
try:
 cells=[];batches=[]
 for start in range(0,len(samples),32):
  src=world+'\n'+field+'\nlet cells=habitat_cells('+lit(samples[start:start+32])+'); let palette=habitat_palette();'
  result=b.call('vibe_eval',{'source':src});cells.extend(result['bindings']['cells']);batches.append(result)
  if start%512==0:print('NATIVE_CELLS',len(cells),flush=True)
 assert len(cells)==4096 and all(isinstance(v,(int,float)) and int(v)==v and 0<=v<=31 for v in cells)
 palette=batches[-1]['bindings']['palette'];assert len(palette)==32 and all(0<=v<=1 for row in palette for v in row)
 repeat=b.call('vibe_eval',{'source':world+'\n'+field+'\nlet cells=habitat_cells('+lit(samples[:32])+');'})
 assert repeat['bindings']['cells']==cells[:32]
 paths=collections.defaultdict(list)
 for i,v in enumerate(cells):paths[int(v)].append(f'M{(i%size)*8} {(i//size)*8}h8v8h-8Z')
 layers=[]
 for key,commands in sorted(paths.items()):
  for start in range(0,len(commands),700):layers.append({'path':''.join(commands[start:start+700]),'fill':[round(v*255) for v in palette[key]]})
 source='requires [capability("capability.invoke")];\nlet habitat=capability.invoke("hypermedia.radiance.render_vector",'+lit({'width':512,'height':512,'layers':layers})+');\nlet soft=capability.invoke("hypermedia.radiance.filter_rgba",{image:habitat,operation:"blur",sigma:3});\nlet colour=capability.invoke("hypermedia.radiance.encode_rgba_png",{image:soft});\nlet mut asset={files:[{key:"habitat-colour-candidate.png",resource:colour}]};'
 (R/'render.vibe').write_text(source);result=b.produce(source,'habitat-field-comparison');assert result['receipt']['strict_replay_verified']
 resource=next(r for r in result['receipt']['resources'] if r['key']=='habitat-colour-candidate.png');shutil.copyfile(resource['path'],R/'candidate.png');shutil.copyfile(P.parent/'aaa-terrain/assets/habitat-colour.png',R/'before.png')
 report={'field_path':str(P.parent/'aaa-terrain/material-field.vibe'),'field_sha256':hashlib.sha256(field.encode()).hexdigest(),'source_frame_sha256':hashlib.sha256(world.encode()).hexdigest(),'native_batches':batches,'sample_count':len(cells),'index_range':[min(cells),max(cells)],'palette':palette,'repeated_sample_verified':True,'production':result,'before_png':str(R/'before.png'),'candidate_png':str(R/'candidate.png'),'review':'pending actual image inspection; no geometry or integratedmaps changed'}
 (R/'comparison.json').write_text(json.dumps(report,indent=2));print('COMPLETE',str(R/'candidate.png'),report['index_range'],flush=True)
finally:b.close()
