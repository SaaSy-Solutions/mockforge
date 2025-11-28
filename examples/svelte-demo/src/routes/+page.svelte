<script lang="ts">
	import { onMount } from 'svelte';

	// Import the generated client (this will be created by the MockForge CLI)
	// import { createGetUsersStore, createCreateUserStore } from '../generated/store';
	// import type { User } from '../generated/types';

	let loading = false;
	let error: string | null = null;
	let status: string | null = null;

	let usersLoading = false;
	let usersError: string | null = null;
	let users: any[] = [];

	let creating = false;
	let createError: string | null = null;
	let createSuccess = false;

	let newUser = {
		name: '',
		email: '',
		role: 'user',
		status: 'active'
	};

	onMount(() => {
		// Initialize component
	});

	function checkApiStatus() {
		loading = true;
		error = null;
		status = null;

		// Mock API status check
		setTimeout(() => {
			loading = false;
			status = 'Mock API is running';
		}, 1000);
	}

	function loadUsers() {
		usersLoading = true;
		usersError = null;

		// Mock users data
		setTimeout(() => {
			usersLoading = false;
			users = [
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

	function createUser() {
		creating = true;
		createError = null;
		createSuccess = false;

		// Mock user creation
		setTimeout(() => {
			creating = false;
			createSuccess = true;

			// Add the new user to the list
			users = [...users, {
				id: users.length + 1,
				...newUser
			}];

			// Reset form
			newUser = {
				name: '',
				email: '',
				role: 'user',
				status: 'active'
			};

			// Clear success message after 3 seconds
			setTimeout(() => {
				createSuccess = false;
			}, 3000);
		}, 1000);
	}
</script>

<div class="container">
	<header class="card">
		<h1>Svelte Demo - MockForge</h1>
		<p>This demo shows how to use MockForge-generated Svelte stores with a mock API.</p>
	</header>

	<div class="card">
		<h2>API Status</h2>
		{#if loading}
			<div class="loading"></div>
		{/if}
		{#if error}
			<div class="error">
				Error: {error}
			</div>
		{/if}
		{#if status}
			<div class="success">
				{status}
			</div>
		{/if}
		<button class="btn btn-primary" on:click={checkApiStatus} disabled={loading}>
			Check API Status
		</button>
	</div>

	<div class="card">
		<h2>Users</h2>
		{#if usersLoading}
			<div class="loading"></div>
		{/if}
		{#if usersError}
			<div class="error">
				Error loading users: {usersError}
			</div>
		{/if}
		{#if users.length > 0}
			<div class="grid grid-cols-3">
				{#each users as user (user.id)}
					<div class="card">
						<h3>{user.name}</h3>
						<p><strong>Email:</strong> {user.email}</p>
						<p><strong>Role:</strong> {user.role}</p>
						<p><strong>Status:</strong> {user.status}</p>
					</div>
				{/each}
			</div>
		{/if}
		{#if users.length === 0 && !usersLoading && !usersError}
			<p>No users found. Click "Load Users" to fetch data from the API.</p>
		{/if}
		<button class="btn btn-primary" on:click={loadUsers} disabled={usersLoading}>
			Load Users
		</button>
	</div>

	<div class="card">
		<h2>Create User</h2>
		<form on:submit|preventDefault={createUser}>
			<div class="grid grid-cols-2">
				<div>
					<label for="name">Name:</label>
					<input
						type="text"
						id="name"
						bind:value={newUser.name}
						required
						class="form-control"
					>
				</div>
				<div>
					<label for="email">Email:</label>
					<input
						type="email"
						id="email"
						bind:value={newUser.email}
						required
						class="form-control"
					>
				</div>
				<div>
					<label for="role">Role:</label>
					<select
						id="role"
						bind:value={newUser.role}
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
						bind:value={newUser.status}
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
				disabled={!newUser.name || !newUser.email || creating}
			>
				{#if creating}
					<span class="loading"></span>
				{/if}
				{creating ? 'Creating...' : 'Create User'}
			</button>
		</form>
		{#if createError}
			<div class="error">
				Error creating user: {createError}
			</div>
		{/if}
		{#if createSuccess}
			<div class="success">
				User created successfully!
			</div>
		{/if}
	</div>

	<footer class="card">
		<h3>How to use this demo:</h3>
		<ol>
			<li>Start the MockForge server: <code>mockforge serve --spec ../user-management-api.json</code></li>
			<li>Generate the Svelte client: <code>npm run generate-client</code></li>
			<li>Uncomment the import statements in this component</li>
			<li>Update the component to use the generated stores</li>
			<li>Start the Svelte app: <code>npm run dev</code></li>
		</ol>
		<p>
			<strong>Note:</strong> This demo currently uses mock data.
			To connect to the real MockForge API, follow the steps above.
		</p>
	</footer>
</div>

<style>
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
</style>
