import React, { useState } from 'react';
import {
  FileCode,
  Upload,
  Save,
  X,
  Plus,
  Trash2,
  AlertCircle,
  CheckCircle2,
} from 'lucide-react';
import {
  ModernCard,
  ModernBadge,
  Alert,
  Section,
} from './ui/DesignSystem';
import { Button } from './ui/button';
import { Input } from './ui/input';
import { Label } from './ui/label';
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from './ui/select';
import { Textarea } from './ui/textarea';
import { protocolContractsApi, type ProtocolType, type CreateGrpcContractRequest, type CreateWebSocketContractRequest, type CreateMqttContractRequest, type CreateKafkaContractRequest, type WebSocketMessageTypeRequest, type MqttTopicSchemaRequest, type KafkaTopicSchemaRequest } from '../services/protocolContractsApi';
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { logger } from '@/utils/logger';

interface ProtocolContractEditorProps {
  onClose?: () => void;
  onSuccess?: () => void;
}

export function ProtocolContractEditor({ onClose, onSuccess }: ProtocolContractEditorProps) {
  const queryClient = useQueryClient();
  const [protocol, setProtocol] = useState<ProtocolType>('grpc');
  const [contractId, setContractId] = useState('');
  const [version, setVersion] = useState('1.0.0');

  // gRPC specific
  const [descriptorSet, setDescriptorSet] = useState<File | null>(null);
  const [descriptorSetBase64, setDescriptorSetBase64] = useState('');

  // WebSocket specific
  const [messageTypes, setMessageTypes] = useState<WebSocketMessageTypeRequest[]>([]);

  // MQTT specific
  const [mqttTopics, setMqttTopics] = useState<MqttTopicSchemaRequest[]>([]);

  // Kafka specific
  const [kafkaTopics, setKafkaTopics] = useState<KafkaTopicSchemaRequest[]>([]);

  const createMutation = useMutation({
    mutationFn: async () => {
      switch (protocol) {
        case 'grpc':
          if (!descriptorSetBase64) {
            throw new Error('Please upload a protobuf descriptor set file');
          }
          const grpcRequest: CreateGrpcContractRequest = {
            contract_id: contractId,
            version,
            descriptor_set: descriptorSetBase64,
          };
          return protocolContractsApi.createGrpcContract(grpcRequest);

        case 'websocket':
          if (messageTypes.length === 0) {
            throw new Error('Please add at least one message type');
          }
          const wsRequest: CreateWebSocketContractRequest = {
            contract_id: contractId,
            version,
            message_types: messageTypes,
          };
          return protocolContractsApi.createWebSocketContract(wsRequest);

        case 'mqtt':
          if (mqttTopics.length === 0) {
            throw new Error('Please add at least one topic');
          }
          const mqttRequest: CreateMqttContractRequest = {
            contract_id: contractId,
            version,
            topics: mqttTopics,
          };
          return protocolContractsApi.createMqttContract(mqttRequest);

        case 'kafka':
          if (kafkaTopics.length === 0) {
            throw new Error('Please add at least one topic');
          }
          const kafkaRequest: CreateKafkaContractRequest = {
            contract_id: contractId,
            version,
            topics: kafkaTopics,
          };
          return protocolContractsApi.createKafkaContract(kafkaRequest);

        default:
          throw new Error(`Unsupported protocol: ${protocol}`);
      }
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['protocol-contracts'] });
      onSuccess?.();
      onClose?.();
    },
    onError: (error: Error) => {
      logger.error('Failed to create contract', error);
      alert(`Failed to create contract: ${error.message}`);
    },
  });

  const handleFileUpload = async (file: File) => {
    const reader = new FileReader();
    reader.onload = (e) => {
      const arrayBuffer = e.target?.result as ArrayBuffer;
      const bytes = new Uint8Array(arrayBuffer);
      // Convert to base64
      let binary = '';
      bytes.forEach((byte) => {
        binary += String.fromCharCode(byte);
      });
      const base64 = btoa(binary);
      setDescriptorSetBase64(base64);
      setDescriptorSet(file);
    };
    reader.readAsArrayBuffer(file);
  };

  const addMessageType = () => {
    setMessageTypes([
      ...messageTypes,
      {
        message_type: '',
        topic: '',
        schema: {},
        direction: 'bidirectional',
      },
    ]);
  };

  const updateMessageType = (index: number, updates: Partial<WebSocketMessageTypeRequest>) => {
    const updated = [...messageTypes];
    updated[index] = { ...updated[index], ...updates };
    setMessageTypes(updated);
  };

  const removeMessageType = (index: number) => {
    setMessageTypes(messageTypes.filter((_, i) => i !== index));
  };

  const addMqttTopic = () => {
    setMqttTopics([
      ...mqttTopics,
      {
        topic: '',
        qos: 0,
        schema: {},
        retained: false,
      },
    ]);
  };

  const updateMqttTopic = (index: number, updates: Partial<MqttTopicSchemaRequest>) => {
    const updated = [...mqttTopics];
    updated[index] = { ...updated[index], ...updates };
    setMqttTopics(updated);
  };

  const removeMqttTopic = (index: number) => {
    setMqttTopics(mqttTopics.filter((_, i) => i !== index));
  };

  const addKafkaTopic = () => {
    setKafkaTopics([
      ...kafkaTopics,
      {
        topic: '',
        value_schema: {
          format: 'json',
          schema: {},
        },
      },
    ]);
  };

  const updateKafkaTopic = (index: number, updates: Partial<KafkaTopicSchemaRequest>) => {
    const updated = [...kafkaTopics];
    updated[index] = { ...updated[index], ...updates };
    setKafkaTopics(updated);
  };

  const removeKafkaTopic = (index: number) => {
    setKafkaTopics(kafkaTopics.filter((_, i) => i !== index));
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h2 className="text-2xl font-bold">Create Protocol Contract</h2>
        {onClose && (
          <Button variant="ghost" size="icon" onClick={onClose}>
            <X className="w-4 h-4" />
          </Button>
        )}
      </div>

      {/* Protocol Selection */}
      <Section title="Protocol Selection">
        <div className="space-y-4">
          <div>
            <Label>Protocol Type</Label>
            <Select value={protocol} onValueChange={(value) => setProtocol(value as ProtocolType)}>
              <SelectTrigger>
                <SelectValue />
              </SelectTrigger>
              <SelectContent>
                <SelectItem value="grpc">gRPC</SelectItem>
                <SelectItem value="websocket">WebSocket</SelectItem>
                <SelectItem value="mqtt">MQTT</SelectItem>
                <SelectItem value="kafka">Kafka</SelectItem>
              </SelectContent>
            </Select>
          </div>

          <div className="grid grid-cols-2 gap-4">
            <div>
              <Label>Contract ID</Label>
              <Input
                placeholder="my-service-contract"
                value={contractId}
                onChange={(e) => setContractId(e.target.value)}
              />
            </div>
            <div>
              <Label>Version</Label>
              <Input
                placeholder="1.0.0"
                value={version}
                onChange={(e) => setVersion(e.target.value)}
              />
            </div>
          </div>
        </div>
      </Section>

      {/* gRPC Configuration */}
      {protocol === 'grpc' && (
        <Section title="gRPC Contract">
          <div className="space-y-4">
            <div>
              <Label>Protobuf Descriptor Set</Label>
              <div className="mt-2">
                <input
                  type="file"
                  accept=".pb,.bin"
                  onChange={(e) => {
                    const file = e.target.files?.[0];
                    if (file) {
                      handleFileUpload(file);
                    }
                  }}
                  className="block w-full text-sm text-gray-500 file:mr-4 file:py-2 file:px-4 file:rounded-md file:border-0 file:text-sm file:font-semibold file:bg-blue-50 file:text-blue-700 hover:file:bg-blue-100"
                />
              </div>
              {descriptorSet && (
                <Alert className="mt-2">
                  <CheckCircle2 className="w-4 h-4" />
                  <span>File loaded: {descriptorSet.name}</span>
                </Alert>
              )}
            </div>
          </div>
        </Section>
      )}

      {/* WebSocket Configuration */}
      {protocol === 'websocket' && (
        <Section title="WebSocket Message Types">
          <div className="space-y-4">
            {messageTypes.map((msgType, index) => (
              <ModernCard key={index} className="p-4">
                <div className="flex items-center justify-between mb-4">
                  <h4 className="font-semibold">Message Type {index + 1}</h4>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removeMessageType(index)}
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label>Message Type ID</Label>
                    <Input
                      value={msgType.message_type}
                      onChange={(e) =>
                        updateMessageType(index, { message_type: e.target.value })
                      }
                    />
                  </div>
                  <div>
                    <Label>Topic (optional)</Label>
                    <Input
                      value={msgType.topic || ''}
                      onChange={(e) =>
                        updateMessageType(index, { topic: e.target.value || undefined })
                      }
                    />
                  </div>
                  <div>
                    <Label>Direction</Label>
                    <Select
                      value={msgType.direction}
                      onValueChange={(value: 'inbound' | 'outbound' | 'bidirectional') =>
                        updateMessageType(index, { direction: value })
                      }
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="inbound">Inbound</SelectItem>
                        <SelectItem value="outbound">Outbound</SelectItem>
                        <SelectItem value="bidirectional">Bidirectional</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <div className="mt-4">
                  <Label>JSON Schema</Label>
                  <Textarea
                    placeholder='{"type": "object", "properties": {...}}'
                    value={JSON.stringify(msgType.schema, null, 2)}
                    onChange={(e) => {
                      try {
                        const schema = JSON.parse(e.target.value);
                        updateMessageType(index, { schema });
                      } catch {
                        // Invalid JSON, ignore
                      }
                    }}
                    rows={6}
                    className="font-mono text-xs"
                  />
                </div>
              </ModernCard>
            ))}
            <Button variant="outline" onClick={addMessageType}>
              <Plus className="w-4 h-4 mr-2" />
              Add Message Type
            </Button>
          </div>
        </Section>
      )}

      {/* MQTT Configuration */}
      {protocol === 'mqtt' && (
        <Section title="MQTT Topics">
          <div className="space-y-4">
            {mqttTopics.map((topic, index) => (
              <ModernCard key={index} className="p-4">
                <div className="flex items-center justify-between mb-4">
                  <h4 className="font-semibold">Topic {index + 1}</h4>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removeMqttTopic(index)}
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </div>
                <div className="grid grid-cols-2 gap-4">
                  <div>
                    <Label>Topic Name</Label>
                    <Input
                      value={topic.topic}
                      onChange={(e) =>
                        updateMqttTopic(index, { topic: e.target.value })
                      }
                    />
                  </div>
                  <div>
                    <Label>QoS Level</Label>
                    <Select
                      value={String(topic.qos || 0)}
                      onValueChange={(value) =>
                        updateMqttTopic(index, { qos: parseInt(value) })
                      }
                    >
                      <SelectTrigger>
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="0">0 - At most once</SelectItem>
                        <SelectItem value="1">1 - At least once</SelectItem>
                        <SelectItem value="2">2 - Exactly once</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>
                </div>
                <div className="mt-4">
                  <Label>JSON Schema</Label>
                  <Textarea
                    placeholder='{"type": "object", "properties": {...}}'
                    value={JSON.stringify(topic.schema, null, 2)}
                    onChange={(e) => {
                      try {
                        const schema = JSON.parse(e.target.value);
                        updateMqttTopic(index, { schema });
                      } catch {
                        // Invalid JSON, ignore
                      }
                    }}
                    rows={6}
                    className="font-mono text-xs"
                  />
                </div>
              </ModernCard>
            ))}
            <Button variant="outline" onClick={addMqttTopic}>
              <Plus className="w-4 h-4 mr-2" />
              Add Topic
            </Button>
          </div>
        </Section>
      )}

      {/* Kafka Configuration */}
      {protocol === 'kafka' && (
        <Section title="Kafka Topics">
          <div className="space-y-4">
            {kafkaTopics.map((topic, index) => (
              <ModernCard key={index} className="p-4">
                <div className="flex items-center justify-between mb-4">
                  <h4 className="font-semibold">Topic {index + 1}</h4>
                  <Button
                    variant="ghost"
                    size="icon"
                    onClick={() => removeKafkaTopic(index)}
                  >
                    <Trash2 className="w-4 h-4" />
                  </Button>
                </div>
                <div className="space-y-4">
                  <div>
                    <Label>Topic Name</Label>
                    <Input
                      value={topic.topic}
                      onChange={(e) =>
                        updateKafkaTopic(index, { topic: e.target.value })
                      }
                    />
                  </div>
                  <div className="grid grid-cols-2 gap-4">
                    <div>
                      <Label>Value Schema Format</Label>
                      <Select
                        value={topic.value_schema.format}
                        onValueChange={(value: 'json' | 'avro' | 'protobuf') =>
                          updateKafkaTopic(index, {
                            value_schema: { ...topic.value_schema, format: value },
                          })
                        }
                      >
                        <SelectTrigger>
                          <SelectValue />
                        </SelectTrigger>
                        <SelectContent>
                          <SelectItem value="json">JSON</SelectItem>
                          <SelectItem value="avro">Avro</SelectItem>
                          <SelectItem value="protobuf">Protobuf</SelectItem>
                        </SelectContent>
                      </Select>
                    </div>
                  </div>
                  <div>
                    <Label>Value Schema</Label>
                    <Textarea
                      placeholder='{"type": "object", "properties": {...}}'
                      value={JSON.stringify(topic.value_schema.schema, null, 2)}
                      onChange={(e) => {
                        try {
                          const schema = JSON.parse(e.target.value);
                          updateKafkaTopic(index, {
                            value_schema: { ...topic.value_schema, schema },
                          });
                        } catch {
                          // Invalid JSON, ignore
                        }
                      }}
                      rows={6}
                      className="font-mono text-xs"
                    />
                  </div>
                </div>
              </ModernCard>
            ))}
            <Button variant="outline" onClick={addKafkaTopic}>
              <Plus className="w-4 h-4 mr-2" />
              Add Topic
            </Button>
          </div>
        </Section>
      )}

      {/* Actions */}
      <div className="flex justify-end gap-4">
        {onClose && (
          <Button variant="outline" onClick={onClose}>
            Cancel
          </Button>
        )}
        <Button
          onClick={() => createMutation.mutate()}
          disabled={!contractId || !version || createMutation.isPending}
        >
          <Save className="w-4 h-4 mr-2" />
          {createMutation.isPending ? 'Creating...' : 'Create Contract'}
        </Button>
      </div>
    </div>
  );
}
