import pathlib,subprocess,json,datetime,sys
p=pathlib.Path(__file__).resolve().parents[1]
r=subprocess.run(sys.argv[1:],cwd=p,capture_output=True,text=True)
with (p/".pilot/actions.jsonl").open("a") as f: f.write(json.dumps({"at":datetime.datetime.now(datetime.timezone.utc).isoformat(),"command":sys.argv[1:],"exit_code":r.returncode,"stdout":r.stdout,"stderr":r.stderr},ensure_ascii=False)+"\n")
print(r.stdout,end="");print(r.stderr,end="",file=sys.stderr);sys.exit(r.returncode)
