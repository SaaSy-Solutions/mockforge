import { create } from 'zustand'
import { EndpointConfig } from '@/lib/api'

interface EndpointStore {
  currentEndpoint: EndpointConfig | null
  setCurrentEndpoint: (endpoint: EndpointConfig | null) => void
  resetEndpoint: () => void
}

export const useEndpointStore = create<EndpointStore>((set) => ({
  currentEndpoint: null,
  setCurrentEndpoint: (endpoint) => set({ currentEndpoint: endpoint }),
  resetEndpoint: () => set({ currentEndpoint: null }),
}))
