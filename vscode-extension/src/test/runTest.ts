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
    await runTests({
        version: '1.85.2',
        extensionDevelopmentPath,
        extensionTestsPath,
        launchArgs: ['--disable-extensions', '--disable-workspace-trust'],
    });
}

main().catch((err) => {
    console.error('Failed to run tests:', err);
    process.exit(1);
});
