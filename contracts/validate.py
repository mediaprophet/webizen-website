"""Strict validation for this schema's documented JSON Schema subset and semantics.

No network/schema loading, plugins or non-standard Python dependencies are used.
Native receipts are immutable referenced evidence, not parsed by this validator.
"""
import argparse,datetime,json,math,re,sys
from pathlib import Path
from urllib.parse import urlsplit,unquote
ROOT=Path(__file__).resolve().parents[1]
SUPPORTED={'$schema','$id','title','$ref','$defs','type','properties','required','additionalProperties','items','minItems','maxItems','minimum','maximum','minLength','maxLength','pattern','format','const','enum','anyOf'}
class ValidationError(ValueError):pass
def fail(path,message):raise ValidationError(path+': '+message)
def check_schema(schema,path='$schema'):
 unknown=set(schema)-SUPPORTED
 if unknown:fail(path,'unsupported schema keywords '+','.join(sorted(unknown)))
 for key in ('properties','$defs'):
  for name,value in schema.get(key,{}).items():check_schema(value,path+'.'+key+'.'+name)
 for key in ('items','additionalProperties'):
  if isinstance(schema.get(key),dict):check_schema(schema[key],path+'.'+key)
 for index,value in enumerate(schema.get('anyOf',[])):check_schema(value,path+'.anyOf['+str(index)+']')
def public_reference(value):
 parsed=urlsplit(value)
 if parsed.scheme and (parsed.scheme!='https' or not parsed.netloc):return False
 if not parsed.scheme and (parsed.netloc or value.startswith('//')):return False
 if '\\' in value or any(ord(c)<32 for c in value):return False
 if parsed.username or parsed.password:return False
 return all(part!='..' for part in unquote(parsed.path).split('/'))
def validate_value(value,schema,document,path='$'):
 if '$ref' in schema:
  ref=schema['$ref']
  if not ref.startswith('#/$defs/'):fail(path,'external schema references are not supported')
  try:target=document['$defs'][ref[8:]]
  except KeyError:fail(path,'unknown schema reference')
  return validate_value(value,target,document,path)
 if 'anyOf' in schema:
  errors=[]
  for branch in schema['anyOf']:
   try:validate_value(value,branch,document,path);return
   except ValidationError as error:errors.append(str(error))
  fail(path,'no allowed form matched ('+'; '.join(errors)+')')
 if 'const' in schema and (type(value)!=type(schema['const']) or value!=schema['const']):fail(path,'incorrect constant')
 if 'enum' in schema and not any(type(value)==type(x) and value==x for x in schema['enum']):fail(path,'unknown enum value')
 kind=schema.get('type')
 correct={'object':isinstance(value,dict),'array':isinstance(value,list),'string':isinstance(value,str),'integer':type(value)==int,'number':type(value) in (int,float) and math.isfinite(value),'boolean':type(value)==bool,'null':value is None}
 if kind and not correct.get(kind,False):fail(path,'expected '+kind)
 if kind=='object':
  for required in schema.get('required',[]):
   if required not in value:fail(path,'missing '+required)
  for key,item in value.items():
   prop=schema.get('properties',{}).get(key,schema.get('additionalProperties',True))
   if prop is False:fail(path,'unknown property '+key)
   if isinstance(prop,dict):validate_value(item,prop,document,path+'.'+key)
 if kind=='array':
  if len(value)<schema.get('minItems',0) or len(value)>schema.get('maxItems',math.inf):fail(path,'array length outside bounds')
  for i,item in enumerate(value):validate_value(item,schema.get('items',{}),document,path+'['+str(i)+']')
 if kind=='string':
  if len(value)<schema.get('minLength',0) or len(value)>schema.get('maxLength',math.inf):fail(path,'text length outside bounds')
  if 'pattern' in schema and not re.search(schema['pattern'],value):fail(path,'text does not match pattern')
  form=schema.get('format')
  if form=='public-reference' and not public_reference(value):fail(path,'unsafe/publicly invalid reference')
  if form=='repository-path' and (not value or value.startswith('/') or ':' in value or '\\' in value or '..' in value.split('/')):fail(path,'expected repository-relative path')
  if form=='date-time':
   try:
    date=datetime.datetime.fromisoformat(value.replace('Z','+00:00'))
    if date.tzinfo is None:raise ValueError()
   except ValueError:fail(path,'expected timezone-qualified ISO timestamp')
 if kind in ('integer','number') and (value<schema.get('minimum',-math.inf) or value>schema.get('maximum',math.inf)):fail(path,'number outside bounds')
def semantic_checks(bundle):
 collections=['modules','exhibits','experiments','test_runs','benchmarks','locales','assets','observers','migrations','runtimes']
 indexes={}
 for collection in collections:
  rows=bundle[collection];ids=[x['id'] for x in rows]
  if len(ids)!=len(set(ids)):fail(collection,'duplicate stable IDs')
  indexes[collection]=set(ids)
 def refs(row,field,target):
  values=row.get(field,[])
  if values is None:return
  if not isinstance(values,list):values=[values]
  for value in values:
   if value not in indexes[target]:fail(row['id']+'.'+field,'unresolved '+target+' reference '+value)
 for row in bundle['modules']:
  refs(row,'related_modules','modules');refs(row,'runtime_ids','runtimes')
  if row['qualification']=='runtime-verified' and not any(e['kind']=='runtime' for e in row['evidence']):fail(row['id'],'runtime verification needs runtime evidence')
 for row in bundle['exhibits']:
  for field,target in [('module_ids','modules'),('experiment_id','experiments'),('asset_ids','assets'),('related_exhibits','exhibits')]:refs(row,field,target)
 for row in bundle['experiments']:
  refs(row,'module_ids','modules');refs(row,'runtime_ids','runtimes')
  ids=[i['id'] for i in row['inputs']]
  if len(ids)!=len(set(ids)):fail(row['id'],'duplicate input IDs')
  for item in row['inputs']:
   if item['type'] in ('number','integer') and ('minimum' not in item or 'maximum' not in item or item['minimum']>item['maximum']):fail(row['id'],'numeric inputs require ordered bounds')
   if item['type']=='select' and not item.get('options'):fail(row['id'],'select requires allowed options')
   if item['type']=='string' and 'max_length' not in item:fail(row['id'],'string input requires a byte-admission companion and character bound')
 for row in bundle['test_runs']:
  refs(row,'runtime_id','runtimes')
  if row['status']=='passed' and row['assertion_count']==0:fail(row['id'],'zero assertions cannot pass')
  if row['status'] in ('passed','failed') and (not row['executed_at'] or not row['evidence']):fail(row['id'],'executed outcome needs timestamp and evidence')
  if row['status'] in ('skipped','not-run','unsupported') and not row['reason']:fail(row['id'],'non-execution needs reason')
 for row in bundle['benchmarks']:
  refs(row,'runtime_id','runtimes')
  if row['classification']=='measured' and row['correctness']!='passed':fail(row['id'],'measured timing requires correctness pass')
  if row['samples'] is not None and len(row['samples'])!=row['sample_count']:fail(row['id'],'sample count mismatch')
  if row['classification']=='not-run' and (row['sample_count'] or row['samples'] or row['summary']):fail(row['id'],'not-run cannot contain timings')
  if row['classification']!='not-run' and (not row['executed_at'] or not row['evidence']):fail(row['id'],'timing needs dated evidence')
  if row['classification']=='measured' and (row['samples'] is None or not row['samples']):fail(row['id'],'new measured run requires raw samples')
  summary=row['summary']
  if summary and not summary['min']<=summary['mean']<=summary['max']:fail(row['id'],'inconsistent summary bounds')
 locales=[row['locale'] for row in bundle['locales']]
 if len(locales)!=len(set(locales)):fail('locales','duplicate locale tags')
 defaults=[row for row in bundle['locales'] if row['default']]
 if len(defaults)!=1 or defaults[0]['locale']!='en-AU':fail('locales','exactly one en-AU default is required')
 for row in bundle['locales']:
  if row['locale'] in row['fallbacks']:fail(row['id'],'self-referential locale fallback')
  for key,message in row['messages'].items():
   if message['review_state']=='reviewed' and not message['reviewer']:fail(row['id']+'.'+key,'reviewed translation needs reviewer')
 for row in bundle['assets']:
  if row['origin']=='native-production' and (not row['recipe'] or not row['receipt'] or row['receipt']['kind']!='runtime'):fail(row['id'],'native production needs recipe and native runtime receipt')
  if row['visual_acceptance']=='accepted' and not any(e['kind']=='visual-review' for e in row['evidence']):fail(row['id'],'visual acceptance needs review evidence')
 for row in bundle['observers']:
  refs(row,'selected_exhibits','exhibits');authority=row['authority']
  if authority['status']=='anonymous' and authority['grant_refs']:fail(row['id'],'anonymous cannot carry authority grants')
  if authority['status']=='verified' and (not authority['grant_refs'] or not any(e['kind']=='runtime' for e in authority['evidence'])):fail(row['id'],'verified authority requires host runtime evidence and grants')
 for row in bundle['migrations']:
  refs(row,'module_ids','modules');refs(row,'experiment_ids','experiments')
  if row['disposition']!='unavailable' and not row['destination']:fail(row['id'],'migration needs a destination')
  if row['disposition'] in ('unavailable','archive') and not row['reason']:fail(row['id'],'archive/unavailability needs reason')
  if row['qualification']!='source-only' and not row['evidence']:fail(row['id'],'qualified migration needs evidence')
 # One repository may contribute many source files, but a release cannot mix revisions.
 revisions={}
 for source in bundle['sources']:
  if source['repository'] in revisions and revisions[source['repository']]!=source['revision']:fail('sources','conflicting repository revisions')
  revisions[source['repository']]=source['revision']
 if bundle['stage']=='release':
  if not bundle['graph']:fail('graph','release requires genuine q42 reader round-trip evidence')
  if any(row['qualification']=='source-only' for row in bundle['migrations']):fail('migrations','release has unqualified legacy routes')
  if any(e['kind']=='test-fixture' for group in collections for row in bundle[group] for e in row.get('evidence',[])):fail('release','test fixtures cannot substantiate publication claims')
def validate(bundle,schema=None):
 schema=schema or json.loads((ROOT/'schemas/publication-v1.schema.json').read_text(encoding='utf-8'))
 check_schema(schema);validate_value(bundle,schema,schema);semantic_checks(bundle);return True
def main():
 parser=argparse.ArgumentParser(description=__doc__);parser.add_argument('publication',type=Path);args=parser.parse_args()
 try:
  validate(json.loads(args.publication.read_text(encoding='utf-8')))
  print(json.dumps({'valid':True,'schema':'webizen.publication/v1','file':str(args.publication)}));return 0
 except (ValidationError,ValueError,OSError) as error:
  print(json.dumps({'valid':False,'error':str(error)}));return 1
if __name__=='__main__':sys.exit(main())
