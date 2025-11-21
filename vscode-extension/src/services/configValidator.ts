//! Configuration file validator for MockForge
//!
//! Provides validation for mockforge.yaml and mockforge.toml files
//! using JSON Schema generated from Rust config structs

import * as vscode from 'vscode';
import * as fs from 'fs';
import * as path from 'path';
import * as yaml from 'js-yaml';
import { exec } from 'child_process';
import { promisify } from 'util';
import Ajv, { ValidateFunction, ErrorObject } from 'ajv';
import addFormats from 'ajv-formats';

const execAsync = promisify(exec);

/**
 * Schema type detection result
 */
interface SchemaTypeResult {
    type: string;
    schemaFile: string;
}

/**
 * Configuration validator that uses JSON Schema for validation
 */
export class ConfigValidator {
    private schemaCache: Map<string, Record<string, unknown>> = new Map();
    private ajv: Ajv;
    private validators: Map<string, ValidateFunction> = new Map();

    constructor() {
        // Initialize AJV with JSON Schema Draft 7 support
        this.ajv = new Ajv({
            allErrors: true,
            verbose: true,
            strict: false, // Allow some flexibility for YAML quirks
            validateFormats: true,
        });

        // Add format validators (email, uri, etc.)
        addFormats(this.ajv);
    }

    /**
     * Validate a MockForge configuration file
     */
    async validateConfig(document: vscode.TextDocument): Promise<vscode.Diagnostic[]> {
        const diagnostics: vscode.Diagnostic[] = [];

        // Check if this is a MockForge config file
        if (!this.isMockForgeConfigFile(document)) {
            return diagnostics;
        }

        try {
            // Detect schema type based on file name/pattern
            const schemaType = this.detectSchemaType(document);
            if (!schemaType) {
                // Unknown file type, skip validation
                return diagnostics;
            }

            // Load or generate schema
            const schema = await this.getSchema(schemaType.type, schemaType.schemaFile);
            if (!schema) {
                // Schema not available, skip validation
                return diagnostics;
            }

            // Parse the config file
            const parseResult = this.parseConfig(document);
            if (!parseResult.success) {
                // Parsing failed, add error with position
                if (parseResult.error) {
                    diagnostics.push({
                        range: parseResult.error.range || new vscode.Range(0, 0, 0, 0),
                        message: parseResult.error.message || 'Failed to parse configuration file',
                        severity: vscode.DiagnosticSeverity.Error,
                        source: 'mockforge',
                    });
                }
                return diagnostics;
            }

            // Get or create validator for this schema
            let validator = this.validators.get(schemaType.type);
            if (!validator) {
                try {
                    validator = this.ajv.compile(schema);
                    this.validators.set(schemaType.type, validator);
                } catch (error) {
                    const errorMessage = error instanceof Error ? error.message : 'Unknown error';
                    diagnostics.push({
                        range: new vscode.Range(0, 0, 0, 0),
                        message: `Failed to compile schema: ${errorMessage}`,
                        severity: vscode.DiagnosticSeverity.Warning,
                        source: 'mockforge',
                    });
                    return diagnostics;
                }
            }

            // Validate against schema
            const valid = validator(parseResult.config);
            if (!valid && validator.errors) {
                // Convert AJV errors to VS Code diagnostics
                const validationErrors = this.convertAjvErrorsToDiagnostics(
                    validator.errors,
                    document
                );
                diagnostics.push(...validationErrors);
            }

        } catch (error) {
            // Log error but don't block user
            const errorMessage = error instanceof Error ? error.message : 'Unknown error';
            console.error('Config validation error:', error);
            diagnostics.push({
                range: new vscode.Range(0, 0, 0, 0),
                message: `Validation error: ${errorMessage}`,
                severity: vscode.DiagnosticSeverity.Warning,
                source: 'mockforge',
            });
        }

        return diagnostics;
    }

    /**
     * Detect schema type based on file name and path
     */
    private detectSchemaType(document: vscode.TextDocument): SchemaTypeResult | null {
        const fileName = path.basename(document.fileName).toLowerCase();
        const filePath = document.fileName.toLowerCase();

        // Main config file
        if (fileName === 'mockforge.yaml' || fileName === 'mockforge.yml' ||
            fileName === 'mockforge.json' || fileName === 'mockforge.toml') {
            return {
                type: 'mockforge-config',
                schemaFile: 'mockforge_config.schema.json'
            };
        }

        // Blueprint file
        if (fileName === 'blueprint.yaml' || fileName === 'blueprint.yml') {
            return {
                type: 'blueprint-config',
                schemaFile: 'blueprint_config.schema.json'
            };
        }

        // Reality config (in reality/ directory or reality*.yaml)
        if (filePath.includes('/reality/') || fileName.startsWith('reality')) {
            return {
                type: 'reality-config',
                schemaFile: 'reality_config.schema.json'
            };
        }

        // Persona config (in personas/ directory)
        if (filePath.includes('/personas/')) {
            return {
                type: 'persona-config',
                schemaFile: 'persona_config.schema.json'
            };
        }

        // Try to detect from file pattern
        if (fileName.endsWith('.mockforge.yaml') || fileName.endsWith('.mockforge.yml')) {
            return {
                type: 'mockforge-config',
                schemaFile: 'mockforge_config.schema.json'
            };
        }

        return null;
    }

    /**
     * Check if document is a MockForge config file
     */
    private isMockForgeConfigFile(document: vscode.TextDocument): boolean {
        const fileName = path.basename(document.fileName).toLowerCase();
        return fileName === 'mockforge.yaml' ||
               fileName === 'mockforge.yml' ||
               fileName === 'mockforge.toml' ||
               fileName === 'mockforge.json' ||
               fileName === 'blueprint.yaml' ||
               fileName === 'blueprint.yml' ||
               fileName.endsWith('.mockforge.yaml') ||
               fileName.endsWith('.mockforge.yml') ||
               fileName.startsWith('reality') ||
               document.fileName.toLowerCase().includes('/personas/');
    }

    /**
     * Get JSON Schema for MockForge config
     */
    private async getSchema(schemaType: string, schemaFile: string): Promise<Record<string, unknown> | null> {
        // Check cache first
        if (this.schemaCache.has(schemaType)) {
            return this.schemaCache.get(schemaType) || null;
        }

        try {
            // Try to find schema file in workspace
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                return null;
            }

            // Look for schema in common locations
            const schemaPaths = [
                path.join(workspaceFolders[0].uri.fsPath, 'schemas', schemaFile),
                path.join(workspaceFolders[0].uri.fsPath, 'schemas', `${schemaType}.schema.json`),
                path.join(workspaceFolders[0].uri.fsPath, '.mockforge', schemaFile),
                path.join(workspaceFolders[0].uri.fsPath, schemaFile),
            ];

            for (const schemaPath of schemaPaths) {
                if (fs.existsSync(schemaPath)) {
                    const schemaContent = fs.readFileSync(schemaPath, 'utf-8');
                    const schema = JSON.parse(schemaContent);
                    this.schemaCache.set(schemaType, schema);
                    return schema;
                }
            }

            // Try to generate schema using CLI
            const schema = await this.generateSchema(schemaType);
            if (schema) {
                this.schemaCache.set(schemaType, schema);
                return schema;
            }

        } catch (error) {
            console.error('Error loading schema:', error);
        }

        return null;
    }

    /**
     * Generate schema using mockforge CLI
     */
    private async generateSchema(schemaType: string): Promise<Record<string, unknown> | null> {
        try {
            // Map schema type to CLI type
            const cliType = schemaType.replace('-config', '');

            // Try to run mockforge schema generate command
            // First try to generate to a temp location
            const workspaceFolders = vscode.workspace.workspaceFolders;
            if (!workspaceFolders || workspaceFolders.length === 0) {
                return null;
            }

            const schemasDir = path.join(workspaceFolders[0].uri.fsPath, 'schemas');
            if (!fs.existsSync(schemasDir)) {
                fs.mkdirSync(schemasDir, { recursive: true });
            }

            // Generate schema using CLI
            const command = `mockforge schema generate --type ${cliType} --output ${schemasDir}`;
            await execAsync(command);

            // Try to load the generated schema
            const schemaFile = path.join(schemasDir, `${schemaType.replace('-', '_')}.schema.json`);
            if (fs.existsSync(schemaFile)) {
                const schemaContent = fs.readFileSync(schemaFile, 'utf-8');
                return JSON.parse(schemaContent);
            }

        } catch (error) {
            // CLI not available or command failed
            console.warn('Could not generate schema via CLI:', error);
        }

        return null;
    }

    /**
     * Parse configuration file with error tracking
     */
    private parseConfig(document: vscode.TextDocument): { success: boolean; config?: unknown; error?: { message: string; range?: vscode.Range } } {
        const content = document.getText();
        const fileName = path.basename(document.fileName).toLowerCase();

        try {
            let config: unknown;

            if (fileName.endsWith('.json')) {
                config = JSON.parse(content);
            } else if (fileName.endsWith('.toml')) {
                // TOML parsing - for now, return error suggesting JSON/YAML
                return {
                    success: false,
                    error: {
                        message: 'TOML parsing not yet supported. Please use YAML or JSON format.',
                        range: new vscode.Range(0, 0, 0, 0)
                    }
                };
            } else {
                // YAML parsing using js-yaml
                try {
                    config = yaml.load(content, {
                        filename: document.fileName,
                        strict: false,
                    });
                } catch (yamlError: unknown) {
                    // Try to extract line/column from YAML error
                    let range = new vscode.Range(0, 0, 0, 0);
                    const error = yamlError as { mark?: { line?: number; column?: number }; message?: string };
                    if (error.mark) {
                        const line = error.mark.line || 0;
                        const column = error.mark.column || 0;
                        range = new vscode.Range(line, column, line, column + 10);
                    }
                    return {
                        success: false,
                        error: {
                            message: `YAML parse error: ${error.message || 'Invalid YAML syntax'}`,
                            range
                        }
                    };
                }
            }

            return { success: true, config };
        } catch (error) {
            const errorMessage = error instanceof Error ? error.message : 'Unknown parse error';
            return {
                success: false,
                error: {
                    message: `Parse error: ${errorMessage}`,
                    range: new vscode.Range(0, 0, 0, 0)
                }
            };
        }
    }

    /**
     * Convert AJV validation errors to VS Code diagnostics
     */
    private convertAjvErrorsToDiagnostics(
        errors: ErrorObject[],
        document: vscode.TextDocument
    ): vscode.Diagnostic[] {
        const diagnostics: vscode.Diagnostic[] = [];

        for (const error of errors) {
            // Get the property path
            const instancePath = error.instancePath || error.schemaPath || '';
            const propertyPath = instancePath.replace(/^\//, '').replace(/\//g, '.');

            // Find the range for this property in the document
            const range = this.findPropertyRange(document, propertyPath);

            // Format error message
            const message = this.formatAjvError(error, propertyPath);

            // Determine severity
            let severity = vscode.DiagnosticSeverity.Error;
            if (error.keyword === 'additionalProperties') {
                severity = vscode.DiagnosticSeverity.Warning;
            }

            diagnostics.push({
                range,
                message,
                severity,
                source: 'mockforge',
            });
        }

        return diagnostics;
    }

    /**
     * Format AJV error message for display
     */
    private formatAjvError(error: ErrorObject, propertyPath: string): string {
        const path = propertyPath || 'root';

        switch (error.keyword) {
            case 'required': {
                const missing = error.params?.missingProperty as string;
                return `Missing required property: ${missing}`;
            }

            case 'type': {
                const expected = error.params?.type as string;
                return `Property "${path}" must be of type ${expected}`;
            }

            case 'enum': {
                const allowed = error.params?.allowedValues as unknown[];
                return `Property "${path}" must be one of: ${allowed?.join(', ') || 'unknown values'}`;
            }

            case 'format': {
                const format = error.params?.format as string;
                return `Property "${path}" must be a valid ${format}`;
            }

            case 'pattern':
                return `Property "${path}" does not match required pattern`;

            case 'minimum':
            case 'maximum': {
                const limit = error.params?.limit as number;
                return `Property "${path}" must be ${error.keyword === 'minimum' ? '>=' : '<='} ${limit}`;
            }

            case 'minLength':
            case 'maxLength': {
                const length = error.params?.limit as number;
                return `Property "${path}" must have length ${error.keyword === 'minLength' ? '>=' : '<='} ${length}`;
            }

            case 'additionalProperties': {
                const additional = error.params?.additionalProperty as string;
                return `Unknown property: ${additional}`;
            }

            default:
                return error.message || `Validation error at "${path}": ${error.keyword}`;
        }
    }

    /**
     * Find the range of a property in the document
     */
    private findPropertyRange(
        document: vscode.TextDocument,
        propertyPath: string
    ): vscode.Range {
        // Try to find the property in the document text
        const parts = propertyPath.split('.').filter(p => p);

        // Search for the property name in the document
        const searchPattern = parts.length > 0 ? parts[parts.length - 1] : propertyPath;
        const lines = document.getText().split('\n');

        for (let i = 0; i < lines.length; i++) {
            const line = lines[i];
            // Look for property pattern: "property:" or "property: value"
            const regex = new RegExp(`^\\s*${searchPattern.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\s*:`, 'i');
            const match = line.match(regex);
            if (match) {
                const startCol = match.index || 0;
                const endCol = startCol + match[0].length;
                return new vscode.Range(i, startCol, i, endCol);
            }
        }

        // Fallback: return first line if not found
        return new vscode.Range(0, 0, 0, 0);
    }

    /**
     * Clear schema cache
     */
    clearCache(): void {
        this.schemaCache.clear();
        this.validators.clear();
    }
}
