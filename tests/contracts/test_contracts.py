"""Contract invariants using explicitly non-public fixtures; no performance claims."""
import copy,importlib.util,json,sys,unittest
from pathlib import Path
ROOT=Path(__file__).resolve().parents[2]
spec=importlib.util.spec_from_file_location('webizen_contracts',ROOT/'contracts/validate.py');validator=importlib.util.module_from_spec(spec);spec.loader.exec_module(validator)
SOURCE={'repository':'contract-fixtures','revision':'test-fixture-only','path':'fixtures/README.md','sha256':'0'*64}
EVIDENCE={'kind':'test-fixture','uri':'/contracts/fixtures/README.md','sha256':'0'*64,'observed_at':None}
def rec(kind,id,**kwargs):return {'schema':'webizen.'+kind+'/v1','id':id,**kwargs}
def fixture():
 return rec('publication','contract-fixture',stage='draft',default_locale='en-AU',created_at='2026-11-16T00:00:00Z',sources=[SOURCE.copy()],runtimes=[{'id':'browser-fixture','profile':'browser','version':'test-fixture','device':None,'backend':None}],
 modules=[rec('module','example-module',name='Contract fixture',purpose='Exercise validation',difference='Not a product claim',human_significance='Prevent unsupported public claims',qualification='source-indexed',sources=[SOURCE.copy()],capabilities=['fixture-only'],runtime_ids=['browser-fixture'],related_modules=[],evidence=[EVIDENCE.copy()])],
 exhibits=[rec('exhibit','example-exhibit',title_key='exhibit.title',body_key='exhibit.body',module_ids=['example-module'],experiment_id='example-experiment',environment='example-environment',position=[0,0,0],camera=[1,2,3],reading_route='/read/example',asset_ids=['example-asset'],related_exhibits=[])],
 experiments=[rec('experiment','example-experiment',module_ids=['example-module'],runner_id='fixture-runner',runner_version='fixture-v1',execution='browser',source=SOURCE.copy(),inputs=[{'id':'count','type':'integer','label_key':'count','unit':'items','minimum':1,'maximum':10}],outputs=[{'name':'result','media_type':'application/json','unit':'items','description':'Fixture output'}],epistemic_class='executable-logic',runtime_ids=['browser-fixture'],max_duration_ms=1000,max_input_bytes=128,supports_cancel=True,supports_reset=True,evidence=[EVIDENCE.copy()])],
 test_runs=[rec('test-run','example-test',suite_id='fixture-suite',runtime_id='browser-fixture',source=SOURCE.copy(),status='not-run',assertion_count=0,duration_ms=None,executed_at=None,reason='Contract fixture; no execution claimed',evidence=[])],
 benchmarks=[rec('benchmark','example-benchmark',workload_id='fixture-workload',runtime_id='browser-fixture',source=SOURCE.copy(),classification='not-run',correctness='not-checked',dataset_id='fixture-dataset',dataset_sha256=None,method='Schema fixture, not a benchmark result',sample_unit='ms',samples=None,sample_count=0,warmup_count=0,summary=None,executed_at=None,exclusions=[],evidence=[])],
 locales=[rec('locale','locale-en-au',locale='en-AU',default=True,source_locale='en-AU',source_revision='fixture-v1',direction='ltr',messages={'exhibit.title':{'value':'Example','language':'en-AU','review_state':'draft','reviewer':None}},fallbacks=[],evidence=[])],
 assets=[rec('asset','example-asset',uri='/contracts/fixtures/example.svg',media_type='image/svg+xml',bytes=0,sha256='0'*64,origin='source-generated',recipe=None,receipt=None,preview_uri=None,visual_acceptance='pending',evidence=[])],
 observers=[rec('observer','public-visitor',purpose='learn',locale='en-AU',support_preferences=['reduced-motion'],authority={'status':'anonymous','grant_refs':[],'evidence':[]},graph_sha256='0'*64,policy_version='fixture-v1',selected_exhibits=['example-exhibit'],reasons=['Declared learning purpose'],authority_from_purpose=False)],
 migrations=[rec('migration','example-route',legacy_route='/legacy/example.html',source_sha256='0'*64,destination='/read/example',disposition='recreate',module_ids=['example-module'],experiment_ids=['example-experiment'],qualification='source-only',reason=None,evidence=[])],graph=None)
class ContractTests(unittest.TestCase):
 def setUp(self):self.data=fixture()
 def rejected(self):
  with self.assertRaises(validator.ValidationError):validator.validate(self.data)
 def test_full_fixture_passes(self):self.assertTrue(validator.validate(self.data))
 def test_unknown_version(self):self.data['schema']='webizen.publication/v999';self.rejected()
 def test_unknown_property(self):self.data['mock_success']=True;self.rejected()
 def test_dangling_reference(self):self.data['exhibits'][0]['experiment_id']='missing';self.rejected()
 def test_duplicate_identity(self):self.data['modules'].append(copy.deepcopy(self.data['modules'][0]));self.rejected()
 def test_ambiguous_default(self):self.data['default_locale']='en';self.rejected()
 def test_missing_default(self):self.data['locales'][0]['default']=False;self.rejected()
 def test_zero_assertions_cannot_pass(self):self.data['test_runs'][0].update(status='passed',executed_at='2026-11-16T00:00:00Z',evidence=[EVIDENCE.copy()]);self.rejected()
 def test_not_run_needs_reason(self):self.data['test_runs'][0]['reason']=None;self.rejected()
 def test_benchmark_correctness_gate(self):self.data['benchmarks'][0].update(classification='measured',samples=[1,2],sample_count=2,executed_at='2026-11-16T00:00:00Z',evidence=[EVIDENCE.copy()]);self.rejected()
 def test_benchmark_sample_count(self):self.data['benchmarks'][0].update(classification='measured',correctness='passed',samples=[1,2],sample_count=3,executed_at='2026-11-16T00:00:00Z',evidence=[EVIDENCE.copy()]);self.rejected()
 def test_not_run_cannot_have_timings(self):self.data['benchmarks'][0].update(samples=[1],sample_count=1);self.rejected()
 def test_historical_summary_does_not_invent_samples(self):self.data['benchmarks'][0].update(classification='historical',sample_count=30,summary={'min':1,'max':3,'mean':2},executed_at='2026-11-15T00:00:00Z',evidence=[EVIDENCE.copy()]);self.assertTrue(validator.validate(self.data))
 def test_anonymous_grants_rejected(self):self.data['observers'][0]['authority']['grant_refs']=['/grant/pretend'];self.rejected()
 def test_purpose_cannot_grant_authority(self):self.data['observers'][0]['authority_from_purpose']=True;self.rejected()
 def test_unbounded_experiment_rejected(self):del self.data['experiments'][0]['inputs'][0]['maximum'];self.rejected()
 def test_local_machine_path_rejected(self):self.data['assets'][0]['uri']='C:/Users/Example/private.10d';self.rejected()
 def test_encoded_traversal_rejected(self):self.data['assets'][0]['uri']='/public/%2e%2e/private';self.rejected()
 def test_raw_receipt_referenced_without_schema_copy(self):a=self.data['assets'][0];a.update(origin='native-production',recipe=SOURCE.copy(),receipt={**EVIDENCE,'kind':'runtime','uri':'/receipts/native.json'});self.assertTrue(validator.validate(self.data))
 def test_review_needs_reviewer(self):self.data['locales'][0]['messages']['exhibit.title']['review_state']='reviewed';self.rejected()
 def test_release_requires_native_q42_evidence(self):self.data['stage']='release';self.rejected()
 def test_source_revision_conflict(self):self.data['sources'].append({**SOURCE,'revision':'conflicting'});self.rejected()
 def test_unsupported_schema_keyword_rejected(self):schema=json.loads((ROOT/'schemas/publication-v1.schema.json').read_text());schema['madeUpConstraint']=True;self.assertRaises(validator.ValidationError,validator.validate,self.data,schema)
if __name__=='__main__':
 if '--write-fixtures' in sys.argv:
  out=ROOT/'contracts/fixtures';out.mkdir(parents=True,exist_ok=True);good=fixture();(out/'valid-draft.json').write_text(json.dumps(good,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');bad=copy.deepcopy(good);bad['test_runs'][0]['status']='passed';(out/'invalid-zero-assertion-pass.json').write_text(json.dumps(bad,ensure_ascii=False,indent=2)+'\n',encoding='utf-8');sys.argv.remove('--write-fixtures')
 unittest.main()
