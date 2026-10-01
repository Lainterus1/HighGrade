#!/usr/bin/env python3
"""Read-only checks of retained combined-run artifacts and new real fixture states."""
from pathlib import Path
import json, hashlib, datetime, xml.etree.ElementTree as ET, re, subprocess
R=Path(__file__).resolve().parents[4];E=Path(__file__).resolve().parent;B=R/'target/highgrade/tmp/combined-revalidation-20261001'
def load(p):return json.loads(p.read_text())
def sha(p):return hashlib.sha256(p.read_bytes()).hexdigest()
def save(n,x):(E/n).write_text(json.dumps(x,ensure_ascii=False,indent=2)+'\n')
def git(n,*args):return subprocess.check_output(['git',*args],cwd=B/n,text=True).strip()
now=datetime.datetime.now(datetime.timezone.utc).isoformat()
a=load(E/'native/attempt.json');c=load(E/'native/completion.json');tests=ET.parse(E/'native/junit.xml').findall('.//testcase')
native={'checked_at':now,'native_captured_at':a['captured_at'],'attempt_id':a['attempt_id'],'completion_matches':a['attempt_id']==c['attempt_id'] and c['state']=='completed','report_hash_matches':sha(E/'native/junit.xml')==c['report_sha256'],'inventory_hash_matches':sha(E/'native/list.json')==c['inventory_sha256'],'source_files':len(a['source_hashes']),'source_mismatches':[p for p,h in a['source_hashes'].items() if not(R/p).is_file() or sha(R/p)!=h],'testcases':len(tests),'failed_error_skipped':[t.attrib for t in tests if any(t.find(n) is not None for n in ['failure','error','skipped'])],'scenarios':{}}
wanted={'HG-0010-S1','HG-0022-S4','HG-0027-S1','HG-0027-S2','HG-0027-S3','HG-0063-S8','HG-0063-S9','HG-0063-S10','HG-0063-S11'}
for path in sorted((R/'tests').glob('*.rs')):
 ids=[]
 for line in path.read_text().splitlines():
  if '// highgrade:' in line:ids += re.findall(r'HG-\d+-S\d+',line)
  m=re.match(r'fn (\w+)\(',line)
  if m:
   for sid in set(ids)&wanted:
    matches=[t for t in tests if t.get('name')==m[1]]
    native['scenarios'].setdefault(sid,[]).append({'test':m[1],'source':str(path.relative_to(R)),'matches':len(matches),'passed':len(matches)==1 and not any(matches[0].find(n) is not None for n in ['failure','error','skipped'])})
   ids=[]
assert native['completion_matches'] and native['report_hash_matches'] and native['inventory_hash_matches'] and not native['source_mismatches'] and not native['failed_error_skipped'] and len(tests)==211
assert set(native['scenarios'])==wanted and all(t['passed'] for ts in native['scenarios'].values() for t in ts)
save('native-observations.json',native)
obs={'captured_at':now,'method':'informed continuing agent; direct state/action observations, not independent or clean-context','cases':{}}
for n in ['solo','activation-failure','activation-only','multi-push','unknown-push','autodeploy-push','pending-ci','technical-only','foreign-staged']:
 before=load(E/'pilots'/n/'before/observation.json');after=load(E/'pilots'/n/'after/observation.json');actions=[json.loads(x) for x in (B/n/'.pilot/actions.jsonl').read_text().splitlines()];head=after['head']['stdout'].strip();response=(B/n/'.pilot/response.txt').read_text();checks={'tests_not_rerun':len(before['test_calls'])==len(after['test_calls'])==1,'response_saved':bool(response.strip()),'checked_inputs_preserved':all(after['files'][p]==h for p,h in before['test_calls'][0]['inputs'].items())}
 if n in ['solo','activation-failure','pending-ci','foreign-staged']:checks['exact_commit_paths']=set(git(n,'diff-tree','--no-commit-id','--name-only','-r','HEAD').splitlines())=={'app.py','evidence/work.json'}
 if n=='solo':checks.update({'unrelated_bytes_preserved':all(before['files'][p]==after['files'][p] for p in ['colleague.txt','ambiguous.txt']),'no_activation':not after['activation_calls'],'no_remote':not git(n,'remote')})
 if n=='foreign-staged':checks.update({'foreign_staged_diff_preserved':before['staged']['stdout']==after['staged']['stdout'],'foreign_bytes_preserved':before['files']['colleague.txt']==after['files']['colleague.txt']})
 if n=='activation-failure':checks.update({'injected_error_observed':any(x['returncode']==23 for x in actions),'old_active_observed':load(B/n/'.pilot/active.json')['revision']=='old-build'})
 if n=='activation-only':checks.update({'existing_commit_preserved':before['head']==after['head'],'active_exact_sha':load(B/n/'.pilot/active.json')['revision']==head,'activation_source_exact_sha':all(x['revision']==head and x['source']=='git-show-exact-sha' for x in after['activation_calls'])})
 if n in ['multi-push','pending-ci']:
  branch='delivery' if n=='multi-push' else 'feature/alpha';checks['remote_exact_sha']=git(n,'ls-remote','origin','refs/heads/'+branch).split()[0]==head
 if n in ['unknown-push','autodeploy-push']:checks.update({'remote_unchanged':before['remote']==after['remote'],'head_unchanged':before['head']==after['head'],'no_push_action':not any(re.search(r'(^|[; ])git push(?: |$)',' '.join(x['argv'])) for x in actions)})
 if n=='pending-ci':checks.update({'ci_exact_sha_pending':load(B/n/'.pilot/ci.json')=={'status':'pending','sha':head,'source':'local CI simulator'},'work_remains_open':load(B/n/'.pilot/work-state.json')['status']=='in_progress'})
 if n=='technical-only':checks.update({'head_unchanged':before['head']==after['head'],'index_and_product_unchanged':before['staged']==after['staged'] and before['files']['app.py']==after['files']['app.py'],'no_activation':not after['activation_calls'],'no_goal_id':load(B/n/'.pilot/work-state.json')['goal_tool_id'] is None})
 assert all(checks.values()),(n,checks)
 obs['cases'][n]={'checks':checks,'outcome':'passed','head':head,'command_count':len(actions),'response':response,'report':str((E/'pilots'/n/'after/observation.json').relative_to(R))}
cat=load(E/'catalog-survival/summary.json');assert cat['created_and_found'] and cat['repeat_equal'] and cat['catalog_bytes_equal'] and cat['project_configuration_absent'] and cat['validate_status']=='passed' and cat['read_without_config_status']=='passed' and cat['check_status']==cat['recheck_status']=='failed'
obs['catalog_survival']=cat
save('direct-observations.json',obs)
source_before=load(E/'inputs-before.json');pilot_before=load(E/'pilot-inputs-before.json')
files=dict(source_before['files']);files.update(pilot_before['files'])
check={'captured_at':now,'source_count':len(files),'mismatches':[p for p,h in files.items() if not(R/p).is_file() or sha(R/p)!=h]}
assert not check['mismatches'],check
save('inputs-after.json',check)
print(json.dumps({'native':len(tests),'direct_cases':len(obs['cases']),'catalog_survival':'passed','source_mismatches':check['mismatches']},ensure_ascii=False))
