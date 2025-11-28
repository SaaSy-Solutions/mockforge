<template>
  <div style="padding: 20px; font-family: system-ui">
    <h1>ForgeConnect - Vue.js Example</h1>
    <div :style="{
      padding: '10px',
      marginBottom: '20px',
      backgroundColor: connected ? '#d4edda' : '#f8d7da',
      color: connected ? '#155724' : '#721c24',
      borderRadius: '4px'
    }">
      {{ connected ? '✓ Connected to MockForge' : '✗ Not connected to MockForge' }}
    </div>
    <button @click="fetchUsers" :disabled="loading">
      {{ loading ? 'Loading...' : 'Fetch Users' }}
    </button>
    <div v-if="error" style="color: #dc3545; margin-top: 10px">
      Error: {{ error }}
    </div>
    <div v-if="users.length > 0" style="margin-top: 20px">
      <h2>Users</h2>
      <ul>
        <li v-for="user in users" :key="user.id">
          {{ user.name }} ({{ user.email }})
        </li>
      </ul>
    </div>
  </div>
</template>

<script setup lang="ts">
import { ref } from 'vue';
import { useForgeConnect } from '@mockforge/forgeconnect/adapters/vue';

const { connected } = useForgeConnect({
  mockMode: 'auto',
  autoMockStatusCodes: [404, 500],
  autoMockNetworkErrors: true,
});

const users = ref<any[]>([]);
const loading = ref(false);
const error = ref<string | null>(null);

const fetchUsers = async () => {
  loading.value = true;
  error.value = null;
  try {
    const response = await fetch('/api/users');
    if (!response.ok) {
      throw new Error(`HTTP ${response.status}`);
    }
    const data = await response.json();
    users.value = data.users || [];
  } catch (err) {
    error.value = err instanceof Error ? err.message : 'Unknown error';
  } finally {
    loading.value = false;
  }
};
</script>
