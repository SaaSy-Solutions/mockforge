import * as vscode from 'vscode';

/**
 * Log levels for structured logging
 */
export enum LogLevel {
    Debug = 0,
    Info = 1,
    Warn = 2,
    Error = 3
}

/**
 * Logger for MockForge extension using VS Code output channel
 */
export class Logger {
    private static outputChannel: vscode.OutputChannel | undefined;
    private static currentLogLevel: LogLevel = LogLevel.Info;

    /**
     * Initialize the logger
     */
    static initialize(): void {
        if (!this.outputChannel) {
            this.outputChannel = vscode.window.createOutputChannel('MockForge');
        }

        // Load log level from configuration
        const config = vscode.workspace.getConfiguration('mockforge');
        const logLevelStr = config.get<string>('logLevel', 'info').toLowerCase();
        this.currentLogLevel = this.parseLogLevel(logLevelStr);
    }

    /**
     * Set the minimum log level
     */
    static setLogLevel(level: LogLevel): void {
        this.currentLogLevel = level;
    }

    /**
     * Parse log level from string
     */
    private static parseLogLevel(level: string): LogLevel {
        switch (level.toLowerCase()) {
            case 'debug':
                return LogLevel.Debug;
            case 'info':
                return LogLevel.Info;
            case 'warn':
                return LogLevel.Warn;
            case 'error':
                return LogLevel.Error;
            default:
                return LogLevel.Info;
        }
    }

    /**
     * Log a message at the specified level
     */
    private static log(level: LogLevel, message: string, ...args: unknown[]): void {
        if (!this.outputChannel) {
            this.initialize();
        }

        if (level < this.currentLogLevel) {
            return; // Skip logging if below threshold
        }

        const timestamp = new Date().toISOString();
        const levelStr = LogLevel[level].toUpperCase();
        const formattedMessage = this.formatMessage(message, args);
        const logEntry = `[${timestamp}] [${levelStr}] ${formattedMessage}`;

        this.outputChannel!.appendLine(logEntry);

        // Show output channel for errors
        if (level === LogLevel.Error) {
            this.outputChannel!.show(true);
        }
    }

    /**
     * Format message with arguments
     */
    private static formatMessage(message: string, args: unknown[]): string {
        if (args.length === 0) {
            return message;
        }

        try {
            // Replace placeholders or append JSON stringified args
            let formatted = message;
            args.forEach((arg, index) => {
                const placeholder = `{${index}}`;
                if (formatted.includes(placeholder)) {
                    formatted = formatted.replace(placeholder, this.stringifyArg(arg));
                } else {
                    formatted += ` ${this.stringifyArg(arg)}`;
                }
            });
            return formatted;
        } catch (error) {
            return `${message} ${args.map(arg => String(arg)).join(' ')}`;
        }
    }

    /**
     * Stringify argument for logging
     */
    private static stringifyArg(arg: unknown): string {
        if (arg === null || arg === undefined) {
            return String(arg);
        }

        if (typeof arg === 'object') {
            try {
                return JSON.stringify(arg, null, 2);
            } catch {
                return String(arg);
            }
        }

        return String(arg);
    }

    /**
     * Log a debug message
     */
    static debug(message: string, ...args: unknown[]): void {
        this.log(LogLevel.Debug, message, ...args);
    }

    /**
     * Log an info message
     */
    static info(message: string, ...args: unknown[]): void {
        this.log(LogLevel.Info, message, ...args);
    }

    /**
     * Log a warning message
     */
    static warn(message: string, ...args: unknown[]): void {
        this.log(LogLevel.Warn, message, ...args);
    }

    /**
     * Log an error message
     */
    static error(message: string, ...args: unknown[]): void {
        this.log(LogLevel.Error, message, ...args);
    }

    /**
     * Show the output channel
     */
    static show(): void {
        if (!this.outputChannel) {
            this.initialize();
        }
        this.outputChannel!.show(true);
    }

    /**
     * Dispose the logger
     */
    static dispose(): void {
        if (this.outputChannel) {
            this.outputChannel.dispose();
            this.outputChannel = undefined;
        }
    }
}
