import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Toaster } from 'sonner'
import ErrorBoundary from './components/ErrorBoundary'
import Layout from './components/Layout'
import Dashboard from './pages/Dashboard'
import EndpointBuilder from './pages/EndpointBuilder'
import ConfigEditor from './pages/ConfigEditor'
import ApiDocs from './pages/ApiDocs'

function App() {
  return (
    <ErrorBoundary>
      <BrowserRouter>
        <Layout>
          <Routes>
            <Route path="/" element={<Dashboard />} />
            <Route path="/endpoints/new" element={<EndpointBuilder />} />
            <Route path="/endpoints/:id" element={<EndpointBuilder />} />
            <Route path="/config" element={<ConfigEditor />} />
            <Route path="/docs" element={<ApiDocs />} />
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </Layout>
        <Toaster position="top-right" />
      </BrowserRouter>
    </ErrorBoundary>
  )
}

export default App
