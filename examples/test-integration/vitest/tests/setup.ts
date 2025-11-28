/**
 * Global setup for Vitest tests
 * Starts the MockForge server before all tests and stops it after
 */

import { spawn, ChildProcess } from 'child_process';
import { promisify } from 'util';

const sleep = promisify(setTimeout);

let serverProcess: ChildProcess | null = null;
const SERVER_PORT = 3000;
const SERVER_URL = `http://localhost:${SERVER_PORT}`;

/**
 * Wait for the server to become healthy
 */
async function waitForServer(maxAttempts = 30): Promise<void> {
  for (let i = 0; i < maxAttempts; i++) {
    try {
      const response = await fetch(`${SERVER_URL}/health`);
      if (response.ok) {
        console.log('✅ MockForge server is ready!');
        return;
      }
    } catch (error) {
      // Server not ready yet, wait and retry
    }
    await sleep(1000);
  }
  throw new Error('Server failed to start within timeout period');
}

/**
 * Global setup - starts MockForge server
 */
export async function setup() {
  console.log('🚀 Starting MockForge server...');

  // Start the server process
  serverProcess = spawn(
    'cargo',
    ['run', '--manifest-path', '../Cargo.toml', '--bin', 'mockforge-test-server'],
    {
      stdio: 'pipe',
      env: {
        ...process.env,
        RUST_LOG: 'info',
      },
    }
  );

  // Log server output
  serverProcess.stdout?.on('data', (data) => {
    console.log(`[MockForge] ${data.toString().trim()}`);
  });

  serverProcess.stderr?.on('data', (data) => {
    console.error(`[MockForge Error] ${data.toString().trim()}`);
  });

  // Handle unexpected exit
  serverProcess.on('exit', (code) => {
    if (code !== 0 && code !== null) {
      console.error(`❌ MockForge server exited with code ${code}`);
    }
  });

  // Wait for server to be ready
  await waitForServer();

  console.log(`📍 Server URL: ${SERVER_URL}`);
}

/**
 * Global teardown - stops MockForge server
 */
export async function teardown() {
  if (serverProcess) {
    console.log('🛑 Stopping MockForge server...');
    serverProcess.kill('SIGTERM');

    // Wait for process to exit
    await new Promise<void>((resolve) => {
      serverProcess?.on('exit', () => {
        console.log('✅ MockForge server stopped');
        resolve();
      });

      // Force kill after 5 seconds if not stopped gracefully
      setTimeout(() => {
        if (serverProcess && !serverProcess.killed) {
          console.log('⚠️  Force killing server...');
          serverProcess.kill('SIGKILL');
          resolve();
        }
      }, 5000);
    });

    serverProcess = null;
  }
}
