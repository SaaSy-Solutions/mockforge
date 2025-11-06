# ForgeConnect Examples

This directory contains example applications demonstrating how to use ForgeConnect with different frameworks.

## Examples

### Vanilla JavaScript

A simple HTML page demonstrating ForgeConnect with vanilla JavaScript.

**To run:**

1. Build the SDK:
   ```bash
   cd ../..
   npm run build
   ```

2. Serve the example:
   ```bash
   # Using Python
   python -m http.server 8080
   
   # Or using Node.js
   npx serve -p 8080
   ```

3. Open `http://localhost:8080/examples/vanilla-js/index.html` in your browser

4. Make sure MockForge is running on `localhost:3000`

### React Query

A React application using `@tanstack/react-query` with ForgeConnect.

**To run:**

```bash
cd react-query
npm install
npm run dev
```

Open `http://localhost:5173` in your browser.

### Next.js

A Next.js application with ForgeConnect integration.

**To run:**

```bash
cd nextjs
npm install
npm run dev
```

Open `http://localhost:3000` in your browser.

### Vue.js

A Vue 3 application with ForgeConnect integration.

**To run:**

```bash
cd vue
npm install
npm run dev
```

Open `http://localhost:5174` in your browser.

### Angular

An Angular application with ForgeConnect integration.

**To run:**

```bash
cd angular
npm install
npm start
```

Open `http://localhost:4200` in your browser.

## Prerequisites

All examples require:

1. MockForge server running on `localhost:3000` (or configure a different URL)
2. Modern browser with ES modules support
3. Node.js 18+ (for React Query and Next.js examples)

## Notes

- The examples use auto-mock mode, which automatically creates mocks for failed requests
- Make requests to endpoints that don't exist to see auto-mock creation in action
- Check the browser console for ForgeConnect logs

