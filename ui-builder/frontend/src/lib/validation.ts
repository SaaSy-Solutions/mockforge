import { z } from 'zod'

// Common schemas
export const headerConfigSchema = z.object({
  name: z.string().min(1, 'Header name is required'),
  value: z.string(),
})

export const responseBodySchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('Static'),
    content: z.any(),
  }),
  z.object({
    type: z.literal('Template'),
    template: z.string().min(1, 'Template content is required'),
  }),
  z.object({
    type: z.literal('Faker'),
    schema: z.any(),
  }),
  z.object({
    type: z.literal('AI'),
    prompt: z.string().min(1, 'AI prompt is required'),
  }),
])

export const latencyConfigSchema = z.object({
  base_ms: z.number().min(0, 'Base latency must be non-negative'),
  jitter_ms: z.number().min(0, 'Jitter must be non-negative'),
  distribution: z.union([
    z.literal('fixed'),
    z.object({ Normal: z.object({ std_dev_ms: z.number().min(0) }) }),
    z.object({ Pareto: z.object({ shape: z.number().min(0) }) }),
  ]),
})

export const failureConfigSchema = z.object({
  error_rate: z.number().min(0, 'Error rate must be at least 0').max(1, 'Error rate cannot exceed 1'),
  status_codes: z.array(z.number().min(100).max(599)),
  error_message: z.string().optional(),
})

export const trafficShapingConfigSchema = z.object({
  bandwidth_limit_bps: z.number().min(0).optional(),
  packet_loss_rate: z.number().min(0).max(1).optional(),
})

export const endpointBehaviorSchema = z.object({
  latency: latencyConfigSchema.optional(),
  failure: failureConfigSchema.optional(),
  traffic_shaping: trafficShapingConfigSchema.optional(),
}).optional()

// HTTP endpoint schema
export const httpResponseConfigSchema = z.object({
  status: z.number().min(100, 'Status must be at least 100').max(599, 'Status cannot exceed 599'),
  headers: z.array(headerConfigSchema).optional(),
  body: responseBodySchema,
})

export const httpEndpointConfigSchema = z.object({
  type: z.literal('Http'),
  method: z.enum(['GET', 'POST', 'PUT', 'DELETE', 'PATCH', 'HEAD', 'OPTIONS']),
  path: z.string().min(1, 'Path is required').regex(/^\//, 'Path must start with /'),
  response: httpResponseConfigSchema,
  behavior: endpointBehaviorSchema,
})

// gRPC endpoint schema
export const grpcResponseConfigSchema = z.object({
  body: responseBodySchema,
  metadata: z.array(headerConfigSchema).optional(),
})

export const grpcEndpointConfigSchema = z.object({
  type: z.literal('Grpc'),
  service: z.string().min(1, 'Service name is required'),
  method: z.string().min(1, 'Method name is required'),
  proto_file: z.string().min(1, 'Proto file path is required'),
  request_type: z.string().min(1, 'Request type is required'),
  response_type: z.string().min(1, 'Response type is required'),
  response: grpcResponseConfigSchema,
  behavior: endpointBehaviorSchema,
})

// WebSocket endpoint schema
export const websocketActionSchema = z.discriminatedUnion('type', [
  z.object({
    type: z.literal('Send'),
    message: responseBodySchema,
  }),
  z.object({
    type: z.literal('Broadcast'),
    message: responseBodySchema,
  }),
  z.object({
    type: z.literal('Echo'),
  }),
  z.object({
    type: z.literal('Close'),
    code: z.number().min(1000).max(4999),
    reason: z.string(),
  }),
])

export const websocketEndpointConfigSchema = z.object({
  type: z.literal('Websocket'),
  path: z.string().min(1, 'Path is required').regex(/^\//, 'Path must start with /'),
  on_connect: websocketActionSchema.optional(),
  on_message: websocketActionSchema.optional(),
  on_disconnect: websocketActionSchema.optional(),
  behavior: endpointBehaviorSchema,
})

// GraphQL endpoint schema
export const graphqlResolverSchema = z.object({
  operationType: z.enum(['query', 'mutation', 'subscription']),
  operationName: z.string().min(1, 'Operation name is required'),
  responseType: z.enum(['static', 'template', 'faker']),
  response: responseBodySchema,
})

export const graphqlErrorSchema = z.object({
  message: z.string().min(1, 'Error message is required'),
  extensions: z.record(z.string(), z.any()).optional(),
  path: z.array(z.string()).optional(),
})

export const graphqlEndpointConfigSchema = z.object({
  type: z.literal('Graphql'),
  path: z.string().min(1, 'Path is required').regex(/^\//, 'Path must start with /'),
  schema: z.string().min(1, 'GraphQL schema is required'),
  introspection: z.boolean(),
  resolvers: z.array(graphqlResolverSchema),
  errorSimulation: z.object({
    enabled: z.boolean(),
    errorRate: z.number().min(0).max(1).optional(),
    customErrors: z.array(graphqlErrorSchema).optional(),
  }).optional(),
  latencySimulation: z.object({
    enabled: z.boolean(),
    minMs: z.number().min(0).optional(),
    maxMs: z.number().min(0).optional(),
  }).optional(),
  behavior: z.object({
    rateLimit: z.object({
      requestsPerSecond: z.number().min(0),
      burstSize: z.number().min(0),
    }).optional(),
    complexityLimit: z.object({
      maxDepth: z.number().min(0),
      maxComplexity: z.number().min(0),
    }).optional(),
  }).optional(),
})

// MQTT endpoint schema
export const mqttQosSchema = z.union([z.literal(0), z.literal(1), z.literal(2)])

export const mqttEndpointConfigSchema = z.object({
  type: z.literal('Mqtt'),
  topicPattern: z.string().min(1, 'Topic pattern is required'),
  qos: mqttQosSchema,
  retained: z.boolean(),
  payload: responseBodySchema,
  connectionBehavior: z.object({
    onConnect: z.object({
      enabled: z.boolean(),
      topic: z.string().optional(),
      payload: z.any().optional(),
    }).optional(),
    onSubscribe: z.object({
      enabled: z.boolean(),
      payload: z.any().optional(),
      sendRetained: z.boolean().optional(),
    }).optional(),
    onPublish: z.object({
      enabled: z.boolean(),
      responseType: z.enum(['echo', 'ack', 'transform', 'custom']).optional(),
      payload: z.any().optional(),
      responseTopic: z.string().optional(),
    }).optional(),
  }).optional(),
  advanced: z.object({
    sessionExpiryInterval: z.number().min(0).optional(),
    keepAliveInterval: z.number().min(0).optional(),
    maxPacketSize: z.number().min(0).optional(),
    cleanSession: z.boolean().optional(),
    latency: z.object({
      minMs: z.number().min(0),
      maxMs: z.number().min(0),
    }).optional(),
  }).optional(),
})

// SMTP endpoint schema
export const smtpTlsModeSchema = z.enum(['none', 'starttls', 'implicit'])
export const smtpAuthMechanismSchema = z.enum(['PLAIN', 'LOGIN', 'CRAM-MD5'])

export const smtpEndpointConfigSchema = z.object({
  type: z.literal('Smtp'),
  port: z.number().min(1, 'Port is required').max(65535, 'Port must be valid'),
  hostname: z.string().optional(),
  tlsMode: smtpTlsModeSchema,
  authentication: z.object({
    enabled: z.boolean(),
    mechanisms: z.array(smtpAuthMechanismSchema).optional(),
    credentials: z.array(z.object({
      username: z.string().min(1, 'Username is required'),
      password: z.string().min(1, 'Password is required'),
    })).optional(),
  }).optional(),
  messageHandling: z.object({
    acceptAll: z.boolean().optional(),
    senderFilters: z.array(z.object({
      pattern: z.string(),
      type: z.enum(['allow', 'block']),
    })).optional(),
    recipientFilters: z.array(z.object({
      pattern: z.string(),
      type: z.enum(['allow', 'block']),
    })).optional(),
  }).optional(),
  behavior: z.object({
    defaultResponse: z.enum(['accept', 'reject', 'bounce']).optional(),
    customResponseCode: z.object({
      enabled: z.boolean(),
      code: z.number().min(200).max(599).optional(),
      message: z.string().optional(),
    }).optional(),
    latency: z.object({
      enabled: z.boolean(),
      minMs: z.number().min(0).optional(),
      maxMs: z.number().min(0).optional(),
    }).optional(),
  }).optional(),
  messageStorage: z.object({
    enabled: z.boolean(),
    maxMessages: z.number().min(1).optional(),
    apiPath: z.string().optional(),
  }).optional(),
  maxMessageSize: z.number().min(0).optional(),
  maxRecipients: z.number().min(1).optional(),
  connectionTimeout: z.number().min(0).optional(),
})

// AMQP endpoint schema
export const amqpExchangeTypeSchema = z.enum(['direct', 'fanout', 'topic', 'headers'])

export const amqpEndpointConfigSchema = z.object({
  type: z.literal('Amqp'),
  exchange: z.string().min(1, 'Exchange name is required'),
  exchangeType: amqpExchangeTypeSchema,
  routingKey: z.string(),
  queue: z.string().optional(),
  durable: z.boolean(),
  autoDelete: z.boolean(),
  payload: responseBodySchema,
  connectionBehavior: z.object({
    onConnect: z.object({
      enabled: z.boolean(),
      exchange: z.string().optional(),
      routingKey: z.string().optional(),
      payload: z.any().optional(),
    }).optional(),
    onPublish: z.object({
      enabled: z.boolean(),
      responseType: z.enum(['ack', 'nack', 'echo', 'custom']).optional(),
      payload: z.any().optional(),
    }).optional(),
  }).optional(),
  advanced: z.object({
    prefetchCount: z.number().min(0).optional(),
    heartbeatInterval: z.number().min(0).optional(),
    connectionTimeout: z.number().min(0).optional(),
    mandatory: z.boolean().optional(),
    immediate: z.boolean().optional(),
    priority: z.number().min(0).max(9).optional(),
    expiration: z.string().optional(),
    messageId: z.string().optional(),
    correlationId: z.string().optional(),
    latency: z.object({
      minMs: z.number().min(0),
      maxMs: z.number().min(0),
    }).optional(),
  }).optional(),
})

// Kafka endpoint schema
export const kafkaEndpointConfigSchema = z.object({
  type: z.literal('Kafka'),
  topic: z.string().min(1, 'Topic name is required'),
  partition: z.number().min(0).optional(),
  key: z.string().optional(),
  payload: responseBodySchema,
  producerConfig: z.object({
    acks: z.enum(['none', 'leader', 'all']).optional(),
    compression: z.enum(['none', 'gzip', 'snappy', 'lz4', 'zstd']).optional(),
    batchSize: z.number().min(0).optional(),
    lingerMs: z.number().min(0).optional(),
  }).optional(),
  consumerConfig: z.object({
    groupId: z.string().optional(),
    autoOffsetReset: z.enum(['earliest', 'latest']).optional(),
    enableAutoCommit: z.boolean().optional(),
    autoCommitIntervalMs: z.number().min(0).optional(),
  }).optional(),
  advanced: z.object({
    replicationFactor: z.number().min(1).optional(),
    partitions: z.number().min(1).optional(),
    retentionMs: z.number().min(0).optional(),
    headers: z.array(z.object({
      key: z.string(),
      value: z.string(),
    })).optional(),
    latency: z.object({
      minMs: z.number().min(0),
      maxMs: z.number().min(0),
    }).optional(),
  }).optional(),
})

// Combined endpoint config schema
export const endpointProtocolConfigSchema = z.discriminatedUnion('type', [
  httpEndpointConfigSchema,
  grpcEndpointConfigSchema,
  websocketEndpointConfigSchema,
  graphqlEndpointConfigSchema,
  mqttEndpointConfigSchema,
  smtpEndpointConfigSchema,
  amqpEndpointConfigSchema,
  kafkaEndpointConfigSchema,
])

// Full endpoint schema
export const endpointConfigSchema = z.object({
  id: z.string(),
  protocol: z.enum(['http', 'grpc', 'websocket', 'graphql', 'mqtt', 'smtp', 'kafka', 'amqp', 'ftp']),
  name: z.string().min(1, 'Endpoint name is required').max(100, 'Name must be 100 characters or less'),
  description: z.string().max(500, 'Description must be 500 characters or less').optional(),
  enabled: z.boolean(),
  config: endpointProtocolConfigSchema,
})

// Partial schema for validation during form editing (id optional for new endpoints)
export const endpointFormSchema = endpointConfigSchema.extend({
  id: z.string().optional(),
})

// Validation helper types
export type ValidationErrors = Record<string, string>

export interface ValidationResult {
  success: boolean
  errors: ValidationErrors
  data?: z.infer<typeof endpointFormSchema>
}

/**
 * Validate an endpoint configuration
 */
export function validateEndpoint(data: unknown): ValidationResult {
  const result = endpointFormSchema.safeParse(data)

  if (result.success) {
    return {
      success: true,
      errors: {},
      data: result.data,
    }
  }

  const errors: ValidationErrors = {}
  for (const issue of result.error.issues) {
    const path = issue.path.join('.')
    errors[path] = issue.message
  }

  return {
    success: false,
    errors,
  }
}

/**
 * Validate a specific field
 */
export function validateField(
  schema: z.ZodSchema,
  value: unknown
): { valid: boolean; error?: string } {
  const result = schema.safeParse(value)

  if (result.success) {
    return { valid: true }
  }

  return {
    valid: false,
    error: result.error.issues[0]?.message,
  }
}

/**
 * Get error message for a specific field path
 */
export function getFieldError(errors: ValidationErrors, path: string): string | undefined {
  return errors[path]
}

/**
 * Check if a field has an error
 */
export function hasFieldError(errors: ValidationErrors, path: string): boolean {
  return path in errors
}

// Export individual field schemas for inline validation
export const nameSchema = z.string().min(1, 'Name is required').max(100, 'Name must be 100 characters or less')
export const pathSchema = z.string().min(1, 'Path is required').regex(/^\//, 'Path must start with /')
export const portSchema = z.number().min(1, 'Port must be at least 1').max(65535, 'Port must be 65535 or less')
export const statusCodeSchema = z.number().min(100, 'Status must be at least 100').max(599, 'Status cannot exceed 599')
export const errorRateSchema = z.number().min(0, 'Must be at least 0').max(1, 'Cannot exceed 1')
