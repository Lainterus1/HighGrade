#!/usr/bin/env python3
"""Record actual source-guided agent exercise actions, without asserting pass/fail."""
from pathlib import Path
import datetime, hashlib, json, os, shutil, subprocess, sys
REPO=Path('/tmp/highgrade-current-evidence')
EVIDENCE=REPO/'docs/evidence/current-requirements/2026-10-01/init'
WORK=REPO/'target/highgrade/tmp/current-evidence-init'
CLI=Path('/tmp/highgrade-ci-fix/target/debug/highgrade')
def now(): return datetime.datetime.now(datetime.timezone.utc).isoformat()
def sha(path): return hashlib.sha256(Path(path).read_bytes()).hexdigest()
def hashes(root): return {p.relative_to(root).as_posix():sha(p) for p in sorted(root.rglob('*')) if p.is_file()}
def log(event,**kwargs):
 with (EVIDENCE/'actions.jsonl').open('a') as f: f.write(json.dumps({'at':now(),'event':event,**kwargs},ensure_ascii=False)+'\n')
def write(root,relative,text):
 p=root/relative; before=sha(p) if p.exists() else None; p.parent.mkdir(parents=True,exist_ok=True); p.write_text(text,encoding='utf-8');log('write',path=str(p.relative_to(REPO)),before_sha256=before,after_sha256=sha(p),bytes=p.stat().st_size)
def run(case,label,*args,allowed=(0,1,2)):
 root=WORK/case
 cmd=[str(CLI),args[0],'--root',str(root),*map(str,args[1:])]
 return command(case,label,cmd,root,allowed)
def command(case,label,cmd,cwd,allowed=(0,1,2)):
 out=EVIDENCE/case/'raw'; out.mkdir(parents=True,exist_ok=True)
 at=now(); result=subprocess.run(cmd,cwd=cwd,text=True,capture_output=True)
 path=out/(label+'.stdout'); path.write_text(result.stdout,encoding='utf-8'); (out/(label+'.stderr')).write_text(result.stderr,encoding='utf-8')
 log('command',case=case,label=label,started_at=at,command=cmd,cwd=str(cwd),exit_code=result.returncode,stdout=path.relative_to(EVIDENCE).as_posix(),stderr=(out/(label+'.stderr')).relative_to(EVIDENCE).as_posix())
 print(case,label,'exit',result.returncode)
 if result.returncode not in allowed: raise RuntimeError(result.stderr or result.stdout)
 try: return json.loads(result.stdout)
 except json.JSONDecodeError: return result.stdout

def snapshot(case,phase):
 root=WORK/case; dst=EVIDENCE/case/phase
 if dst.exists(): raise RuntimeError('Snapshot already exists')
 shutil.copytree(root,dst)
 manifest=hashes(root); write(EVIDENCE/case,phase+'-sha256.json',json.dumps(manifest,ensure_ascii=False,indent=2)+'\n'); log('snapshot',case=case,phase=phase,files=len(manifest));return manifest

def survey_sha(case,label): return run(case,label,'survey-read','--view','editable')['result']['survey_sha256']
def survey_edit(case,label,patch):
 path=EVIDENCE/case/'inputs'/(label+'.json'); write(EVIDENCE/case,'inputs/'+label+'.json',json.dumps(patch,ensure_ascii=False,indent=2)+'\n')
 relative='.highgrade/local/inputs/'+label+'.json'
 write(WORK/case,relative,path.read_text())
 return run(case,label,'survey-edit','--input',relative,'--expected',survey_sha(case,label+'-read'),allowed=(0,))
