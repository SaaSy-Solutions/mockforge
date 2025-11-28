import { useEffect, useState } from 'react';
import { useQuery, QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { ForgeConnect } from '@mockforge/forgeconnect';

const queryClient = new QueryClient();

function UsersList() {
  const { data, isLoading, error } = useQuery({
    queryKey: ['users'],
    queryFn: async () => {
      const response = await fetch('/api/users');
      if (!response.ok) {
        throw new Error(`HTTP ${response.status}`);
      }
      return response.json();
    },
  });

  if (isLoading) return <div>Loading users...</div>;
  if (error) return <div>Error: {error.message}</div>;

  return (
    <div>
      <h2>Users</h2>
      <ul>
        {data?.users?.map((user: any) => (
          <li key={user.id}>{user.name} ({user.email})</li>
        )) || <li>No users found</li>}
      </ul>
    </div>
  );
}

function App() {
  const [forgeConnect, setForgeConnect] = useState<ForgeConnect | null>(null);
  const [connected, setConnected] = useState(false);

  useEffect(() => {
    const fc = new ForgeConnect({
      mockMode: 'auto',
      autoMockStatusCodes: [404, 500],
      autoMockNetworkErrors: true,
      onMockCreated: (mock) => {
        console.log('Mock created:', mock);
      },
      onConnectionChange: (isConnected) => {
        setConnected(isConnected);
      },
    });

    fc.initialize().then((isConnected) => {
      setConnected(isConnected);
      if (isConnected) {
        console.log('ForgeConnect initialized');
      } else {
        console.warn('Failed to connect to MockForge');
      }
    });

    setForgeConnect(fc);

    return () => {
      fc.stop();
    };
  }, []);

  return (
    <QueryClientProvider client={queryClient}>
      <div style={{ padding: '20px', fontFamily: 'system-ui' }}>
        <h1>ForgeConnect - React Query Example</h1>
        <div style={{
          padding: '10px',
          marginBottom: '20px',
          backgroundColor: connected ? '#d4edda' : '#f8d7da',
          color: connected ? '#155724' : '#721c24',
          borderRadius: '4px'
        }}>
          {connected ? '✓ Connected to MockForge' : '✗ Not connected to MockForge'}
        </div>
        <UsersList />
      </div>
    </QueryClientProvider>
  );
}

export default App;
