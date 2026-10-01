#!/usr/bin/env python3
"""Independent fixture observations; requires actual before/after snapshots."""
from pathlib import Path
import json,subprocess,hashlib
E=Path(__file__).resolve().parent;ROOT=E.parents[4];BASE=ROOT/'target/highgrade/tmp/portable-pilot-20261001'
def load(p):return json.loads(p.read_text())
def call(c,r):
 p=subprocess.run(c,cwd=r,text=True,capture_output=True);return {'exit':p.returncode,'stdout':p.stdout,'stderr':p.stderr}
def observe(name):
 folder=E/name
 if not (folder/'after/observation.json').exists():return None
 a=load(folder/'before/observation.json');b=load(folder/'after/observation.json');r=BASE/name
 o={'fixture':name,'check_calls_before':len(a['test_calls']),'check_calls_after':len(b['test_calls']),'check_journal_unchanged':a['test_calls']==b['test_calls'],'activation_calls':b['activation_calls'],'file_hashes_before':a['files'],'file_hashes_after':b['files']}
 if 'head' in a:
  old=a['head']['stdout'].strip();new=b['head']['stdout'].strip();o.update(head_before=old,head_after=new,head_unchanged=old==new,status_before=a['status']['stdout'],status_after=b['status']['stdout'],remote_before=a['remote'],remote_after=b['remote'],new_commits=call(['git','log','--format=%H %s',old+'..'+new],r),new_commit_paths=call(['git','diff','--name-only',old,new],r),committed_app=call(['git','show',new+':app.py'],r))
 for rel in ['.pilot/active.json','.pilot/ci.json','.pilot/work-state.json']:
  if (r/rel).is_file():o[rel]=load(r/rel)
 resp=r/('.pilot/response.txt' if name!='no-git' else 'response.txt')
 if resp.exists():o.update(response=resp.read_text(),response_words=len(resp.read_text().split()))
 if name=='no-git':
  o['has_local_git']=(r/'.git').exists();o['bundles']=[]
  for manifest in r.glob('artifacts/*/manifest.json'):
   data=load(manifest);data['actual_payload_sha256']=hashlib.sha256((manifest.parent/'payload.txt').read_bytes()).hexdigest();o['bundles'].append(data)
 (folder/'observed-result.json').write_text(json.dumps(o,ensure_ascii=False,indent=2)+'\n');return o
NAMES=['solo','activation-failure','activation-only','multi-push','unknown-push','autodeploy-push','pending-ci','no-git','technical-only','planner-boundary','foreign-staged','init-consume','init-follow-on']
all_results=[o for name in NAMES if (o:=observe(name))]
(E/'observed-results.json').write_text(json.dumps(all_results,ensure_ascii=False,indent=2)+'\n')
print(json.dumps([{'fixture':o['fixture'],'head_after':o.get('head_after'),'check_calls':[o['check_calls_before'],o['check_calls_after']],'response_words':o.get('response_words')} for o in all_results],indent=2))
