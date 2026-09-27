"""Bounded CLI data fixtures; no application, agent exercise, or project runner."""
import hashlib,json,subprocess,tempfile,shutil
from pathlib import Path
ROOT=Path(__file__).resolve().parents[3]
EXE=ROOT/'target/debug/highgrade.exe'
OUT=Path(__file__).parent
rows=[]
def call(root,op,allow=(0,),**kw):
 p=subprocess.run([str(EXE),op,'--root',str(root),*[v for k,x in kw.items() for v in (['--bootstrap'] if k=='bootstrap' else ['--'+k.replace('_','-'),str(x)])]],capture_output=True)
 r=json.loads(p.stdout);assert p.returncode in allow,(op,p.returncode,r)
 rows.append({'case':root.name,'operation':op,'exit_code':p.returncode,'status':r['status'],'findings':r['findings'],'result':r.get('result',{})})
 return r['result']
def put(root,name,data):
 (root/name).write_text(json.dumps(data,ensure_ascii=False),encoding='utf-8');return name
def survey_sha(root):return call(root,'survey-read')['survey_sha256']
def edit(root,patch):return call(root,'survey-edit',expected=survey_sha(root),input=put(root,'patch.json',patch))
def review(root):
 call(root,'survey-snapshot',expected=survey_sha(root))
 call(root,'survey-review',expected=survey_sha(root),verdict='go',reviewer='fixture observer',conclusion='Only structural fixture states checked; no product or user acceptance')
base=ROOT/'target/highgrade/tmp';base.mkdir(parents=True,exist_ok=True)
owned=Path(tempfile.mkdtemp(prefix='init-routes-',dir=base))
try:
 for case in ['empty','template']:
  root=owned/case;root.mkdir()
  if case=='template':(root/'README.md').write_text('Existing template sentinel\n',encoding='utf-8')
  sentinel=(root/'README.md').read_bytes() if case=='template' else None
  inventory=call(root,'inventory',mode='structure')
  bootstrap=call(root,'inspect',allow=(0,1,2),bootstrap='true')
  assert bootstrap['validation']=='passed' and bootstrap['adaptation']=='required'
  call(root,'survey-new')
  content=call(root,'survey-read',view='editable')['value']
  content['goal']='Prepare repository and initial requirements, without product implementation'
  for d in content['decisions']:
   d.update(state='confirmed',description='Fixture decision: '+d['id'],basis=['Explicit synthetic fixture mandate: preparation only'],blocks=[])
  for name in content['sections']:
   content['sections'][name]=[{'id':name+'-state','state':'confirmed','description':'Observed absence at this stage; no implementation claimed','basis':['Inventory of bounded fixture'],'sources':[],'blocks':[]}]
  content['sections']['adaptation'][0].update(description='Decision: define a future component; not yet implemented',basis=['Synthetic design decision'])
  content['sections']['findings'][0].update(state='unknown',description='Future product detail belongs to the draft specification; outside completed preparation',basis=[])
  content['source_paths']=['README.md'] if case=='template' else []
  content['areas']=[]
  edit(root,content);review(root)
  initial=call(root,'survey-check');assert initial['plan_ready'] and initial['adaptation_ready']
  edit(root,{'stage':'adapting'})
  edit(root,{'goal':'Preparation refined; actual stage preserved'})
  stale=call(root,'survey-check',allow=(2,));assert not stale['review_record_current']
  assert call(root,'survey-read')['value']['stage']=='adapting'
  # No product application is created: only a native catalog and contract data.
  init=call(root,'spec-init');new=call(root,'spec-new',title='Initial draft result',expected=init['store_sha256'])
  ident=new['change']['id']
  patch={'goal':'Describe the first result','rationale':'Fixture for postponed elaboration','scope':'Initial bounded result','questions':['Clarify an implementation detail before work'],'tasks':[{'id':ident+'-T1','description':'Clarify the supported input format','done':False},{'id':ident+'-T2','description':'Implement reading the agreed source','done':False}], 'operations':[{'action':'add','requirement':{'id':ident+'-R1','title':'Read selected source','statement':'The selected supported source is read without changing its bytes. Supported formats remain an explicit open question.','scenarios':[{'id':ident+'-S1','given':'A selected supported source and its original bytes','when':'The source is read','then':'Its content is returned and original bytes stay unchanged','verification':'A future test compares content and source bytes for the agreed format.'}]}}]}
  call(root,'spec-edit',id=ident,expected=new['store_sha256'],input=put(root,'spec-patch.json',patch))
  call(root,'spec-validate',id=ident,allow=(1,))
  assert {f['code'] for f in rows[-1]['findings']}=={'OpenQuestions'}
  draft=call(root,'spec-check',id=ident,brief='true',allow=(0,1,2));assert draft['brief']['technical']!='ready'
  # Missing implementation data never becomes a fictitious source snapshot.
  review(root);edit(root,{'stage':'completed','next':''})
  assert call(root,'survey-check')['completion_ready']
  summary=call(root,'spec-read',id=ident,view='summary')
  assert summary['value']['questions']
  resumed=call(root,'spec-edit',id=ident,expected=summary['store_sha256'],input=put(root,'resume.json',{'questions':['Choose the exact text encoding before work']}))
  assert resumed['change']['id']==ident and len(resumed['change']['tasks'])==2
  if sentinel is not None:assert (root/'README.md').read_bytes()==sentinel
  assert not (root/'src').exists() and not (root/'.git').exists()
  print(case,'survey complete, product draft not ready, no product created',flush=True)
 (OUT/'cli-observations.json').write_text(json.dumps({'status':'passed','cli_sha256':hashlib.sha256(EXE.read_bytes()).hexdigest(),'limit':'Data fixtures establish representability only, not agent behavior or product readiness','operations':rows},ensure_ascii=False,indent=2)+'\n',encoding='utf-8')
finally:
 assert owned.resolve().parent==base.resolve()
 shutil.rmtree(owned)
