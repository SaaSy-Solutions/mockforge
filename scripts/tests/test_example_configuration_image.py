"""Behavior guards for configuration-only image attribution and restricted inputs."""
import hashlib
import importlib.util
import io
import json
from pathlib import Path
import subprocess
import tarfile
import tempfile
import unittest
from unittest import mock

SPEC = importlib.util.spec_from_file_location('example_image', Path(__file__).resolve().parents[1] / 'example_configuration_image.py')
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class SourceGuard(unittest.TestCase):
    def test_actual_input_changes_and_dirty_checkout_rejected(self):
        with tempfile.TemporaryDirectory(prefix='mockforge-source-guard-') as directory:
            root = Path(directory)
            def git(*args):
                return subprocess.check_output(['git', *args], cwd=root).decode().strip()
            git('init', '-q')
            git('config', 'user.email', 'fixture@example.invalid')
            git('config', 'user.name', 'Fixture')
            for p in MODULE.SPECS:
                (root / p).parent.mkdir(parents=True, exist_ok=True)
                (root / p).write_text('{}')
            (root / 'Cargo.toml').write_text('original')
            git('add', '.')
            git('commit', '-qm', 'base')
            baseline = git('rev-parse', 'HEAD')
            with mock.patch.object(MODULE, 'ROOT', root), mock.patch.object(MODULE, 'RUNTIME_SOURCE', baseline):
                self.assertEqual(MODULE.source_guard(baseline)['changed_inputs'], [])
                with self.assertRaisesRegex(ValueError, 'actual HEAD'):
                    MODULE.source_guard('a' * 40)
                (root / MODULE.SPECS[0]).write_text('{"updated":true}')
                with self.assertRaisesRegex(ValueError, 'completely clean'):
                    MODULE.source_guard(baseline)
                git('add', '.')
                git('commit', '-qm', 'configuration')
                updated = git('rev-parse', 'HEAD')
                self.assertEqual(MODULE.source_guard(updated)['changed_inputs'], [MODULE.SPECS[0]])
                (root / 'Cargo.toml').write_text('native changed')
                git('add', '.')
                git('commit', '-qm', 'native change')
                with self.assertRaisesRegex(ValueError, 'full rebuild'):
                    MODULE.source_guard(git('rev-parse', 'HEAD'))
                (root / 'untracked').write_text('unexpected')
                with self.assertRaisesRegex(ValueError, 'completely clean'):
                    MODULE.source_guard(git('rev-parse', 'HEAD'))

    def test_no_run_or_external_frontend(self):
        text = (MODULE.ROOT / 'docker/examples-refresh.Dockerfile').read_text()
        MODULE.refresh_dockerfile_guard(text)
        for invalid in (text + '\nRUN touch /bad\n', '# syntax=untrusted/frontend\n' + text,
                        text.replace('FROM ${RUNTIME_IMAGE}', 'FROM unrelated/image'),
                        text.replace('examples/ /app/examples/', './ /app/')):
            with self.assertRaises(ValueError):
                MODULE.refresh_dockerfile_guard(invalid)


class ArchiveGuard(unittest.TestCase):
    def archive(self, path, bad_file=False, corrupt=False, changed_runtime=False):
        blobs = {}
        def blob(content, media):
            digest = 'sha256:' + hashlib.sha256(content).hexdigest()
            blobs['blobs/sha256/' + digest[7:]] = content
            return {'mediaType': media, 'digest': digest, 'size': len(content)}
        empty = io.BytesIO()
        with tarfile.open(fileobj=empty, mode='w'):
            pass
        base_layer = blob(empty.getvalue(), 'application/vnd.oci.image.layer.v1.tar')
        overlay = io.BytesIO()
        with tarfile.open(fileobj=overlay, mode='w') as tar:
            for p in MODULE.SPECS:
                content = (MODULE.ROOT / p).read_bytes()
                info = tarfile.TarInfo('app/' + p)
                info.size = len(content)
                tar.addfile(info, io.BytesIO(content))
            if bad_file:
                tar.addfile(tarfile.TarInfo('app/unexpected'))
        extra = blob(overlay.getvalue(), 'application/vnd.oci.image.layer.v1.tar')
        runtime = {'User': 'mockforge', 'Entrypoint': ['/usr/local/bin/mockforge-entrypoint']}
        labels = {'org.opencontainers.image.revision': MODULE.RUNTIME_SOURCE,
                  'io.mockforge.image.kind': 'example-configuration-refresh',
                  'io.mockforge.runtime.source': MODULE.RUNTIME_SOURCE,
                  'io.mockforge.config.source': 'a' * 40,
                  'io.mockforge.runtime.base.digest': MODULE.BASE_MANIFEST,
                  'io.mockforge.runtime.binary.sha256': MODULE.BINARY}
        image_runtime = {**runtime, 'Labels': labels, 'Image': 'sha256:' + 'b' * 64,
                         'AttachStdin': False, 'Tty': False}
        if changed_runtime:
            image_runtime['User'] = 'root'
        diff_ids = [base_layer['digest']] * 12
        config = blob(json.dumps({'config': image_runtime, 'rootfs': {'diff_ids': diff_ids + [extra['digest']]}}).encode(), 'application/vnd.oci.image.config.v1+json')
        manifest = blob(json.dumps({'schemaVersion': 2, 'config': config, 'layers': [base_layer] * 12 + [extra]}).encode(), 'application/vnd.oci.image.manifest.v1+json')
        index = json.dumps({'schemaVersion': 2, 'manifests': [manifest]}).encode()
        if corrupt:
            blobs['blobs/sha256/' + extra['digest'][7:]] += b'corrupt'
        with tarfile.open(path, 'w') as tar:
            for name, content in {'index.json': index, **blobs}.items():
                info = tarfile.TarInfo(name)
                info.size = len(content)
                tar.addfile(info, io.BytesIO(content))
        return {'Config': runtime, 'RootFS': {'Layers': diff_ids}}

    def test_exact_three_spec_overlay_accepts(self):
        with tempfile.TemporaryDirectory(prefix='mockforge-oci-guard-') as directory:
            path = Path(directory) / 'image.tar'
            base = self.archive(path)
            proof = MODULE.verify_archive(path, base, 'a' * 40)
            self.assertEqual(len(proof['overlay_files']), 3)
            self.assertEqual(len(proof['rootfs_diff_ids']), 13)

    def test_other_file_blob_corruption_and_runtime_change_rejected(self):
        for fault in ('bad_file', 'corrupt', 'changed_runtime'):
            with self.subTest(fault=fault), tempfile.TemporaryDirectory(prefix='mockforge-oci-negative-') as directory:
                path = Path(directory) / 'image.tar'
                base = self.archive(path, **{fault: True})
                with self.assertRaises(ValueError):
                    MODULE.verify_archive(path, base, 'a' * 40)


if __name__ == '__main__':
    unittest.main()
