#!/usr/bin/env python3
"""Build NSH's Android app, include native system services, and sign the APK."""
import argparse
import hashlib
import json
import os
from pathlib import Path
import shutil
import subprocess
import tomllib
import zipfile

ROOT = Path(__file__).resolve().parents[1]


def run(*args, **kwargs):
    return subprocess.run([str(arg) for arg in args], cwd=ROOT, check=True, **kwargs)


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--keystore', required=True, type=Path)
    parser.add_argument('--alias', default='omasend')
    args = parser.parse_args()
    if not os.environ.get('NSH_KEYSTORE_PASSWORD'):
        parser.error('Set NSH_KEYSTORE_PASSWORD for the release keystore')
    version = tomllib.loads((ROOT / 'Cargo.toml').read_text())['package']['version']
    sdk = Path(os.environ.get('ANDROID_HOME', str(Path.home() / 'Library/Android/sdk')))
    tools = max((sdk / 'build-tools').iterdir(), key=lambda path: tuple(map(int, path.name.split('.'))))
    android_jar = sdk / 'platforms/android-35/android.jar'
    # NSH builds the Rust app and the ARM64 Android runtime.
    run('nsh', 'package', '--target', 'android', '--app-version', version,
        '--build', '200', '--icon', 'assets/omasend.png',
        '--keystore', args.keystore, '--key-alias', args.alias)
    metadata = json.loads(run('cargo', 'metadata', '--format-version', '1',
        capture_output=True, text=True).stdout)
    base = next(Path(package['manifest_path']).parent for package in metadata['packages'] if package['name'] == 'nsh-base')
    java = base.parent / 'nsh-platform-android/java'
    work = ROOT / '.nsh/android-services'
    shutil.rmtree(work, ignore_errors=True)
    classes = work / 'classes'
    classes.mkdir(parents=True)
    sources = sorted(java.rglob('*.java')) + sorted((ROOT / 'native/android').rglob('*.java'))
    run('javac', '--release', '17', '-encoding', 'UTF-8', '-classpath', android_jar, '-d', classes, *sources)
    run('jar', 'cf', work / 'classes.jar', '-C', classes, '.')
    run(tools / 'd8', '--min-api', '28', '--lib', android_jar, '--output', work, work / 'classes.jar')
    manifest = work / 'AndroidManifest.xml'
    manifest.write_text('''<manifest xmlns:android="http://schemas.android.com/apk/res/android" package="com.huacnlee.omasend">
    <uses-permission android:name="android.permission.INTERNET" />
    <uses-permission android:name="android.permission.ACCESS_NETWORK_STATE" />
    <uses-permission android:name="android.permission.CHANGE_WIFI_MULTICAST_STATE" />
    <application android:label="Omasend" android:icon="@mipmap/ic_launcher" android:usesCleartextTraffic="true"
        android:extractNativeLibs="false" android:enableOnBackInvokedCallback="true" android:supportsRtl="true"
        android:theme="@android:style/Theme.Material.Light.NoActionBar" android:allowBackup="false">
        <meta-data android:name="com.longbridge.nsh.RUST" android:value="true" />
        <provider android:name="com.longbridge.nsh.OmasendFiles" android:authorities="com.huacnlee.omasend.files"
            android:exported="false" android:grantUriPermissions="true" />
        <activity android:name="com.longbridge.nsh.OmasendActivity" android:exported="true"
            android:windowSoftInputMode="adjustResize"
            android:configChanges="orientation|screenSize|screenLayout|smallestScreenSize|keyboard|keyboardHidden|navigation|uiMode|density|fontScale|locale|layoutDirection">
            <intent-filter><action android:name="android.intent.action.MAIN" /><category android:name="android.intent.category.LAUNCHER" /></intent-filter>
        </activity>
    </application>
</manifest>''')
    icon = work / 'res/mipmap-xxxhdpi/ic_launcher.png'
    icon.parent.mkdir(parents=True)
    shutil.copyfile(ROOT / 'assets/omasend.png', icon)
    compiled = work / 'compiled'
    compiled.mkdir()
    run(tools / 'aapt2', 'compile', icon, '-o', compiled)
    base_apk = work / 'base.apk'
    run(tools / 'aapt2', 'link', '-o', base_apk, '-I', android_jar, '--manifest', manifest,
        '--min-sdk-version', '28', '--target-sdk-version', '35', '--version-code', '200',
        '--version-name', version, *compiled.glob('*.flat'))
    unaligned = work / 'unaligned.apk'
    with zipfile.ZipFile(unaligned, 'w', zipfile.ZIP_DEFLATED) as output:
        with zipfile.ZipFile(base_apk) as resources:
            for entry in resources.infolist():
                output.writestr(entry.filename, resources.read(entry))
        output.write(work / 'classes.dex', 'classes.dex')
        with zipfile.ZipFile(ROOT / '.nsh/out/Omasend.apk') as runtime:
            for entry in runtime.infolist():
                if entry.filename.startswith(('lib/', 'assets/')):
                    output.writestr(entry.filename, runtime.read(entry), compress_type=zipfile.ZIP_STORED if entry.filename.endswith('.so') else zipfile.ZIP_DEFLATED)
    dist = ROOT / 'dist'
    dist.mkdir(exist_ok=True)
    apk = dist / f'omasend-{version}-aarch64-linux-android.apk'
    run(tools / 'zipalign', '-P', '16', '-f', '4', unaligned, apk)
    run(tools / 'apksigner', 'sign', '--ks', args.keystore, '--ks-key-alias', args.alias,
        '--ks-pass', 'env:NSH_KEYSTORE_PASSWORD', apk)
    run(tools / 'apksigner', 'verify', '--verbose', apk)
    run(tools / 'zipalign', '-c', '-P', '16', '4', apk)
    apk.with_name(apk.name + '.sha256').write_text(f'{hashlib.sha256(apk.read_bytes()).hexdigest()}  {apk.name}\n')
    print(f'Packaged: {apk}')


if __name__ == '__main__':
    main()
