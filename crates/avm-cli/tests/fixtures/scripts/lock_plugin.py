#!/usr/bin/env python3
import hashlib, json, os, pathlib, sys
tool = 'TOOL'
root = pathlib.Path(os.environ['HOME']).parent
cmd, *args = sys.argv[1:]
def version():
    return root.joinpath('remote').read_text() if tool == 'node' else 'openjdk-21.1.0+1'
def target(v):
    return root / 'home/.avm/tools' / tool / v / 'bin' / tool
def install(v):
    with root.joinpath('installs').open('a') as f: f.write(tool + ':' + v + '\n')
    path = target(v)
    path.parent.mkdir(parents=True, exist_ok=True)
    path.write_text('#!/bin/sh\necho ' + v + '\n')
    path.chmod(0o755)
if cmd == 'manifest':
    print(json.dumps(dict(name=tool, version='1.0.0', api_version=1)))
elif cmd == 'versions':
    print(json.dumps(dict(versions=[dict(version=version(), label=version(), channel=None, is_lts=True, is_security=False)])))
elif cmd == 'resolve':
    req = json.loads(args[0])
    asset = dict(url=str(root / 'archive'), sha256=hashlib.sha256(root.joinpath('archive').read_bytes()).hexdigest())
    print(json.dumps(dict(version=req['version'], assets={p: asset for p in req['platforms']})))
elif cmd == 'is-installed':
    print(json.dumps(dict(installed=target(args[0]).exists())))
elif cmd == 'installed-versions':
    print('{"versions":[]}')
elif cmd == 'executable-path':
    print(json.dumps(dict(path=str(target(args[0])) if target(args[0]).exists() else None)))
elif cmd == 'env-vars':
    print('{"env":{}}')
elif cmd == 'install':
    install(args[0])
elif cmd == 'install-locked':
    asset = json.loads(args[1])
    assert asset['url'] == str(root / 'archive'), 'host must pass the original asset'
    if hashlib.sha256(pathlib.Path(asset['url']).read_bytes()).hexdigest() != asset['sha256']:
        sys.exit('checksum mismatch')
    install(args[0])
else:
    sys.exit(1)
