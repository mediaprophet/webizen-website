"""Real codec, query, corruption and changed-policy checks; no mock Q42 reader."""
import copy,hashlib,importlib.util,json,subprocess,unittest,uuid
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('graph_paths',ROOT/'adapters/qualiadb/graph_paths.py');gp=importlib.util.module_from_spec(spec);spec.loader.exec_module(gp)
HELPER=ROOT/'adapters/qualiadb/q42_web.exe';PUB=ROOT/'publication/generated'
spec=importlib.util.spec_from_file_location('publisher',ROOT/'publication/publish.py');publisher=importlib.util.module_from_spec(spec);spec.loader.exec_module(publisher)
def native(*args):
 r=subprocess.run([str(HELPER),*map(str,args)],capture_output=True,text=True,encoding='utf-8',timeout=30)
 if r.returncode:raise RuntimeError(r.stderr)
 return json.loads(r.stdout)
class PublicationTests(unittest.TestCase):
 @classmethod
 def setUpClass(cls):
  cls.pointer=json.loads((PUB/'current.json').read_text());cls.volume=PUB/cls.pointer['q42'];cls.routes={p:gp.GraphPaths().select(p) for p in ['learn','research','create','participate']}
 def test_native_roundtrip_preserves_terms(self):
  decoded=native('inspect',self.volume);expected=json.loads((PUB/'triples.json').read_text(encoding='utf-8'));self.assertEqual(sorted(decoded['rows']),sorted(expected));self.assertTrue(decoded['verified_blocks']);self.assertEqual(decoded['version'],4)
 def test_native_places_preserve_vibe_camera_and_position(self):
  layout=json.loads((ROOT/'data/native-world-layout.json').read_text(encoding='utf-8'));rows=native('inspect',self.volume)['rows'];bundle=json.loads((PUB/'publication.json').read_text(encoding='utf-8'))
  self.assertEqual(layout['source_sha256'],hashlib.sha256((ROOT/'site/world/living-world.vibe').read_bytes()).hexdigest())
  for place in layout['places']:
   published=next(e for e in bundle['exhibits'] if e['id']==place['id'])
   for key in ('position','camera'):
    values=[json.loads(o) for s,p,o in rows if s==gp.W+place['id'] and p==gp.R+key]
    self.assertEqual(values,[place[key]]);self.assertEqual(published[key],place[key])
 def test_seven_journey_chapters_native_projection_roundtrip(self):
  source=json.loads((ROOT/'data/journey.json').read_text(encoding='utf-8'));projection=json.loads((PUB/'journey.json').read_text(encoding='utf-8'));decoded=native('inspect',self.volume)
  self.assertEqual(publisher.decoded_journey(decoded['rows']),source)
  self.assertEqual({k:projection[k] for k in source},source);self.assertEqual(len(projection['chapters']),7)
  self.assertEqual(projection['graph']['sha256'],self.pointer['sha256']);self.assertEqual(projection['graph']['format'],'q42');self.assertTrue(projection['graph']['verified_blocks']);self.assertTrue(projection['projection']['semantic_roundtrip_equal'])
  expected_hash=hashlib.sha256((ROOT/'data/journey.json').read_bytes()).hexdigest();self.assertEqual(projection['projection']['source_sha256'],expected_hash)
  receipt=json.loads((PUB/'q42-receipt.json').read_text());self.assertEqual(receipt['journey']['source_sha256'],expected_hash);self.assertIn(expected_hash,[s['sha256'] for s in receipt['sources']])
  links=native('query',self.volume,gp.R+'hasChapter',gp.W+'field-studio','*')['rows'];self.assertEqual(len(links),7);self.assertTrue(all(o.startswith(publisher.J) and not o.startswith(gp.W) for s,p,o in links))
 def test_journey_public_allowlist_excludes_exact_locations_and_dates(self):
  source=json.loads((ROOT/'data/journey.json').read_text(encoding='utf-8'))
  for c in source['chapters']:self.assertEqual(set(c),set(publisher.JOURNEY_FIELDS))
  for key,value in [('latitude',-35.123456),('longitude',149.123456),('coordinates',[-35,149]),('address','private location'),('arrival_time','2026-05-23T12:34:00Z')]:
   changed=copy.deepcopy(source);changed['chapters'][0][key]=value;self.assertRaises(ValueError,publisher.validate_journey,changed)
  changed=copy.deepcopy(source);changed['chapters'][0]['month_bands']=['2026-05-23'];self.assertRaises(ValueError,publisher.validate_journey,changed)
 def test_journey_decoded_membership_and_duplicate_fields_fail_closed(self):
  rows=native('inspect',self.volume)['rows'];missing=[r for r in rows if not (r[0]==gp.W+'field-studio' and r[1]==gp.R+'hasChapter' and r[2]==publisher.J+'riverina')];self.assertRaises(ValueError,publisher.decoded_journey,missing)
  duplicate=rows+[[publisher.J+'riverina',gp.R+'title','"Conflicting title"']];self.assertRaises(ValueError,publisher.decoded_journey,duplicate)
 def test_four_distinct_graph_derived_purposes(self):
  sequences={tuple(r['places']) for r in self.routes.values()};self.assertEqual(len(sequences),4)
  for r in self.routes.values():self.assertEqual(len(r['places']),7);self.assertEqual(len(set(r['places'])),7);self.assertEqual(r['selection_receipt']['authority'],'anonymous-public');self.assertFalse(r['selection_receipt']['authority_from_purpose']);self.assertTrue(all(q['verified_blocks'] for q in r['selection_receipt']['queries']))
 def test_decoded_relationships_match_real_q42(self):
  triples=json.loads((PUB/'triples.json').read_text(encoding='utf-8'))
  for kind in (*gp.RELATIONS,'all'):
   result=gp.GraphPaths().relationships(kind);predicates={gp.R+r for r in (gp.RELATIONS if kind=='all' else (kind,))};expected=sorted(tuple(row) for row in triples if row[1] in predicates);actual=[(row['subject'],row['predicate'],row['object']) for row in result['rows']];self.assertEqual(actual,expected);self.assertEqual(result['graph_sha256'],self.pointer['sha256']);self.assertEqual(result['reader'],'qualia_core_db::q42_volume::Q42Volume');self.assertTrue(result['verified_blocks'])
 def test_unknown_relationship_kind_refused(self):self.assertRaises(ValueError,gp.GraphPaths().relationships,'https://example.invalid/arbitrary')
 def test_relationship_digest_mismatch_refused(self):
  out=ROOT/'tests/publication/runs'/uuid.uuid4().hex;out.mkdir(parents=True);volume=out/'wrong.q42';volume.write_bytes(self.volume.read_bytes());(out/'current.json').write_text(json.dumps({'q42':volume.name,'sha256':'0'*64}));self.assertRaises(ValueError,gp.GraphPaths(out).relationships,'all')
 def test_unknown_purpose_refused(self):self.assertRaises(ValueError,gp.GraphPaths().select,'administrator')
 def test_unknown_support_does_not_escalate(self):self.assertRaises(ValueError,gp.GraphPaths().select,'learn',support_preferences=['administrator'])
 def test_corrupt_file_refused_by_real_reader(self):
  out=ROOT/'tests/publication/runs'/uuid.uuid4().hex;out.mkdir(parents=True);bad=out/'truncated.q42';bad.write_bytes(self.volume.read_bytes()[:100]);self.assertRaises(RuntimeError,native,'inspect',bad)
 def test_actual_graph_policy_change_changes_route(self):
  out=ROOT/'tests/publication/runs'/uuid.uuid4().hex;out.mkdir(parents=True);triples=json.loads((PUB/'triples.json').read_text(encoding='utf-8'))
  changed=0
  for row in triples:
   if row[:2]==[gp.P+'research',gp.R+'startsAt']:row[2]=gp.W+'canopy';changed+=1
  self.assertEqual(changed,1);source=out/'triples.json';source.write_text(json.dumps(triples,ensure_ascii=False),encoding='utf-8');volume=out/'changed.q42';native('build',source,volume);(out/'current.json').write_text(json.dumps({'q42':volume.name,'sha256':hashlib.sha256(volume.read_bytes()).hexdigest()}));result=gp.GraphPaths(out).select('research');self.assertEqual(result['places'][0],'canopy');self.assertNotEqual(result['places'],self.routes['research']['places'])
 def test_publication_digest_mismatch_refused(self):
  out=ROOT/'tests/publication/runs'/uuid.uuid4().hex;out.mkdir(parents=True);volume=out/'wrong.q42';volume.write_bytes(self.volume.read_bytes());(out/'current.json').write_text(json.dumps({'q42':volume.name,'sha256':'0'*64}));self.assertRaises(ValueError,gp.GraphPaths(out).select,'learn')
if __name__=='__main__':unittest.main()
