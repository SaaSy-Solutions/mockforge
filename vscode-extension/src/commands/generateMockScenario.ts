//! Generate Mock Scenario code action
//!
//! Provides a code action to generate MockForge scenario files from OpenAPI specifications

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as yaml from 'js-yaml';

/**
 * Register the "Generate Mock Scenario" code action
 */
export function registerGenerateMockScenarioCommand(context: vscode.ExtensionContext): void {
    // Register code action provider for OpenAPI files
    const codeActionProvider = vscode.languages.registerCodeActionsProvider(
        [
            { scheme: 'file', pattern: '**/*.yaml' },
            { scheme: 'file', pattern: '**/*.yml' },
            { scheme: 'file', pattern: '**/*.json' },
        ],
        {
            provideCodeActions: async (document, range, context) => {
                return provideGenerateMockScenarioActions(document, range, context);
            },
        },
        {
            providedCodeActionKinds: [vscode.CodeActionKind.QuickFix],
        }
    );

    context.subscriptions.push(codeActionProvider);

    // Register command handler
    const command = vscode.commands.registerCommand(
        'mockforge.generateMockScenario',
        async (document: vscode.TextDocument, operation?: any) => {
            await generateMockScenario(document, operation);
        }
    );

    context.subscriptions.push(command);
}

/**
 * Provide code actions for generating mock scenarios
 */
function provideGenerateMockScenarioActions(
    document: vscode.TextDocument,
    range: vscode.Range,
    context: vscode.CodeActionContext
): vscode.CodeAction[] {
    const actions: vscode.CodeAction[] = [];

    // Check if this looks like an OpenAPI spec
    const content = document.getText();
    const isOpenAPI = content.includes('openapi:') ||
                      content.includes('"openapi"') ||
                      content.includes('swagger:') ||
                      content.includes('"swagger"');

    if (isOpenAPI) {
        // Find operation definitions in the range
        const line = document.lineAt(range.start.line);
        if (line.text.includes('paths:') || line.text.includes('"paths"') ||
            line.text.match(/\s+(get|post|put|patch|delete|options|head):/i)) {

            const action = new vscode.CodeAction(
                'Generate MockForge Scenario',
                vscode.CodeActionKind.QuickFix
            );
            action.command = {
                command: 'mockforge.generateMockScenario',
                title: 'Generate MockForge Scenario',
                arguments: [document, line.text],
            };
            action.diagnostics = [...context.diagnostics];
            actions.push(action);
        }
    }

    return actions;
}

/**
 * Generate mock scenario from OpenAPI operation
 */
async function generateMockScenario(
    document: vscode.TextDocument,
    operation?: string
): Promise<void> {
    try {
        // Parse OpenAPI spec
        const content = document.getText();
        let spec: any;

        if (document.fileName.endsWith('.json')) {
            spec = JSON.parse(content);
        } else {
            // YAML parsing
            try {
                spec = yaml.load(content) as any;
            } catch (yamlError) {
                vscode.window.showErrorMessage(
                    `Failed to parse YAML: ${yamlError instanceof Error ? yamlError.message : 'Unknown error'}`
                );
                return;
            }
        }

        // Extract operations from OpenAPI spec
        const operations = extractOperations(spec);
        if (operations.length === 0) {
            vscode.window.showWarningMessage('No operations found in OpenAPI specification');
            return;
        }

        // Ask user which operations to generate scenarios for
        const selectedOperations = await vscode.window.showQuickPick(
            operations.map(op => ({
                label: `${op.method.toUpperCase()} ${op.path}`,
                description: op.summary || op.operationId,
                operation: op,
            })),
            {
                canPickMany: true,
                placeHolder: 'Select operations to generate scenarios for',
            }
        );

        if (!selectedOperations || selectedOperations.length === 0) {
            return;
        }

        // Ask for scenario name
        const scenarioName = await vscode.window.showInputBox({
            prompt: 'Enter scenario name (used for filename)',
            placeHolder: 'generated-scenario',
            value: 'generated-scenario',
            validateInput: (value) => {
                if (!value || value.trim().length === 0) {
                    return 'Scenario name cannot be empty';
                }
                if (!/^[a-z0-9-]+$/.test(value)) {
                    return 'Scenario name must contain only lowercase letters, numbers, and hyphens';
                }
                return null;
            }
        });

        if (!scenarioName) {
            return;
        }

        // Generate scenario file
        const scenarioContent = generateScenarioYaml(scenarioName, selectedOperations.map(s => s.operation));

        // Ask for output location
        const workspaceFolders = vscode.workspace.workspaceFolders;
        if (!workspaceFolders || workspaceFolders.length === 0) {
            vscode.window.showErrorMessage('No workspace folder open');
            return;
        }

        const outputPath = path.join(
            workspaceFolders[0].uri.fsPath,
            'scenarios',
            `${scenarioName}.yaml`
        );

        // Create scenarios directory if it doesn't exist
        const scenariosDir = path.dirname(outputPath);
        if (!fs.existsSync(scenariosDir)) {
            fs.mkdirSync(scenariosDir, { recursive: true });
        }

        // Write scenario file
        fs.writeFileSync(outputPath, scenarioContent, 'utf-8');

        // Open the generated file
        const doc = await vscode.workspace.openTextDocument(outputPath);
        await vscode.window.showTextDocument(doc);

        vscode.window.showInformationMessage(
            `Generated MockForge scenario: ${path.basename(outputPath)}`
        );

    } catch (error) {
        vscode.window.showErrorMessage(
            `Failed to generate scenario: ${error instanceof Error ? error.message : 'Unknown error'}`
        );
    }
}

/**
 * Extract operations from OpenAPI spec
 */
function extractOperations(spec: any): Array<{ method: string; path: string; operationId?: string; summary?: string }> {
    const operations: Array<{ method: string; path: string; operationId?: string; summary?: string }> = [];

    if (!spec.paths) {
        return operations;
    }

    const methods = ['get', 'post', 'put', 'patch', 'delete', 'options', 'head'];

    for (const [path, pathItem] of Object.entries(spec.paths)) {
        if (typeof pathItem !== 'object' || pathItem === null) {
            continue;
        }

        for (const method of methods) {
            if (method in pathItem) {
                const operation = (pathItem as any)[method];
                operations.push({
                    method,
                    path,
                    operationId: operation.operationId,
                    summary: operation.summary,
                });
            }
        }
    }

    return operations;
}

/**
 * Generate scenario YAML from operations
 */
function generateScenarioYaml(scenarioName: string, operations: Array<{ method: string; path: string; operationId?: string; summary?: string }>): string {
    const timestamp = new Date().toISOString();
    const title = scenarioName.split('-').map(word => word.charAt(0).toUpperCase() + word.slice(1)).join(' ');
    let yaml = `# Generated MockForge Scenario
# Generated from OpenAPI specification
# Generated at: ${timestamp}

manifest_version: "1.0"
name: ${scenarioName}
version: "1.0.0"
title: ${title}
description: |
  Auto-generated scenario from OpenAPI specification
  Contains ${operations.length} operation${operations.length !== 1 ? 's' : ''}

steps:
`;

    for (const op of operations) {
        yaml += `  - name: ${op.summary || op.operationId || `${op.method.toUpperCase()} ${op.path}`}
    method: ${op.method.toUpperCase()}
    path: ${op.path}
    response:
      status: 200
      body:
        # TODO: Customize response body
        message: "Mock response for ${op.method.toUpperCase()} ${op.path}"
`;

        if (op.method.toLowerCase() === 'post' || op.method.toLowerCase() === 'put' || op.method.toLowerCase() === 'patch') {
            yaml += `    request:
      body:
        # TODO: Add request body schema
`;
        }

        yaml += '\n';
    }

    return yaml;
}
