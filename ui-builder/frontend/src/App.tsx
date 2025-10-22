import { BrowserRouter, Routes, Route, Navigate } from 'react-router-dom'
import { Toaster } from 'sonner'
import Layout from './components/Layout'
import Dashboard from './pages/Dashboard'
import EndpointBuilder from './pages/EndpointBuilder'
import ConfigEditor from './pages/ConfigEditor'

function App() {
  return (
    <BrowserRouter>
      <Layout>
        <Routes>
          <Route path="/" element={<Dashboard />} />
          <Route path="/endpoints/new" element={<EndpointBuilder />} />
          <Route path="/endpoints/:id" element={<EndpointBuilder />} />
          <Route path="/config" element={<ConfigEditor />} />
          <Route path="*" element={<Navigate to="/" replace />} />
        </Routes>
      </Layout>
      <Toaster position="top-right" />
    </BrowserRouter>
  )
}

export default App
