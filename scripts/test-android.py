import importlib.util
import io
import unittest
import zipfile
from pathlib import Path
spec = importlib.util.spec_from_file_location('android', Path(__file__).with_name('android.py'))
android = importlib.util.module_from_spec(spec)
spec.loader.exec_module(android)
class AndroidAbiTest(unittest.TestCase):
    def test_nsh_03_runtime_with_32_bit_abi(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w') as apk:
            for abi in ('arm64-v8a', 'armeabi-v7a', 'x86_64'):
                apk.writestr(f'lib/{abi}/libnsh.so', b'runtime')
            apk.writestr('assets/example', b'asset')
        data.seek(0)
        with zipfile.ZipFile(data) as apk:
            self.assertEqual(android.supported_abis(apk), {'arm64-v8a': 'aarch64-linux-android', 'x86_64': 'x86_64-linux-android'})
            self.assertFalse(android.include_runtime_entry('lib/armeabi-v7a/libnsh.so'))
            self.assertTrue(android.include_runtime_entry('lib/arm64-v8a/libnsh.so'))
            self.assertTrue(android.include_runtime_entry('assets/example'))
    def test_missing_supported_abi_fails(self):
        data = io.BytesIO()
        with zipfile.ZipFile(data, 'w') as apk:
            apk.writestr('lib/arm64-v8a/libnsh.so', b'runtime')
        data.seek(0)
        with zipfile.ZipFile(data) as apk, self.assertRaises(SystemExit):
            android.supported_abis(apk)
if __name__ == '__main__': unittest.main()
