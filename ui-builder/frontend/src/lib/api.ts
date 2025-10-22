import axios from 'axios'

const api = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
  },
})

export interface EndpointConfig {
  id: string
  protocol: 'http' | 'grpc' | 'websocket' | 'graphql' | 'mqtt' | 'smtp' | 'kafka' | 'amqp' | 'ftp'
  name: string
  description?: string
  enabled: boolean
  config: EndpointProtocolConfig
}

export type EndpointProtocolConfig =
  | { type: 'Http'; method: string; path: string; request?: HttpRequestConfig; response: HttpResponseConfig; behavior?: EndpointBehavior }
  | { type: 'Grpc'; service: string; method: string; proto_file: string; request_type: string; response_type: string; response: GrpcResponseConfig; behavior?: EndpointBehavior }
  | { type: 'Websocket'; path: string; on_connect?: WebsocketAction; on_message?: WebsocketAction; on_disconnect?: WebsocketAction; behavior?: EndpointBehavior }

export interface HttpRequestConfig {
  validation?: ValidationConfig
  headers?: HeaderConfig[]
  query_params?: QueryParamConfig[]
  body_schema?: any
}

export interface HttpResponseConfig {
  status: number
  headers?: HeaderConfig[]
  body: ResponseBody
}

export type ResponseBody =
  | { type: 'Static'; content: any }
  | { type: 'Template'; template: string }
  | { type: 'Faker'; schema: any }
  | { type: 'AI'; prompt: string }

export interface HeaderConfig {
  name: string
  value: string
}

export interface QueryParamConfig {
  name: string
  required: boolean
  schema?: any
}

export interface ValidationConfig {
  mode: 'off' | 'warn' | 'enforce'
  schema?: any
}

export interface EndpointBehavior {
  latency?: LatencyConfig
  failure?: FailureConfig
  traffic_shaping?: TrafficShapingConfig
}

export interface LatencyConfig {
  base_ms: number
  jitter_ms: number
  distribution: 'fixed' | { Normal: { std_dev_ms: number } } | { Pareto: { shape: number } }
}

export interface FailureConfig {
  error_rate: number
  status_codes: number[]
  error_message?: string
}

export interface TrafficShapingConfig {
  bandwidth_limit_bps?: number
  packet_loss_rate?: number
}

export interface GrpcResponseConfig {
  body: ResponseBody
  metadata?: HeaderConfig[]
}

export type WebsocketAction =
  | { type: 'Send'; message: ResponseBody }
  | { type: 'Broadcast'; message: ResponseBody }
  | { type: 'Echo' }
  | { type: 'Close'; code: number; reason: string }

export interface ValidationResult {
  valid: boolean
  errors: ValidationError[]
  warnings: string[]
}

export interface ValidationError {
  field: string
  message: string
}

export interface EndpointListResponse {
  endpoints: EndpointConfig[]
  total: number
  enabled: number
  by_protocol: {
    http: number
    grpc: number
    websocket: number
  }
}

// API methods
export const endpointsApi = {
  list: () => api.get<EndpointListResponse>('/endpoints'),
  get: (id: string) => api.get<EndpointConfig>(`/endpoints/${id}`),
  create: (endpoint: Omit<EndpointConfig, 'id'>) => api.post<EndpointConfig>('/endpoints', endpoint),
  update: (id: string, endpoint: EndpointConfig) => api.put<EndpointConfig>(`/endpoints/${id}`, endpoint),
  delete: (id: string) => api.delete(`/endpoints/${id}`),
  validate: (endpoint: EndpointConfig) => api.post<ValidationResult>('/endpoints/validate', endpoint),
}

export interface ServerConfig {
  http?: any
  websocket?: any
  grpc?: any
  graphql?: any
  mqtt?: any
  smtp?: any
  ftp?: any
  kafka?: any
  amqp?: any
  admin?: any
  core?: any
  logging?: any
  data?: any
}

export const configApi = {
  get: () => api.get<ServerConfig>('/config'),
  update: (config: ServerConfig) => api.put<ServerConfig>('/config', config),
  export: () => api.get<string>('/config/export', { responseType: 'text' }),
  import: (config: string, format: 'yaml' | 'json') => api.post('/config/import', { config, format }),
}

export interface OpenApiSpecInfo {
  title: string
  version: string
  description?: string
  openapi_version: string
  servers: string[]
}

export interface ImportOpenApiResponse {
  success: boolean
  endpoints_created: number
  warnings: string[]
  spec_info: OpenApiSpecInfo
}

export const openApiApi = {
  import: (content: string, base_url?: string, auto_enable?: boolean) =>
    api.post<ImportOpenApiResponse>('/openapi/import', { content, base_url, auto_enable }),
  export: () => api.get<any>('/openapi/export'),
}

export default api
