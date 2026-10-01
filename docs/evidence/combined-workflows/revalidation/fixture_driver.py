#!/usr/bin/env python3
"""New isolated run of historical fixture constructor; never writes historical evidence."""
import importlib.util, pathlib, sys, json
R=pathlib.Path(__file__).resolve().parents[4]
S=R/'docs/evidence/commit-push/pilots/2026-10-01/fixture_harness.py'
spec=importlib.util.spec_from_file_location('combined_fixture',S);h=importlib.util.module_from_spec(spec);spec.loader.exec_module(h)
h.BASE=R/'target/highgrade/tmp/combined-revalidation-20261001'
h.EVID=pathlib.Path(__file__).resolve().parent/'pilots';h.PROFILE=h.BASE/'profile'
if __name__=='__main__':
 if sys.argv[1]=='prepare':
  h.BASE.mkdir(parents=True,exist_ok=False);h.EVID.mkdir(exist_ok=False);h.prepare()
 elif sys.argv[1]=='snapshot': print(json.dumps(h.snapshot(sys.argv[2],sys.argv[3]),ensure_ascii=False,indent=2))
