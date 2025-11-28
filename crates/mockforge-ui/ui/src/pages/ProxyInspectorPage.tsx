/**
 * Proxy Inspector Page
 *
 * Page wrapper for the Proxy Inspector component that provides
 * browser proxy mode inspection and replacement rule management.
 */

import React from 'react';
import { ProxyInspector } from '../components/proxy/ProxyInspector';

export function ProxyInspectorPage() {
  return (
    <div className="container mx-auto px-4 py-6">
      <ProxyInspector />
    </div>
  );
}
