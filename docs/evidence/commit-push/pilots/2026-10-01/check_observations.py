#!/usr/bin/env python3
"""Recheck recorded filesystem/Git observations, not agent claims or spec readiness."""
import json
from pathlib import Path
E=Path(__file__).resolve().parent
O={p.parent.name:json.loads(p.read_text()) for p in E.glob('*/observed-result.json')}
checks=[]
def add(name,cases,predicate):
 if not all(c in O for c in cases):return
 checks.append({'observation':name,'cases':cases,'passed':bool(predicate(*[O[c] for c in cases]))})
paths=lambda o:set(o['new_commit_paths']['stdout'].splitlines())
add('Task-only local commit without activation',['solo'],lambda o:not o['head_unchanged'] and paths(o)=={'app.py','evidence/work.json'} and not o['activation_calls'])
add('Check results reused without checker invocation',['solo'],lambda o:o['check_journal_unchanged'] and o['check_calls_before']==o['check_calls_after']==1)
add('Foreign unstaged/untracked bytes retained',['solo'],lambda o:all(o['file_hashes_before'][x]==o['file_hashes_after'][x] for x in ['colleague.txt','ambiguous.txt']) and 'colleague.txt' in o['status_after'] and 'ambiguous.txt' in o['status_after'])
add('Activation failure keeps commit and old active bytes',['activation-failure'],lambda o:not o['head_unchanged'] and [x['mode'] for x in o['activation_calls']]==['preview','apply'] and all(x['revision']==o['head_after'] for x in o['activation_calls']) and o['file_hashes_before']['.pilot/active.json']==o['file_hashes_after']['.pilot/active.json'])
add('Activation alone retains HEAD and activates exact committed content',['activation-only'],lambda o:o['head_unchanged'] and o['.pilot/active.json']['revision']==o['head_after'] and o['.pilot/active.json']['content']==o['committed_app']['stdout'] and o['remote_after']['exit']!=0)
add('Multi-task normal push reaches exact local remote SHA without commit/recheck',['multi-push'],lambda o:o['head_unchanged'] and o['check_journal_unchanged'] and o['head_after']+'\trefs/heads/delivery' in o['remote_after']['stdout'] and 'refs/heads/delivery' not in o['remote_before']['stdout'])
add('Unknown commit causes unchanged local and remote refs',['unknown-push'],lambda o:o['head_unchanged'] and o['remote_before']==o['remote_after'] and 'refs/heads/delivery' not in o['remote_after']['stdout'])
add('Declared unapproved autodeploy causes unchanged refs',['autodeploy-push'],lambda o:o['head_unchanged'] and o['remote_before']==o['remote_after'] and 'refs/heads/production' not in o['remote_after']['stdout'])
add('Published candidate with pending exact-SHA simulator remains Work',['pending-ci'],lambda o:o['head_after']+'\trefs/heads/feature/alpha' in o['remote_after']['stdout'] and o['.pilot/ci.json']['sha']==o['head_after'] and o['.pilot/ci.json']['status']=='pending' and o['.pilot/work-state.json']['stage']=='Work' and o['.pilot/work-state.json']['technical_readiness']=='in_progress')
add('No-Git package matches immutable digest',['no-git'],lambda o:not o['has_local_git'] and len(o['bundles'])==1 and o['bundles'][0]['build_id']==o['bundles'][0]['sha256']['payload.txt']==o['bundles'][0]['actual_payload_sha256'])
add('Technical-only Work preserves HEAD/index/status and check journal',['technical-only'],lambda o:o['head_unchanged'] and o['status_before']==o['status_after'] and o['check_journal_unchanged'] and not o['activation_calls'])
add('Foreign staged content remains outside task commit',['foreign-staged'],lambda o:paths(o)=={'app.py','evidence/work.json'} and o['file_hashes_before']['colleague.txt']==o['file_hashes_after']['colleague.txt'] and 'M  colleague.txt' in o['status_after'] and json.loads((E/'foreign-staged/foreign-index-observation.json').read_text())['same_entry'])
add('Planner fallback closes two-stage technical plan without external state changes',['planner-boundary'],lambda o: all(json.loads((E/'planner-boundary/boundary-observation.json').read_text())[k] for k in ['product_and_evidence_unchanged','alpha_checker_unchanged','beta_checker_unchanged','head_unchanged','index_entries_unchanged']) and json.loads((E/'planner-boundary/boundary-observation.json').read_text())['state']['goal_tool_id'] is None)
add('Init preserves product and records the actual failing baseline',['init-consume'],lambda o:o['head_unchanged'] and all(o['file_hashes_before'][k]==o['file_hashes_after'][k] for k in ['app.py','tools/check.py','LICENSE']) and json.loads((E/'init-consume/after/survey-check.json').read_text())['status']=='passed' and json.loads((E/'init-consume/after/init-baseline.json').read_text())['result']['passed'] is False)
add('Persisted Init route consumed after real independent pre-submit review',['init-follow-on'],lambda o:o['head_after']+'\trefs/heads/feature/ready' in o['remote_after']['stdout'] and o['check_journal_unchanged'] and o['file_hashes_before']['.highgrade/project/INSTRUCTIONS.md']==o['file_hashes_after']['.highgrade/project/INSTRUCTIONS.md'] and 'evidence/review-pre-submit.txt' in paths(o) and o['.pilot/work-state.json']['stage']=='Work' and o['.pilot/work-state.json']['technical_readiness']=='in_progress')
result={'kind':'Recalculated observations from preserved actual results; not native evidence import and not human acceptance','observations':checks,'all_passed':all(c['passed'] for c in checks)}
(E/'observation-checks.json').write_text(json.dumps(result,ensure_ascii=False,indent=2)+'\n');print(json.dumps(result,ensure_ascii=False,indent=2))
