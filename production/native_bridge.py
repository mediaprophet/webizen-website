"""A bounded, private stdio session with the current Synth executable."""
import subprocess,threading,queue,json,os,time
from pathlib import Path
class SynthBridge:
 def __init__(self,root):
  self.root=Path(root);self.output=self.root/'artifacts';self.output.mkdir(exist_ok=True)
  env={**os.environ,'SYNTH_INPUT_ROOT':str(self.root),'SYNTH_OUTPUT_ROOT':str(self.output)}
  self.process=subprocess.Popen(['C:/github/synth/target-prod/debug/synth.exe','mcp'],stdin=subprocess.PIPE,stdout=subprocess.PIPE,stderr=subprocess.DEVNULL,text=True,encoding='utf-8',bufsize=1,env=env,creationflags=getattr(subprocess,'CREATE_NO_WINDOW',0))
  self.lock=threading.Lock();self.responses=queue.Queue();self.serial=0
  threading.Thread(target=self._read,daemon=True).start()
  self.request('initialize',{'protocolVersion':'2024-11-05','capabilities':{},'clientInfo':{'name':'webizen-world-review','version':'2'}})
  self.process.stdin.write(json.dumps({'jsonrpc':'2.0','method':'notifications/initialized'})+'\n');self.process.stdin.flush()
  self.catalog=self.call('synth_operations',{})
 def _read(self):
  for line in self.process.stdout:
   try:self.responses.put(json.loads(line))
   except ValueError:pass
 def request(self,method,params):
  with self.lock:
   self.serial+=1;serial=self.serial
   self.process.stdin.write(json.dumps({'jsonrpc':'2.0','id':serial,'method':method,'params':params})+'\n');self.process.stdin.flush()
   deadline=time.monotonic()+600
   while time.monotonic()<deadline:
    result=self.responses.get(timeout=max(.1,deadline-time.monotonic()))
    if result.get('id')==serial:
     if result.get('error'):raise RuntimeError(str(result['error']))
     return result['result']
   raise TimeoutError('Synth did not return a result within 600 seconds')
 def call(self,name,args):
  result=self.request('tools/call',{'name':name,'arguments':args})
  data=result.get('structuredContent')
  if data is None:
   content=next((x['text'] for x in result.get('content',[]) if x.get('type')=='text'),'{}')
   try:data=json.loads(content)
   except ValueError:data={'text':content}
  if result.get('isError') or data.get('error'):raise RuntimeError(str(data))
  return data
 def produce(self,source,label='work',inputs=None):
  import uuid
  run=label+'-'+uuid.uuid4().hex[:12]
  args={'source':source,'quality_profile':'production16','out_hmc':run+'.hmc','export_dir':run,'receipt_json':run+'.json'}
  if inputs:args['inputs']=inputs
  data=self.call('synth_produce_present',args)
  for r in data.get('receipt',{}).get('resources',[]):
   r['url']='/artifacts/'+run+'/'+r['key']
  data['package_url']='/artifacts/'+run+'.hmc';data['receipt_url']='/artifacts/'+run+'.json'
  return data
 def close(self):
  self.process.terminate()
