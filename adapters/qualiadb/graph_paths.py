"""Bounded observer-purpose routes queried from the genuine published Q42 graph.

This is website route selection, not authentication or an access-control grant.
"""
import hashlib,json,subprocess
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
W='https://webizen.au/world/';R='https://webizen.au/relation/';P='https://webizen.au/purpose/'
RELATIONS=('information','knowledge','socioeconomic')
def sha(path):return hashlib.sha256(Path(path).read_bytes()).hexdigest()
class GraphPaths:
 def __init__(self,publication=None,helper=None):
  self.publication=Path(publication or ROOT/'publication/generated');self.helper=Path(helper or Path(__file__).with_name('q42_web.exe'))
 def _query(self,volume,predicate,subject='*',object='*'):
  run=subprocess.run([str(self.helper),'query',str(volume),predicate,subject,object],capture_output=True,text=True,encoding='utf-8',timeout=15)
  if run.returncode:raise RuntimeError(run.stderr.strip() or 'Native Q42 query failed')
  result=json.loads(run.stdout)
  return {'predicate':predicate,'subject':subject,'object':object,'rows':result['rows'],'reader':result['reader'],'verified_blocks':result['verified_blocks'],'total_quins':result['total_quins']}
 def relationships(self,kind='all'):
  if kind not in (*RELATIONS,'all'):raise ValueError('Unknown public relationship kind')
  pointer=json.loads((self.publication/'current.json').read_text(encoding='utf-8'));volume=(self.publication/pointer['q42']).resolve()
  if volume.parent!=self.publication.resolve() or volume.suffix!='.q42':raise ValueError('Invalid publication pointer')
  graph_hash=sha(volume)
  if graph_hash!=pointer['sha256']:raise ValueError('Published Q42 digest mismatch')
  queries=[self._query(volume,R+relation) for relation in (RELATIONS if kind=='all' else (kind,))]
  rows=sorted({tuple(row) for query in queries for row in query['rows']})
  return {'kind':kind,'rows':[{'subject':s,'predicate':p,'object':o} for s,p,o in rows],'graph_sha256':graph_hash,'reader':queries[0]['reader'],'verified_blocks':all(query['verified_blocks'] for query in queries),'graph_uri':'/publication/'+volume.name}
 def select(self,purpose,locale='en-AU',support_preferences=None):
  if purpose not in ('learn','research','create','participate'):raise ValueError('Unknown declared public purpose')
  if not isinstance(locale,str) or not 1<=len(locale)<=64:raise ValueError('Invalid locale')
  support_preferences=support_preferences or []
  if not isinstance(support_preferences,list) or any(x not in ('reduced-motion','large-text','reading-mode') for x in support_preferences):raise ValueError('Unknown support preference')
  pointer=json.loads((self.publication/'current.json').read_text(encoding='utf-8'));volume=(self.publication/pointer['q42']).resolve()
  if volume.parent!=self.publication.resolve() or volume.suffix!='.q42':raise ValueError('Invalid publication pointer')
  graph_hash=sha(volume)
  if graph_hash!=pointer['sha256']:raise ValueError('Published Q42 digest mismatch')
  queries=[]
  def query(predicate,subject='*',object='*'):
   result=self._query(volume,predicate,subject,object);queries.append(result);return result['rows']
  starts=query(R+'startsAt',P+purpose);preferred=query(R+'prefersRelation',P+purpose)
  if len(starts)!=1 or len(preferred)!=1:raise ValueError('Purpose policy is absent or ambiguous in the graph')
  start=starts[0][2];preference=preferred[0][2].removeprefix(R)
  if preference not in RELATIONS:raise ValueError('Unrecognised relationship policy')
  kinds=query(R+'kind');nodes={s for s,p,o in kinds if s.startswith(W)}
  if start not in nodes:raise ValueError('Purpose anchor is not a published exhibit')
  rank=[preference]+[r for r in RELATIONS if r!=preference]
  adjacency={n:[] for n in nodes};reasons={start:'Graph purpose anchor'}
  for relation in rank:
   for source,predicate,target in query(R+relation):
    if source in nodes and target in nodes:adjacency[source].append((rank.index(relation),target,relation))
  route=[];pending=[start];seen=set()
  while pending:
   current=pending.pop(0)
   if current in seen:continue
   seen.add(current);route.append(current)
   for _,target,relation in sorted(adjacency[current]):
    if target not in seen:
     pending.append(target);reasons.setdefault(target,relation+' relation from '+current.removeprefix(W))
  for remaining in sorted(nodes-seen):route.append(remaining);reasons[remaining]='Other public exhibit; no directed path from this purpose anchor'
  places=[x.removeprefix(W) for x in route]
  return {'purpose':purpose,'places':places,'graph':{'returned':len(nodes),'stats':{'matchCount':len(nodes)},'total_quins':queries[0]['total_quins']},'selection_receipt':{'schema':'webizen.path-selection/v1','graph_sha256':graph_hash,'graph_uri':'/publication/'+volume.name,'reader_binary_sha256':sha(self.helper),'policy':'public-graph-breadth-first/v1','preferred_relation':preference,'locale':locale,'support_preferences':support_preferences,'authority':'anonymous-public','authority_from_purpose':False,'queries':queries,'decisions':[{'entity':x,'reason':reasons[W+x]} for x in places]},'method':'Native QualiaDB Q42 predicate queries select the declared-purpose anchor and relationship priority. The website traverses those returned edges; remaining public places stay reachable. This changes presentation, never authority.'}
if __name__=='__main__':
 import argparse
 p=argparse.ArgumentParser();p.add_argument('purpose',choices=['learn','research','create','participate']);p.add_argument('--publication',type=Path);a=p.parse_args();print(json.dumps(GraphPaths(a.publication).select(a.purpose),ensure_ascii=False,indent=2))