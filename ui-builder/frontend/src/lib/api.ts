import axios, { AxiosError, AxiosResponse } from 'axios'
import { toast } from 'sonner'

const api = axios.create({
  baseURL: '/api',
  headers: {
    'Content-Type': 'application/json',
  },
})

// Flag to prevent duplicate toasts for the same error
let isHandlingError = false

// Response interceptor for global error handling
api.interceptors.response.use(
  (response: AxiosResponse) => {
    return response
  },
  (error: AxiosError) => {
    // Prevent duplicate error handling
    if (isHandlingError) {
      return Promise.reject(error)
    }

    // Handle network errors (no response from server)
    if (!error.response) {
      if (error.code === 'ERR_NETWORK' || error.message === 'Network Error') {
        isHandlingError = true
        toast.error('Unable to connect to server. Please check your network connection.')
        setTimeout(() => { isHandlingError = false }, 1000)
      }
      return Promise.reject(error)
    }

    const status = error.response.status
    const responseData = error.response.data as { message?: string; error?: string; details?: string } | undefined

    // Extract error message from response
    const errorMessage = responseData?.message || responseData?.error || responseData?.details || ''

    switch (status) {
      case 401:
        // Unauthorized - user needs to log in
        isHandlingError = true
        toast.error('Your session has expired. Please log in again.')
        // If there's a login page, redirect to it
        // window.location.href = '/login'
        setTimeout(() => { isHandlingError = false }, 1000)
        break

      case 403:
        // Forbidden - user doesn't have permission
        isHandlingError = true
        toast.error(errorMessage || 'You do not have permission to perform this action.')
        setTimeout(() => { isHandlingError = false }, 1000)
        break

      case 404:
        // Not found - resource doesn't exist (let individual handlers deal with this)
        break

      case 422:
        // Validation error - let individual handlers deal with this
        break

      case 429:
        // Rate limited
        isHandlingError = true
        toast.error('Too many requests. Please slow down and try again.')
        setTimeout(() => { isHandlingError = false }, 1000)
        break

      case 500:
      case 502:
      case 503:
      case 504:
        // Server errors
        isHandlingError = true
        toast.error('A server error occurred. Please try again later.')
        setTimeout(() => { isHandlingError = false }, 1000)
        break

      default:
        // For other errors, only show generic message if not already handled
        if (status >= 400 && status < 500) {
          // Client error - let individual handlers deal with specific messages
        } else if (status >= 500) {
          isHandlingError = true
          toast.error('An unexpected error occurred. Please try again.')
          setTimeout(() => { isHandlingError = false }, 1000)
        }
        break
    }

    return Promise.reject(error)
  }
)

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
  | { type: 'Graphql'; path: string; schema: string; introspection: boolean; resolvers: GraphqlMockResolver[]; errorSimulation?: GraphqlErrorSimulation; latencySimulation?: GraphqlLatencySimulation; behavior?: GraphqlBehavior }
  | { type: 'Mqtt'; topicPattern: string; qos: MqttQos; retained: boolean; payload: ResponseBody; connectionBehavior?: MqttConnectionBehavior; subscriptionMatching?: MqttSubscriptionMatching; advanced?: MqttAdvancedSettings }
  | { type: 'Smtp'; port: number; hostname?: string; tlsMode: SmtpTlsMode; authentication?: SmtpAuthentication; messageHandling?: SmtpMessageHandling; behavior?: SmtpBehavior; messageStorage?: SmtpMessageStorage; maxMessageSize?: number; maxRecipients?: number; connectionTimeout?: number; requireHelo?: boolean; enableVrfy?: boolean; enableExpn?: boolean }
  | { type: 'Amqp'; exchange: string; exchangeType: AmqpExchangeType; routingKey: string; queue?: string; durable: boolean; autoDelete: boolean; payload: ResponseBody; connectionBehavior?: AmqpConnectionBehavior; advanced?: AmqpAdvancedSettings }
  | { type: 'Kafka'; topic: string; partition?: number; key?: string; payload: ResponseBody; producerConfig?: KafkaProducerConfig; consumerConfig?: KafkaConsumerConfig; advanced?: KafkaAdvancedSettings }

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

// GraphQL types
export interface GraphqlMockResolver {
  operationType: 'query' | 'mutation' | 'subscription'
  operationName: string
  responseType: 'static' | 'template' | 'faker'
  response: ResponseBody
}

export interface GraphqlErrorSimulation {
  enabled: boolean
  errorRate?: number
  customErrors?: GraphqlError[]
}

export interface GraphqlError {
  message: string
  extensions?: Record<string, any>
  path?: string[]
}

export interface GraphqlLatencySimulation {
  enabled: boolean
  minMs?: number
  maxMs?: number
}

export interface GraphqlBehavior {
  rateLimit?: {
    requestsPerSecond: number
    burstSize: number
  }
  complexityLimit?: {
    maxDepth: number
    maxComplexity: number
  }
}

// MQTT types
export type MqttQos = 0 | 1 | 2

export interface MqttConnectionBehavior {
  onConnect?: {
    enabled: boolean
    topic?: string
    payload?: any
  }
  onSubscribe?: {
    enabled: boolean
    payload?: any
    sendRetained?: boolean
  }
  onPublish?: {
    enabled: boolean
    responseType?: 'echo' | 'ack' | 'transform' | 'custom'
    payload?: any
    responseTopic?: string
  }
}

export interface MqttSubscriptionMatching {
  echoPublished?: boolean
  topicFilters?: MqttTopicFilter[]
}

export interface MqttTopicFilter {
  pattern: string
  qos: MqttQos
}

export interface MqttAdvancedSettings {
  sessionExpiryInterval?: number
  keepAliveInterval?: number
  maxPacketSize?: number
  cleanSession?: boolean
  messageExpiryInterval?: number
  latency?: {
    minMs: number
    maxMs: number
  }
}

// SMTP types
export type SmtpTlsMode = 'none' | 'starttls' | 'implicit'

export interface SmtpAuthentication {
  enabled: boolean
  mechanisms?: SmtpAuthMechanism[]
  credentials?: SmtpCredential[]
}

export type SmtpAuthMechanism = 'PLAIN' | 'LOGIN' | 'CRAM-MD5'

export interface SmtpCredential {
  username: string
  password: string
}

export interface SmtpMessageHandling {
  acceptAll?: boolean
  senderFilters?: SmtpEmailFilter[]
  recipientFilters?: SmtpEmailFilter[]
}

export interface SmtpEmailFilter {
  pattern: string
  type: 'allow' | 'block'
}

export interface SmtpBehavior {
  defaultResponse?: SmtpDefaultResponse
  customResponseCode?: SmtpCustomResponseCode
  latency?: SmtpLatencyConfig
}

export type SmtpDefaultResponse = 'accept' | 'reject' | 'bounce'

export interface SmtpCustomResponseCode {
  enabled: boolean
  code?: number
  message?: string
}

export interface SmtpLatencyConfig {
  enabled: boolean
  minMs?: number
  maxMs?: number
}

export interface SmtpMessageStorage {
  enabled: boolean
  maxMessages?: number
  apiPath?: string
}

// AMQP types
export type AmqpExchangeType = 'direct' | 'fanout' | 'topic' | 'headers'

export interface AmqpConnectionBehavior {
  onConnect?: {
    enabled: boolean
    exchange?: string
    routingKey?: string
    payload?: any
  }
  onPublish?: {
    enabled: boolean
    responseType?: 'ack' | 'nack' | 'echo' | 'custom'
    payload?: any
  }
}

export interface AmqpAdvancedSettings {
  prefetchCount?: number
  heartbeatInterval?: number
  connectionTimeout?: number
  mandatory?: boolean
  immediate?: boolean
  priority?: number
  expiration?: string
  messageId?: string
  correlationId?: string
  latency?: {
    minMs: number
    maxMs: number
  }
}

// Kafka types
export interface KafkaProducerConfig {
  acks?: 'none' | 'leader' | 'all'
  compression?: 'none' | 'gzip' | 'snappy' | 'lz4' | 'zstd'
  batchSize?: number
  lingerMs?: number
}

export interface KafkaConsumerConfig {
  groupId?: string
  autoOffsetReset?: 'earliest' | 'latest'
  enableAutoCommit?: boolean
  autoCommitIntervalMs?: number
}

export interface KafkaAdvancedSettings {
  replicationFactor?: number
  partitions?: number
  retentionMs?: number
  headers?: { key: string; value: string }[]
  latency?: {
    minMs: number
    maxMs: number
  }
}

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
    graphql: number
    mqtt: number
    smtp: number
    kafka: number
    amqp: number
    ftp: number
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

// Server configuration types
export interface HttpServerConfig {
  enabled: boolean
  port: number
  host: string
  openapi_spec?: string
  cors?: {
    allowed_origins?: string[]
    allowed_methods?: string[]
    allowed_headers?: string[]
    allow_credentials?: boolean
  }
  request_timeout_secs?: number
  validation?: {
    enabled: boolean
    mode: 'warn' | 'enforce'
  }
  tls?: TlsServerConfig
}

export interface TlsServerConfig {
  enabled: boolean
  cert_path?: string
  key_path?: string
  port?: number
}

export interface GrpcServerConfig {
  enabled: boolean
  port: number
  host: string
  proto_dir?: string
  tls?: TlsServerConfig
}

export interface WebsocketServerConfig {
  enabled: boolean
  port: number
  host: string
  path?: string
  max_connections?: number
}

export interface GraphqlServerConfig {
  enabled: boolean
  port: number
  host: string
  path?: string
  introspection?: boolean
  playground?: boolean
}

export interface MqttServerConfig {
  enabled: boolean
  port: number
  host: string
  max_connections?: number
  session_expiry_secs?: number
  tls?: TlsServerConfig
}

export interface SmtpServerConfig {
  enabled: boolean
  port: number
  host: string
  hostname?: string
  tls_mode?: 'none' | 'starttls' | 'tls'
  max_message_size?: number
}

export interface KafkaServerConfig {
  enabled: boolean
  port: number
  host: string
  broker_id?: number
  auto_create_topics?: boolean
  default_partitions?: number
}

export interface AmqpServerConfig {
  enabled: boolean
  port: number
  host: string
  max_connections?: number
  max_channels_per_connection?: number
  heartbeat_interval?: number
  virtual_hosts?: string[]
  tls?: TlsServerConfig
}

export interface AdminConfig {
  enabled: boolean
  port: number
  host: string
  path?: string
}

export interface LoggingConfig {
  level: 'trace' | 'debug' | 'info' | 'warn' | 'error'
  format: 'json' | 'pretty' | 'compact'
  file?: string
}

export interface DataConfig {
  fixtures_dir?: string
  fake_data?: {
    seed?: number
    locale?: string
  }
}

export interface ServerConfig {
  http?: HttpServerConfig
  websocket?: WebsocketServerConfig
  grpc?: GrpcServerConfig
  graphql?: GraphqlServerConfig
  mqtt?: MqttServerConfig
  smtp?: SmtpServerConfig
  ftp?: { enabled: boolean; port: number; host: string }
  kafka?: KafkaServerConfig
  amqp?: AmqpServerConfig
  admin?: AdminConfig
  core?: { name?: string; version?: string }
  logging?: LoggingConfig
  data?: DataConfig
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
