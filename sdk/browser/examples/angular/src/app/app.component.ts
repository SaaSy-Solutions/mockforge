import { Component, OnInit, OnDestroy } from '@angular/core';
import { ForgeConnectService } from '@mockforge/forgeconnect/adapters/angular';

@Component({
  selector: 'app-root',
  template: `
    <div style="padding: 20px; font-family: system-ui">
      <h1>ForgeConnect - Angular Example</h1>
      <div [style.background-color]="connected ? '#d4edda' : '#f8d7da'"
           [style.color]="connected ? '#155724' : '#721c24'"
           style="padding: 10px; margin-bottom: 20px; border-radius: 4px">
        {{ connected ? '✓ Connected to MockForge' : '✗ Not connected to MockForge' }}
      </div>
      <button (click)="fetchUsers()" [disabled]="loading">
        {{ loading ? 'Loading...' : 'Fetch Users' }}
      </button>
      <div *ngIf="error" style="color: #dc3545; margin-top: 10px">
        Error: {{ error }}
      </div>
      <div *ngIf="users.length > 0" style="margin-top: 20px">
        <h2>Users</h2>
        <ul>
          <li *ngFor="let user of users">
            {{ user.name }} ({{ user.email }})
          </li>
        </ul>
      </div>
    </div>
  `,
})
export class AppComponent implements OnInit, OnDestroy {
  connected = false;
  users: any[] = [];
  loading = false;
  error: string | null = null;

  constructor(private forgeConnect: ForgeConnectService) {}

  ngOnInit() {
    this.connected = this.forgeConnect.connected;
    
    // Listen for connection changes
    this.forgeConnect.getForgeConnect().getConnectionStatus();
  }

  async fetchUsers() {
    this.loading = true;
    this.error = null;
    try {
      const response = await fetch('/api/users');
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      this.users = data.users || [];
    } catch (err) {
      this.error = err instanceof Error ? err.message : 'Unknown error';
    } finally {
      this.loading = false;
    }
  }

  ngOnDestroy() {
    this.forgeConnect.stop();
  }
}

