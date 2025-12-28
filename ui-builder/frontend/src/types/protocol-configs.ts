/**
 * Protocol-specific configuration types for endpoint forms
 *
 * These types extract the protocol-specific configurations from EndpointProtocolConfig
 * to provide proper typing for form components.
 *
 * Note: Form components use looser typing internally to handle partial states
 * during editing. The strict types are used for API boundaries.
 */

import type {
  ResponseBody,
  EndpointBehavior,
  HttpRequestConfig,
  HttpResponseConfig,
  GrpcResponseConfig,
  WebsocketAction,
  GraphqlMockResolver,
  GraphqlErrorSimulation,
  GraphqlLatencySimulation,
  GraphqlBehavior,
  MqttQos,
  MqttConnectionBehavior,
  MqttSubscriptionMatching,
  MqttAdvancedSettings,
  SmtpTlsMode,
  SmtpAuthentication,
  SmtpMessageHandling,
  SmtpBehavior,
  SmtpMessageStorage,
  AmqpExchangeType,
  AmqpConnectionBehavior,
  AmqpAdvancedSettings,
  KafkaProducerConfig,
  KafkaConsumerConfig,
  KafkaAdvancedSettings,
} from '@/lib/api'

// ============================================================================
// Form Config Types
// ============================================================================

/**
 * Form components work with partially-complete configs during editing.
 * These types allow flexibility while the strict Config types are used
 * for API boundaries.
 */
// eslint-disable-next-line @typescript-eslint/no-explicit-any
export type FormConfig = Record<string, any>

// ============================================================================
// HTTP Protocol Types
// ============================================================================

export interface HttpConfig {
  type: 'Http'
  method: string
  path: string
  request?: HttpRequestConfig
  response: HttpResponseConfig
  behavior?: EndpointBehavior
}

export interface HttpFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// gRPC Protocol Types
// ============================================================================

export interface GrpcConfig {
  type: 'Grpc'
  service: string
  method: string
  proto_file: string
  request_type: string
  response_type: string
  response: GrpcResponseConfig
  behavior?: EndpointBehavior
}

export interface GrpcFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// WebSocket Protocol Types
// ============================================================================

export interface WebsocketConfig {
  type: 'Websocket'
  path: string
  on_connect?: WebsocketAction
  on_message?: WebsocketAction
  on_disconnect?: WebsocketAction
  behavior?: EndpointBehavior
}

export interface WebsocketFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// GraphQL Protocol Types
// ============================================================================

export interface GraphqlConfig {
  type: 'Graphql'
  path: string
  schema: string
  introspection: boolean
  resolvers: GraphqlMockResolver[]
  errorSimulation?: GraphqlErrorSimulation
  latencySimulation?: GraphqlLatencySimulation
  behavior?: GraphqlBehavior
}

export interface GraphqlFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// MQTT Protocol Types
// ============================================================================

export interface MqttConfig {
  type: 'Mqtt'
  topicPattern: string
  qos: MqttQos
  retained: boolean
  payload: ResponseBody
  connectionBehavior?: MqttConnectionBehavior
  subscriptionMatching?: MqttSubscriptionMatching
  advanced?: MqttAdvancedSettings
}

export interface MqttFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// SMTP Protocol Types
// ============================================================================

export interface SmtpConfig {
  type: 'Smtp'
  port: number
  hostname?: string
  tlsMode: SmtpTlsMode
  authentication?: SmtpAuthentication
  messageHandling?: SmtpMessageHandling
  behavior?: SmtpBehavior
  messageStorage?: SmtpMessageStorage
  maxMessageSize?: number
  maxRecipients?: number
  connectionTimeout?: number
  requireHelo?: boolean
  enableVrfy?: boolean
  enableExpn?: boolean
}

export interface SmtpFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// AMQP Protocol Types
// ============================================================================

export interface AmqpConfig {
  type: 'Amqp'
  exchange: string
  exchangeType: AmqpExchangeType
  routingKey: string
  queue?: string
  durable: boolean
  autoDelete: boolean
  payload: ResponseBody
  connectionBehavior?: AmqpConnectionBehavior
  advanced?: AmqpAdvancedSettings
}

export interface AmqpFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// Kafka Protocol Types
// ============================================================================

export interface KafkaConfig {
  type: 'Kafka'
  topic: string
  partition?: number
  key?: string
  payload: ResponseBody
  producerConfig?: KafkaProducerConfig
  consumerConfig?: KafkaConsumerConfig
  advanced?: KafkaAdvancedSettings
}

export interface KafkaFormProps {
  config: FormConfig
  onChange: (config: FormConfig) => void
  onValidationChange?: (isValid: boolean) => void
}

// ============================================================================
// Union type for all protocol configs
// ============================================================================

export type ProtocolConfig =
  | HttpConfig
  | GrpcConfig
  | WebsocketConfig
  | GraphqlConfig
  | MqttConfig
  | SmtpConfig
  | AmqpConfig
  | KafkaConfig

// ============================================================================
// Type guards for protocol configs
// ============================================================================

export function isHttpConfig(config: ProtocolConfig): config is HttpConfig {
  return config.type === 'Http'
}

export function isGrpcConfig(config: ProtocolConfig): config is GrpcConfig {
  return config.type === 'Grpc'
}

export function isWebsocketConfig(config: ProtocolConfig): config is WebsocketConfig {
  return config.type === 'Websocket'
}

export function isGraphqlConfig(config: ProtocolConfig): config is GraphqlConfig {
  return config.type === 'Graphql'
}

export function isMqttConfig(config: ProtocolConfig): config is MqttConfig {
  return config.type === 'Mqtt'
}

export function isSmtpConfig(config: ProtocolConfig): config is SmtpConfig {
  return config.type === 'Smtp'
}

export function isAmqpConfig(config: ProtocolConfig): config is AmqpConfig {
  return config.type === 'Amqp'
}

export function isKafkaConfig(config: ProtocolConfig): config is KafkaConfig {
  return config.type === 'Kafka'
}

// ============================================================================
// Default config factories
// ============================================================================

export function createDefaultHttpConfig(): HttpConfig {
  return {
    type: 'Http',
    method: 'GET',
    path: '/api/example',
    response: {
      status: 200,
      headers: [],
      body: { type: 'Static', content: {} },
    },
  }
}

export function createDefaultGrpcConfig(): GrpcConfig {
  return {
    type: 'Grpc',
    service: 'ExampleService',
    method: 'GetExample',
    proto_file: 'example.proto',
    request_type: 'GetExampleRequest',
    response_type: 'GetExampleResponse',
    response: {
      body: { type: 'Static', content: {} },
    },
  }
}

export function createDefaultWebsocketConfig(): WebsocketConfig {
  return {
    type: 'Websocket',
    path: '/ws',
  }
}

export function createDefaultGraphqlConfig(): GraphqlConfig {
  return {
    type: 'Graphql',
    path: '/graphql',
    schema: 'type Query { hello: String }',
    introspection: true,
    resolvers: [],
  }
}

export function createDefaultMqttConfig(): MqttConfig {
  return {
    type: 'Mqtt',
    topicPattern: 'sensors/#',
    qos: 0,
    retained: false,
    payload: { type: 'Static', content: {} },
  }
}

export function createDefaultSmtpConfig(): SmtpConfig {
  return {
    type: 'Smtp',
    port: 2525,
    tlsMode: 'none',
  }
}

export function createDefaultAmqpConfig(): AmqpConfig {
  return {
    type: 'Amqp',
    exchange: 'example-exchange',
    exchangeType: 'direct',
    routingKey: 'example-key',
    durable: true,
    autoDelete: false,
    payload: { type: 'Static', content: {} },
  }
}

export function createDefaultKafkaConfig(): KafkaConfig {
  return {
    type: 'Kafka',
    topic: 'example-topic',
    payload: { type: 'Static', content: {} },
  }
}

export function createDefaultConfigForProtocol(protocol: string): ProtocolConfig {
  switch (protocol) {
    case 'http':
      return createDefaultHttpConfig()
    case 'grpc':
      return createDefaultGrpcConfig()
    case 'websocket':
      return createDefaultWebsocketConfig()
    case 'graphql':
      return createDefaultGraphqlConfig()
    case 'mqtt':
      return createDefaultMqttConfig()
    case 'smtp':
      return createDefaultSmtpConfig()
    case 'amqp':
      return createDefaultAmqpConfig()
    case 'kafka':
      return createDefaultKafkaConfig()
    default:
      return createDefaultHttpConfig()
  }
}
