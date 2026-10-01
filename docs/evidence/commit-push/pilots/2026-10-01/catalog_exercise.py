import json,subprocess,pathlib,hashlib,shutil
E=pathlib.Path(__file__).resolve().parent;R=E.parents[4];B=R/'target/highgrade/tmp/portable-pilot-20261001';P=B/'catalog-survival';EXE=B/'profile-frozen/.highgrade/global/releases/v0-3-6/highgrade';O=E/'catalog-survival';O.mkdir(exist_ok=True)
(P/'.highgrade/project').mkdir(parents=True,exist_ok=True)
(P/'.highgrade/project/INSTRUCTIONS.md').write_text('---\nhighgrade_project_schema: 1\nhighgrade_spec_format: native-v1\n---\n# Независимый каталог\n')
def call(name,args):
 p=subprocess.run([str(EXE),*args,'--root',str(P)],text=True,capture_output=True); (O/(name+'.json')).write_text(p.stdout);(O/(name+'.command.json')).write_text(json.dumps({'argv':p.args,'exit':p.returncode,'stderr':p.stderr},indent=2)+'\n');return json.loads(p.stdout)
def val(v,key):
 if key in v.get('result',{}):return v['result'][key]
 for m in v.get('measurements',[]):
  if key in m:return m[key]
 raise KeyError((key,v))
v=call('01-init',['spec-init']);v=call('02-list-empty',['spec-list']);print(json.dumps(v,ensure_ascii=False))
h=val(v,'store_sha256');v=call('03-create',['spec-new','--title','Проверяемая автономность каталога','--expected',h]);print(json.dumps(v,ensure_ascii=False))
v=call('04-read-draft',['spec-read','--id','HG-0001','--view','editable']);print(json.dumps(v,ensure_ascii=False))
patch={'goal':'Проверить автономность данных проекта от служебной конфигурации процесса','rationale':'Изолированное упражнение установленной CLI без исходников Поставки','scope':'Только синтетический локальный каталог','operations':[{'action':'add','requirement':{'id':'HG-0001-R1','title':'Автономный каталог','statement':'Каталог проекта SHALL сохранять данные после удаления служебной конфигурации.','scenarios':[{'id':'HG-0001-S1','given':'В specs/ сохранена спецификация','when':'Служебная конфигурация удалена из проекта','then':'spec-read возвращает прежнюю спецификацию без потери данных','verification':'Фактические CLI-команды и хеши всех файлов каталога'}]}}],'tasks':[{'id':'HG-0001-T1','description':'Проверить чтение каталога после удаления конфигурации','done':False}]}
(O/'patch.json').write_text(json.dumps(patch,ensure_ascii=False,indent=2)+'\n')
(P/'patch.json').write_bytes((O/'patch.json').read_bytes())
v=call('05-edit',['spec-edit','--id','HG-0001','--input','patch.json','--expected',val(v,'store_sha256'),'--validate','true'])
call('06-find',['spec-list','--id','HG-0001'])
call('07-validate',['spec-validate','--id','HG-0001'])
a=call('08-check',['spec-check','--id','HG-0001','--brief','true'])
b=call('09-recheck',['spec-check','--id','HG-0001','--brief','true'])
def hashes():return {str(p.relative_to(P)):hashlib.sha256(p.read_bytes()).hexdigest() for p in sorted((P/'specs').rglob('*')) if p.is_file()}
before=hashes();(O/'catalog-before.json').write_text(json.dumps(before,indent=2)+'\n')
shutil.copytree(P/'specs',O/'catalog-snapshot')
shutil.move(str(P/'.highgrade'),str(B/'catalog-survival-removed-service-config'))
(O/'10-remove-service-config.command.json').write_text(json.dumps({'action':'move complete .highgrade service configuration out of project','source':str(P/'.highgrade'),'destination':str(B/'catalog-survival-removed-service-config'),'project_config_exists_after':(P/'.highgrade').exists()},indent=2)+'\n')
v=call('11-read-without-service-config',['spec-read','--id','HG-0001','--view','editable'])
after=hashes();(O/'catalog-after.json').write_text(json.dumps(after,indent=2)+'\n')
summary={'created_and_found':json.loads((O/'03-create.json').read_text())['status']=='passed' and 'HG-0001' in (O/'06-find.json').read_text(),'validate_status':json.loads((O/'07-validate.json').read_text())['status'],'check_status':a['status'],'recheck_status':b['status'],'repeat_equal':a==b,'read_without_config_status':v['status'],'catalog_bytes_equal':before==after,'project_configuration_absent':not(P/'.highgrade/project').exists(),'regenerated_service_files':[str(p.relative_to(P)) for p in (P/'.highgrade').rglob('*') if p.is_file()],'acceptance_not_claimed':True,'readiness_not_claimed':True}
(O/'summary.json').write_text(json.dumps(summary,indent=2)+'\n');print(json.dumps(summary,indent=2))
