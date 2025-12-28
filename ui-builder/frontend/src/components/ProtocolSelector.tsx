import { Globe, Zap, MessageSquare, Activity, Mail, Database, Radio, Server, LucideIcon } from 'lucide-react'
import { cn } from '@/lib/utils'

interface ProtocolOption {
  id: string
  name: string
  description: string
  icon: LucideIcon
  color: string
}

const protocols: ProtocolOption[] = [
  {
    id: 'http',
    name: 'HTTP/REST',
    description: 'RESTful API endpoints with OpenAPI support',
    icon: Globe,
    color: 'text-blue-500',
  },
  {
    id: 'grpc',
    name: 'gRPC',
    description: 'High-performance RPC with Protocol Buffers',
    icon: Zap,
    color: 'text-purple-500',
  },
  {
    id: 'websocket',
    name: 'WebSocket',
    description: 'Real-time bidirectional communication',
    icon: MessageSquare,
    color: 'text-green-500',
  },
  {
    id: 'graphql',
    name: 'GraphQL',
    description: 'Flexible query language for APIs',
    icon: Activity,
    color: 'text-pink-500',
  },
  {
    id: 'mqtt',
    name: 'MQTT',
    description: 'Lightweight messaging for IoT',
    icon: Database,
    color: 'text-orange-500',
  },
  {
    id: 'smtp',
    name: 'SMTP',
    description: 'Email protocol mocking',
    icon: Mail,
    color: 'text-red-500',
  },
  {
    id: 'amqp',
    name: 'AMQP',
    description: 'Advanced message queue protocol (RabbitMQ)',
    icon: Radio,
    color: 'text-cyan-500',
  },
  {
    id: 'kafka',
    name: 'Kafka',
    description: 'Distributed event streaming platform',
    icon: Server,
    color: 'text-amber-500',
  },
]

interface ProtocolSelectorProps {
  selected: string
  onSelect: (protocol: string) => void
}

export default function ProtocolSelector({ selected, onSelect }: ProtocolSelectorProps) {
  return (
    <div className="rounded-lg border border-border bg-card p-6">
      <h2 className="mb-4 text-lg font-semibold" id="protocol-selector-heading">Select Protocol</h2>
      <div
        className="grid grid-cols-1 gap-4 md:grid-cols-2 lg:grid-cols-3 xl:grid-cols-4"
        role="radiogroup"
        aria-labelledby="protocol-selector-heading"
      >
        {protocols.map((protocol) => {
          const Icon = protocol.icon
          const isSelected = selected === protocol.id
          return (
            <button
              key={protocol.id}
              onClick={() => onSelect(protocol.id)}
              className={cn(
                'rounded-lg border-2 p-4 text-left transition-all focus:outline-none focus:ring-2 focus:ring-ring focus:ring-offset-2',
                isSelected
                  ? 'border-primary bg-primary/5'
                  : 'border-border bg-background hover:border-primary/50'
              )}
              role="radio"
              aria-checked={isSelected}
              aria-label={`${protocol.name}: ${protocol.description}`}
            >
              <div className="flex items-start space-x-3">
                <Icon className={cn('h-6 w-6', protocol.color)} aria-hidden="true" />
                <div className="flex-1">
                  <h3 className="font-semibold">{protocol.name}</h3>
                  <p className="mt-1 text-xs text-muted-foreground">{protocol.description}</p>
                </div>
              </div>
            </button>
          )
        })}
      </div>
    </div>
  )
}
