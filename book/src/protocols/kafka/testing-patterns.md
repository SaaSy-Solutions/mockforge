# Kafka Testing Patterns

Common testing patterns and scenarios for Kafka-based applications.

## Consumer Group Testing

### Testing Rebalancing

```bash
# Start consumer group
mockforge kafka consume --topic orders --group order-processor

# In another terminal, trigger rebalance
mockforge kafka simulate rebalance --group order-processor

# Observe rebalancing behavior in application logs
```

### Consumer Lag Simulation

```bash
# Simulate consumer falling behind
mockforge kafka simulate lag --group order-processor --topic orders --lag 1000

# Monitor lag monitoring systems
# Test alert thresholds
# Verify catch-up behavior
```

### Offset Reset Testing

```bash
# Reset to earliest
mockforge kafka simulate reset-offsets --group order-processor --topic orders --to earliest

# Reset to latest
mockforge kafka simulate reset-offsets --group order-processor --topic orders --to latest

# Verify application handles offset resets correctly
```

## Message Ordering Testing

### Out-of-Order Messages

```bash
# Produce messages with timestamps out of order
mockforge kafka produce --topic events --value '{"id": "1", "timestamp": "2024-01-01T10:00:00Z"}'
mockforge kafka produce --topic events --value '{"id": "2", "timestamp": "2024-01-01T09:00:00Z"}'
mockforge kafka produce --topic events --value '{"id": "3", "timestamp": "2024-01-01T11:00:00Z"}'

# Test consumer handles ordering correctly
```

### Duplicate Messages

```bash
# Produce same message multiple times
for i in {1..3}; do
  mockforge kafka produce --topic orders --key "order-123" --value '{"id": "123", "amount": 100}'
done

# Test idempotency and duplicate handling
```

## Partition Testing

### Partition Assignment

```bash
# Create topic with multiple partitions
mockforge kafka topic create orders --partitions 6

# Produce messages with different keys
mockforge kafka produce --topic orders --key "customer-1" --value "order data"
mockforge kafka produce --topic orders --key "customer-2" --value "order data"

# Verify messages go to correct partitions
mockforge kafka consume --topic orders --partition 0
```

### Partition Failures

```bash
# Simulate partition unavailability (future feature)
# Test consumer failover behavior
# Verify data consistency across partitions
```

## Performance Testing

### Throughput Testing

```bash
# High-volume message production
mockforge kafka produce --topic high-volume --value "test message" --count 10000 --batch-size 100

# Measure consumer throughput
mockforge kafka consume --topic high-volume --group perf-test --benchmark
```

### Latency Testing

```bash
# Measure end-to-end latency
time mockforge kafka produce --topic latency-test --value "ping"

# Consumer measures processing time
mockforge kafka consume --topic latency-test --measure-latency
```

## Error Scenario Testing

### Network Partition Simulation

```bash
# Simulate network issues (future feature)
# Test reconnection logic
# Verify message delivery guarantees
```

### Broker Failure Simulation

```bash
# Simulate broker unavailability
# Test failover to other brokers
# Verify leader election (future feature)
```

## Integration Testing

### Multi-Service Testing

```bash
# Start multiple mock services
mockforge serve --kafka --http --grpc

# Test end-to-end message flow
# Order service -> Kafka -> Payment service -> Kafka -> Notification service
```

### Schema Evolution Testing

```bash
# Produce messages with different schemas
mockforge kafka produce --topic orders --value '{"v1": "data"}'
mockforge kafka produce --topic orders --value '{"v1": "data", "v2": "new_field"}'

# Test consumer handles schema changes
```

## Load Testing

### Gradual Load Increase

```bash
# Start with low rate
mockforge kafka fixtures start-auto-produce --rate 10

# Gradually increase
mockforge kafka fixtures update-rate --rate 50
mockforge kafka fixtures update-rate --rate 100
mockforge kafka fixtures update-rate --rate 500

# Monitor system performance
```

### Burst Load Testing

```bash
# Simulate traffic spikes
mockforge kafka produce --topic burst-test --value "burst message" --count 1000 --rate 100

# Test autoscaling behavior
# Monitor queue depths
```

## Monitoring and Observables

### Consumer Lag Monitoring

```bash
# Monitor consumer group lag
mockforge kafka groups offsets test-group

# Set up alerts for high lag
# Test alert thresholds
```

### Throughput Monitoring

```bash
# Monitor message rates
mockforge kafka topic describe orders --metrics

# Track partition distribution
# Monitor broker performance
```

## Best Practices

1. **Test Realistic Scenarios**: Use production-like message volumes and patterns
2. **Monitor System Behavior**: Track latency, throughput, and error rates
3. **Test Failure Modes**: Network issues, broker failures, consumer problems
4. **Validate Data Consistency**: Ensure messages are processed correctly
5. **Performance Baseline**: Establish performance expectations
6. **Continuous Testing**: Integrate Kafka tests into CI/CD pipeline