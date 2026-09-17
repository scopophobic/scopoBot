"""Package locally built binaries; does not publish, install, or sign for distribution."""
from pathlib import Path
import os
import hashlib
import platform
import plistlib
import shutil
import subprocess
import sys
import zipfile

root = Path(__file__).resolve().parents[1]
target = os.environ.get('CARGO_BUILD_TARGET')
release = root / 'target' / (target or '') / 'release'
dist = root / 'dist'
dist.mkdir(exist_ok=True)
is_windows = sys.platform == 'win32' or (target and 'windows' in target)
if sys.platform == 'darwin' and not is_windows:
    bundle = dist / 'Scopobot.app' / 'Contents'
    (bundle / 'MacOS').mkdir(parents=True, exist_ok=True)
    (bundle / 'Resources').mkdir(exist_ok=True)
    shutil.copy2(release / 'scopobot', bundle / 'MacOS' / 'scopobot')
    shutil.copy2(release / 'scopobot-terminal', bundle / 'MacOS' / 'scopobot-terminal')
    script = bundle / 'Resources' / 'StartTerminal.command'
    script.write_text('#!/bin/zsh\nexec "${0:A:h}/../MacOS/scopobot-terminal" "$@"\n')
    script.chmod(0o755)
    for name in ['README.md', 'LICENSE']:
        shutil.copy2(root / name, bundle / 'Resources' / name)
    shutil.copytree(root / 'characters', bundle / 'Resources' / 'characters', dirs_exist_ok=True)
    plist = {'CFBundleName':'Scopobot','CFBundleDisplayName':'Scopobot','CFBundleIdentifier':'dev.scopobot.companion',
             'CFBundleExecutable':'scopobot','CFBundlePackageType':'APPL','CFBundleShortVersionString':'0.1.0',
             'CFBundleVersion':'1','LSMinimumSystemVersion':'12.0','LSUIElement':True,
             'NSHighResolutionCapable':True,
             'NSBluetoothAlwaysUsageDescription':'Scopobot checks generic connection categories of paired accessories. It never reads device names or content.'}
    with (bundle / 'Info.plist').open('wb') as f:
        plistlib.dump(plist, f)
    shutil.copy2(release / 'scopobot-terminal', dist / 'scopobot-terminal')
    shutil.copytree(root / 'characters', dist / 'characters', dirs_exist_ok=True)
    subprocess.run(['xattr','-cr',str(bundle.parent)],check=True)
    # File-provider folders can retain FinderInfo on the bundle root after a recursive clear.
    subprocess.run(['xattr', '-d', 'com.apple.FinderInfo', str(bundle.parent)], stderr=subprocess.DEVNULL)
    subprocess.run(['codesign','--force','--deep','--sign','-',str(bundle.parent)], check=True)
    archive = dist / ('Scopobot-macos-' + platform.machine() + '.zip')
    with zipfile.ZipFile(archive, 'w', zipfile.ZIP_DEFLATED) as z:
        for file in bundle.parent.rglob('*'):
            if file.is_file(): z.write(file, file.relative_to(dist))
    print(archive)
elif is_windows:
    folder = dist / 'Scopobot-windows'
    folder.mkdir(exist_ok=True)
    for name in ['scopobot.exe','scopobot-terminal.exe']:
        shutil.copy2(release / name, folder / name)
    shutil.copytree(root / 'characters', folder / 'characters', dirs_exist_ok=True)
    for name in ['README.md', 'LICENSE']:
        shutil.copy2(root / name, folder / name)
    with zipfile.ZipFile(dist / 'Scopobot-windows.zip','w',zipfile.ZIP_DEFLATED) as z:
        for file in folder.rglob('*'):
            if file.is_file(): z.write(file,file.relative_to(dist))
    print(dist / 'Scopobot-windows.zip')
else:
    shutil.copy2(release / 'scopobot-terminal',dist / 'scopobot-terminal')
    shutil.copytree(root / 'characters',dist / 'characters',dirs_exist_ok=True)
    print(dist / 'scopobot-terminal')

for archive in dist.glob('Scopobot-*.zip'):
    digest = hashlib.sha256(archive.read_bytes()).hexdigest()
    archive.with_suffix(archive.suffix + '.sha256').write_text(digest + '  ' + archive.name + '\n')
