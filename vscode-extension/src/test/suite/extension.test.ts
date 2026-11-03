import * as assert from 'assert';
import * as vscode from 'vscode';
import { validateServerUrl } from '../../utils/validation';

const EXTENSION_ID = 'saasy-solutions.mockforge-vscode';

suite('MockForge extension', () => {
    test('activates', async () => {
        const extension = vscode.extensions.getExtension(EXTENSION_ID);
        assert.ok(extension, `${EXTENSION_ID} is not loaded`);
        await extension.activate();
        assert.strictEqual(extension.isActive, true);
    });

    test('registers every contributed command', async () => {
        const extension = vscode.extensions.getExtension(EXTENSION_ID);
        assert.ok(extension);
        await extension.activate();
        const contributed: string[] = extension.packageJSON.contributes.commands.map(
            (c: { command: string }) => c.command
        );
        const registered = new Set(await vscode.commands.getCommands(true));
        const missing = contributed.filter((c) => !registered.has(c));
        assert.deepStrictEqual(missing, []);
    });

    test('validateServerUrl accepts http(s) URLs and rejects others', () => {
        assert.strictEqual(validateServerUrl('http://localhost:3000').valid, true);
        assert.strictEqual(validateServerUrl('https://mocks.example.com').valid, true);
        assert.strictEqual(validateServerUrl('').valid, false);
        assert.strictEqual(validateServerUrl('ftp://localhost').valid, false);
        assert.strictEqual(validateServerUrl('not a url').valid, false);
    });
});
