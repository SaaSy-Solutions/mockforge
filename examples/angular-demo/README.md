# Angular Demo - MockForge

This demo shows how to use MockForge-generated Angular clients with a mock API.

## Features

- **Angular 17** with standalone components
- **TypeScript** support
- **HTTP Client** integration
- **Mock API** integration with MockForge
- **Responsive design** with modern CSS
- **User management** example

## Prerequisites

- Node.js 18+
- Angular CLI 17+
- MockForge CLI

## Setup

1. **Install dependencies:**
   ```bash
   npm install
   ```

2. **Start MockForge server:**
   ```bash
   # From the examples directory
   mockforge serve --spec user-management-api.json --port 3000
   ```

3. **Generate Angular client:**
   ```bash
   npm run generate-client
   ```

4. **Update the component:**
   - Uncomment the import statements in `src/app/app.component.ts`
   - Replace mock data with real API calls using the generated service

5. **Start the Angular app:**
   ```bash
   npm start
   ```

## Project Structure

```
angular-demo/
├── src/
│   ├── app/
│   │   └── app.component.ts    # Main component
│   ├── index.html             # Main HTML file
│   ├── main.ts                # Bootstrap file
│   └── styles.scss            # Global styles
├── angular.json               # Angular configuration
├── package.json               # Dependencies and scripts
├── tsconfig.json              # TypeScript configuration
└── generated/                 # Generated client files (after running generate-client)
    ├── types.ts
    ├── user-management.service.ts
    ├── user-management.module.ts
    └── package.json
```

## Generated Files

After running `npm run generate-client`, the following files will be created:

- **`types.ts`** - TypeScript type definitions
- **`user-management.service.ts`** - Angular service for API calls
- **`user-management.module.ts`** - Angular module configuration
- **`package.json`** - Package configuration for the generated client

## Usage Example

```typescript
import { Component, OnInit } from '@angular/core';
import { UserManagementService } from './generated/user-management.service';
import { User } from './generated/types';

@Component({
  selector: 'app-example',
  template: `
    <div *ngIf="loading">Loading...</div>
    <div *ngFor="let user of users">
      {{ user.name }} - {{ user.email }}
    </div>
  `
})
export class ExampleComponent implements OnInit {
  users: User[] = [];
  loading = false;

  constructor(private userService: UserManagementService) {}

  ngOnInit() {
    this.loadUsers();
  }

  loadUsers() {
    this.loading = true;
    this.userService.getUsers().subscribe({
      next: (users) => {
        this.users = users;
        this.loading = false;
      },
      error: (error) => {
        console.error('Error loading users:', error);
        this.loading = false;
      }
    });
  }
}
```

## API Endpoints

The demo uses the following mock API endpoints:

- `GET /api/users` - Get all users
- `POST /api/users` - Create a new user
- `GET /api/users/{id}` - Get user by ID
- `PUT /api/users/{id}` - Update user
- `DELETE /api/users/{id}` - Delete user

## Customization

You can customize the generated client by:

1. **Modifying templates** in the MockForge plugin
2. **Adding custom options** to the generation command
3. **Extending the generated service** with additional methods
4. **Creating custom components** that use the generated types

## Troubleshooting

### Common Issues

1. **CORS errors**: Make sure MockForge is running and accessible
2. **Import errors**: Ensure the generated client files exist
3. **Type errors**: Check that TypeScript configuration is correct
4. **Build errors**: Verify all dependencies are installed

### Debug Mode

Enable debug logging in MockForge:
```bash
mockforge serve --spec user-management-api.json --log-level debug
```

## Next Steps

- Explore the generated client files
- Add more complex API operations
- Implement error handling
- Add unit tests
- Deploy to production

## Learn More

- [Angular Documentation](https://angular.io/docs)
- [MockForge Documentation](../README.md)
- [TypeScript Documentation](https://www.typescriptlang.org/docs/)
