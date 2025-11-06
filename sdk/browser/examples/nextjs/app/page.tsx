'use client';

import { useState, useEffect } from 'react';

export default function Home() {
  const [users, setUsers] = useState<any[]>([]);
  const [loading, setLoading] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const fetchUsers = async () => {
    setLoading(true);
    setError(null);
    try {
      const response = await fetch('/api/users');
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      const data = await response.json();
      setUsers(data.users || []);
    } catch (err) {
      setError(err instanceof Error ? err.message : 'Unknown error');
    } finally {
      setLoading(false);
    }
  };

  useEffect(() => {
    fetchUsers();
  }, []);

  return (
    <main style={{ padding: '20px', fontFamily: 'system-ui' }}>
      <h1>ForgeConnect - Next.js Example</h1>
      <button
        onClick={fetchUsers}
        disabled={loading}
        style={{
          padding: '10px 20px',
          backgroundColor: '#007bff',
          color: 'white',
          border: 'none',
          borderRadius: '4px',
          cursor: loading ? 'not-allowed' : 'pointer',
          marginBottom: '20px',
        }}
      >
        {loading ? 'Loading...' : 'Fetch Users'}
      </button>
      {error && (
        <div style={{ color: '#dc3545', marginBottom: '20px' }}>
          Error: {error}
        </div>
      )}
      <div>
        <h2>Users</h2>
        {users.length === 0 ? (
          <p>No users found. Make a request to /api/users to create a mock!</p>
        ) : (
          <ul>
            {users.map((user) => (
              <li key={user.id}>
                {user.name} ({user.email})
              </li>
            ))}
          </ul>
        )}
      </div>
    </main>
  );
}

