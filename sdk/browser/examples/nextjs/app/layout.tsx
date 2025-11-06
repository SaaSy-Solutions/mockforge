'use client';

import { useEffect } from 'react';
import { ForgeConnect } from '@mockforge/forgeconnect';
import './globals.css';

export default function RootLayout({
  children,
}: {
  children: React.ReactNode;
}) {
  useEffect(() => {
    // Only initialize in development
    if (process.env.NODE_ENV === 'development') {
      const forgeConnect = new ForgeConnect({
        serverUrl: process.env.NEXT_PUBLIC_MOCKFORGE_URL || 'http://localhost:3000',
        mockMode: 'auto',
        autoMockStatusCodes: [404, 500],
        autoMockNetworkErrors: true,
        onMockCreated: (mock) => {
          console.log('[ForgeConnect] Mock created:', mock);
        },
        onConnectionChange: (connected, url) => {
          console.log('[ForgeConnect] Connection:', connected ? `Connected to ${url}` : 'Disconnected');
        },
      });

      forgeConnect.initialize().then((connected) => {
        if (connected) {
          console.log('[ForgeConnect] Initialized successfully');
        } else {
          console.warn('[ForgeConnect] Failed to connect to MockForge');
        }
      });

      return () => {
        forgeConnect.stop();
      };
    }
  }, []);

  return (
    <html lang="en">
      <body>{children}</body>
    </html>
  );
}

