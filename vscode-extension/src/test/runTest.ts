import * as fs from 'fs';
import * as os from 'os';
import * as path from 'path';
import { runTests } from '@vscode/test-electron';

/**
 * Launches VS Code with this extension loaded and runs the mocha suite in
 * `suite/`. Pinned to the oldest VS Code the extension supports
 * (`engines.vscode`). On Linux without a display, run under `xvfb-run -a`.
 */
async function main(): Promise<void> {
    const extensionDevelopmentPath = path.resolve(__dirname, '../../');
    const extensionTestsPath = path.resolve(__dirname, './suite/index');
    // VS Code puts its IPC socket in the user-data dir. Under a CI checkout the
    // default (.vscode-test/user-data) exceeds the 107-byte Unix socket path
    // limit and VS Code exits before running tests, so use a short temp dir.
    const userDataDir = fs.mkdtempSync(path.join(os.tmpdir(), 'mfvsc-'));
    try {
        await runTests({
            version: '1.85.2',
            extensionDevelopmentPath,
            extensionTestsPath,
            launchArgs: ['--disable-extensions', '--disable-workspace-trust', `--user-data-dir=${userDataDir}`],
        });
    } finally {
        fs.rmSync(userDataDir, { recursive: true, force: true });
    }
}

main().catch((err) => {
    console.error('Failed to run tests:', err);
    process.exit(1);
});
