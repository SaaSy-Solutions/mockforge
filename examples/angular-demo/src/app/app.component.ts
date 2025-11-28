import { Component, OnInit } from '@angular/core';
import { CommonModule } from '@angular/common';
import { HttpClientModule } from '@angular/common/http';

// Import the generated client (this will be created by the MockForge CLI)
// import { UserManagementService } from './generated/user-management.service';
// import { User } from './generated/types';

@Component({
  selector: 'app-root',
  standalone: true,
  imports: [CommonModule, HttpClientModule],
  template: `
    <div class="container">
      <header class="card">
        <h1>Angular Demo - MockForge</h1>
        <p>This demo shows how to use MockForge-generated Angular clients with a mock API.</p>
      </header>

      <div class="card">
        <h2>API Status</h2>
        <div *ngIf="loading" class="loading"></div>
        <div *ngIf="error" class="error">
          Error: {{ error }}
        </div>
        <div *ngIf="status" class="success">
          API is running and accessible!
        </div>
        <button class="btn btn-primary" (click)="checkApiStatus()" [disabled]="loading">
          Check API Status
        </button>
      </div>

      <div class="card">
        <h2>Users</h2>
        <div *ngIf="usersLoading" class="loading"></div>
        <div *ngIf="usersError" class="error">
          Error loading users: {{ usersError }}
        </div>
        <div *ngIf="users.length > 0" class="grid grid-cols-3">
          <div *ngFor="let user of users" class="card">
            <h3>{{ user.name }}</h3>
            <p><strong>Email:</strong> {{ user.email }}</p>
            <p><strong>Role:</strong> {{ user.role }}</p>
            <p><strong>Status:</strong> {{ user.status }}</p>
          </div>
        </div>
        <div *ngIf="users.length === 0 && !usersLoading && !usersError">
          <p>No users found. Click "Load Users" to fetch data from the API.</p>
        </div>
        <button class="btn btn-primary" (click)="loadUsers()" [disabled]="usersLoading">
          Load Users
        </button>
      </div>

      <div class="card">
        <h2>Create User</h2>
        <form (ngSubmit)="createUser()" #userForm="ngForm">
          <div class="grid grid-cols-2">
            <div>
              <label for="name">Name:</label>
              <input
                type="text"
                id="name"
                name="name"
                [(ngModel)]="newUser.name"
                required
                class="form-control"
              >
            </div>
            <div>
              <label for="email">Email:</label>
              <input
                type="email"
                id="email"
                name="email"
                [(ngModel)]="newUser.email"
                required
                class="form-control"
              >
            </div>
            <div>
              <label for="role">Role:</label>
              <select
                id="role"
                name="role"
                [(ngModel)]="newUser.role"
                required
                class="form-control"
              >
                <option value="user">User</option>
                <option value="admin">Admin</option>
                <option value="moderator">Moderator</option>
              </select>
            </div>
            <div>
              <label for="status">Status:</label>
              <select
                id="status"
                name="status"
                [(ngModel)]="newUser.status"
                required
                class="form-control"
              >
                <option value="active">Active</option>
                <option value="inactive">Inactive</option>
                <option value="pending">Pending</option>
              </select>
            </div>
          </div>
          <button
            type="submit"
            class="btn btn-primary"
            [disabled]="!userForm.form.valid || creating"
          >
            <span *ngIf="creating" class="loading"></span>
            {{ creating ? 'Creating...' : 'Create User' }}
          </button>
        </form>
        <div *ngIf="createError" class="error">
          Error creating user: {{ createError }}
        </div>
        <div *ngIf="createSuccess" class="success">
          User created successfully!
        </div>
      </div>

      <footer class="card">
        <h3>How to use this demo:</h3>
        <ol>
          <li>Start the MockForge server: <code>mockforge serve --spec ../user-management-api.json</code></li>
          <li>Generate the Angular client: <code>npm run generate-client</code></li>
          <li>Uncomment the import statements in this component</li>
          <li>Update the component to use the generated service</li>
          <li>Start the Angular app: <code>npm start</code></li>
        </ol>
        <p>
          <strong>Note:</strong> This demo currently uses mock data.
          To connect to the real MockForge API, follow the steps above.
        </p>
      </footer>
    </div>
  `,
  styles: [`
    .form-control {
      width: 100%;
      padding: 0.5rem;
      border: 1px solid #d1d5db;
      border-radius: 4px;
      margin: 0.25rem 0;
    }

    label {
      display: block;
      font-weight: 500;
      margin-bottom: 0.25rem;
    }

    h1 {
      color: #1e293b;
      margin-bottom: 0.5rem;
    }

    h2 {
      color: #374151;
      margin-bottom: 1rem;
    }

    h3 {
      color: #4b5563;
      margin-bottom: 0.5rem;
    }

    code {
      background-color: #f3f4f6;
      padding: 0.25rem 0.5rem;
      border-radius: 4px;
      font-family: 'Courier New', monospace;
    }

    ol {
      margin-left: 1.5rem;
    }

    li {
      margin-bottom: 0.5rem;
    }
  `]
})
export class AppComponent implements OnInit {
  loading = false;
  error: string | null = null;
  status: string | null = null;

  usersLoading = false;
  usersError: string | null = null;
  users: any[] = [];

  creating = false;
  createError: string | null = null;
  createSuccess = false;

  newUser = {
    name: '',
    email: '',
    role: 'user',
    status: 'active'
  };

  ngOnInit() {
    // Initialize component
  }

  checkApiStatus() {
    this.loading = true;
    this.error = null;
    this.status = null;

    // Mock API status check
    setTimeout(() => {
      this.loading = false;
      this.status = 'Mock API is running';
    }, 1000);
  }

  loadUsers() {
    this.usersLoading = true;
    this.usersError = null;

    // Mock users data
    setTimeout(() => {
      this.usersLoading = false;
      this.users = [
        {
          id: 1,
          name: 'John Doe',
          email: 'john@example.com',
          role: 'admin',
          status: 'active'
        },
        {
          id: 2,
          name: 'Jane Smith',
          email: 'jane@example.com',
          role: 'user',
          status: 'active'
        },
        {
          id: 3,
          name: 'Bob Johnson',
          email: 'bob@example.com',
          role: 'moderator',
          status: 'inactive'
        }
      ];
    }, 1000);
  }

  createUser() {
    this.creating = true;
    this.createError = null;
    this.createSuccess = false;

    // Mock user creation
    setTimeout(() => {
      this.creating = false;
      this.createSuccess = true;
      this.newUser = {
        name: '',
        email: '',
        role: 'user',
        status: 'active'
      };

      // Add the new user to the list
      this.users.push({
        id: this.users.length + 1,
        ...this.newUser
      });

      // Clear success message after 3 seconds
      setTimeout(() => {
        this.createSuccess = false;
      }, 3000);
    }, 1000);
  }
}
