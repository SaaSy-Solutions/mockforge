# Deploying MockForge on AWS

This guide covers deploying MockForge on Amazon Web Services using various services.

## Table of Contents

- [AWS ECS with Fargate](#aws-ecs-with-fargate)
- [AWS EKS (Kubernetes)](#aws-eks-kubernetes)
- [AWS App Runner](#aws-app-runner)
- [AWS EC2](#aws-ec2)
- [Cost Estimation](#cost-estimation)

## AWS ECS with Fargate

AWS Fargate is a serverless compute engine for containers. This is the recommended approach for production deployments.

### Prerequisites

- AWS CLI installed and configured
- Docker installed locally
- An AWS account with appropriate permissions

### Step 1: Push Docker Image to ECR

```bash
# Create ECR repository
aws ecr create-repository --repository-name mockforge --region us-east-1

# Get login token
aws ecr get-login-password --region us-east-1 | \
  docker login --username AWS --password-stdin \
  123456789012.dkr.ecr.us-east-1.amazonaws.com

# Tag and push image
docker pull ghcr.io/saasy-solutions/mockforge:latest
docker tag ghcr.io/saasy-solutions/mockforge:latest \
  123456789012.dkr.ecr.us-east-1.amazonaws.com/mockforge:latest
docker push 123456789012.dkr.ecr.us-east-1.amazonaws.com/mockforge:latest
```

### Step 2: Create ECS Task Definition

Create `task-definition.json`:

```json
{
  "family": "mockforge",
  "networkMode": "awsvpc",
  "requiresCompatibilities": ["FARGATE"],
  "cpu": "512",
  "memory": "1024",
  "executionRoleArn": "arn:aws:iam::123456789012:role/ecsTaskExecutionRole",
  "containerDefinitions": [
    {
      "name": "mockforge",
      "image": "123456789012.dkr.ecr.us-east-1.amazonaws.com/mockforge:latest",
      "portMappings": [
        {
          "containerPort": 3000,
          "protocol": "tcp"
        },
        {
          "containerPort": 3001,
          "protocol": "tcp"
        },
        {
          "containerPort": 9080,
          "protocol": "tcp"
        }
      ],
      "environment": [
        {
          "name": "MOCKFORGE_HTTP_PORT",
          "value": "3000"
        },
        {
          "name": "MOCKFORGE_WS_PORT",
          "value": "3001"
        },
        {
          "name": "MOCKFORGE_ADMIN_PORT",
          "value": "9080"
        },
        {
          "name": "MOCKFORGE_ADMIN_ENABLED",
          "value": "true"
        },
        {
          "name": "RUST_LOG",
          "value": "info"
        }
      ],
      "logConfiguration": {
        "logDriver": "awslogs",
        "options": {
          "awslogs-group": "/ecs/mockforge",
          "awslogs-region": "us-east-1",
          "awslogs-stream-prefix": "ecs"
        }
      },
      "healthCheck": {
        "command": ["CMD-SHELL", "curl -f http://localhost:9080/health/live || exit 1"],
        "interval": 30,
        "timeout": 5,
        "retries": 3,
        "startPeriod": 60
      }
    }
  ]
}
```

### Step 3: Create CloudWatch Log Group

```bash
aws logs create-log-group --log-group-name /ecs/mockforge --region us-east-1
```

### Step 4: Register Task Definition

```bash
aws ecs register-task-definition \
  --cli-input-json file://task-definition.json \
  --region us-east-1
```

### Step 5: Create ECS Cluster

```bash
aws ecs create-cluster \
  --cluster-name mockforge-cluster \
  --region us-east-1
```

### Step 6: Create Application Load Balancer

```bash
# Create ALB
aws elbv2 create-load-balancer \
  --name mockforge-alb \
  --subnets subnet-12345678 subnet-87654321 \
  --security-groups sg-12345678 \
  --region us-east-1

# Create target group
aws elbv2 create-target-group \
  --name mockforge-tg \
  --protocol HTTP \
  --port 3000 \
  --vpc-id vpc-12345678 \
  --target-type ip \
  --health-check-path /ping \
  --region us-east-1

# Create listener
aws elbv2 create-listener \
  --load-balancer-arn arn:aws:elasticloadbalancing:... \
  --protocol HTTP \
  --port 80 \
  --default-actions Type=forward,TargetGroupArn=arn:aws:elasticloadbalancing:...
```

### Step 7: Create ECS Service

```bash
aws ecs create-service \
  --cluster mockforge-cluster \
  --service-name mockforge-service \
  --task-definition mockforge \
  --desired-count 3 \
  --launch-type FARGATE \
  --network-configuration "awsvpcConfiguration={subnets=[subnet-12345678,subnet-87654321],securityGroups=[sg-12345678],assignPublicIp=ENABLED}" \
  --load-balancers "targetGroupArn=arn:aws:elasticloadbalancing:...,containerName=mockforge,containerPort=3000" \
  --region us-east-1
```

### Using CloudFormation

Save this as `cloudformation-template.yaml`:

```yaml
AWSTemplateFormatVersion: '2010-09-09'
Description: 'MockForge ECS Fargate Deployment'

Parameters:
  VpcId:
    Type: AWS::EC2::VPC::Id
    Description: VPC ID for deployment

  SubnetIds:
    Type: List<AWS::EC2::Subnet::Id>
    Description: Subnet IDs for deployment

  ImageUri:
    Type: String
    Description: Docker image URI
    Default: ghcr.io/saasy-solutions/mockforge:latest

Resources:
  ECSCluster:
    Type: AWS::ECS::Cluster
    Properties:
      ClusterName: mockforge-cluster

  TaskDefinition:
    Type: AWS::ECS::TaskDefinition
    Properties:
      Family: mockforge
      NetworkMode: awsvpc
      RequiresCompatibilities:
        - FARGATE
      Cpu: 512
      Memory: 1024
      ExecutionRoleArn: !GetAtt ExecutionRole.Arn
      ContainerDefinitions:
        - Name: mockforge
          Image: !Ref ImageUri
          PortMappings:
            - ContainerPort: 3000
            - ContainerPort: 3001
            - ContainerPort: 9080
          Environment:
            - Name: MOCKFORGE_HTTP_PORT
              Value: "3000"
            - Name: MOCKFORGE_ADMIN_ENABLED
              Value: "true"
          LogConfiguration:
            LogDriver: awslogs
            Options:
              awslogs-group: !Ref LogGroup
              awslogs-region: !Ref AWS::Region
              awslogs-stream-prefix: ecs

  ExecutionRole:
    Type: AWS::IAM::Role
    Properties:
      AssumeRolePolicyDocument:
        Statement:
          - Effect: Allow
            Principal:
              Service: ecs-tasks.amazonaws.com
            Action: sts:AssumeRole
      ManagedPolicyArns:
        - arn:aws:iam::aws:policy/service-role/AmazonECSTaskExecutionRolePolicy

  LogGroup:
    Type: AWS::Logs::LogGroup
    Properties:
      LogGroupName: /ecs/mockforge
      RetentionInDays: 30

  LoadBalancer:
    Type: AWS::ElasticLoadBalancingV2::LoadBalancer
    Properties:
      Name: mockforge-alb
      Subnets: !Ref SubnetIds
      SecurityGroups:
        - !Ref LoadBalancerSecurityGroup

  TargetGroup:
    Type: AWS::ElasticLoadBalancingV2::TargetGroup
    Properties:
      Name: mockforge-tg
      Port: 3000
      Protocol: HTTP
      VpcId: !Ref VpcId
      TargetType: ip
      HealthCheckPath: /ping

  Listener:
    Type: AWS::ElasticLoadBalancingV2::Listener
    Properties:
      LoadBalancerArn: !Ref LoadBalancer
      Port: 80
      Protocol: HTTP
      DefaultActions:
        - Type: forward
          TargetGroupArn: !Ref TargetGroup

  Service:
    Type: AWS::ECS::Service
    DependsOn: Listener
    Properties:
      ServiceName: mockforge-service
      Cluster: !Ref ECSCluster
      TaskDefinition: !Ref TaskDefinition
      DesiredCount: 3
      LaunchType: FARGATE
      NetworkConfiguration:
        AwsvpcConfiguration:
          AssignPublicIp: ENABLED
          Subnets: !Ref SubnetIds
          SecurityGroups:
            - !Ref ServiceSecurityGroup
      LoadBalancers:
        - ContainerName: mockforge
          ContainerPort: 3000
          TargetGroupArn: !Ref TargetGroup

  LoadBalancerSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Security group for ALB
      VpcId: !Ref VpcId
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 80
          ToPort: 80
          CidrIp: 0.0.0.0/0

  ServiceSecurityGroup:
    Type: AWS::EC2::SecurityGroup
    Properties:
      GroupDescription: Security group for ECS service
      VpcId: !Ref VpcId
      SecurityGroupIngress:
        - IpProtocol: tcp
          FromPort: 3000
          ToPort: 3000
          SourceSecurityGroupId: !Ref LoadBalancerSecurityGroup

Outputs:
  LoadBalancerDNS:
    Description: DNS name of the load balancer
    Value: !GetAtt LoadBalancer.DNSName
```

Deploy with:

```bash
aws cloudformation create-stack \
  --stack-name mockforge \
  --template-body file://cloudformation-template.yaml \
  --parameters ParameterKey=VpcId,ParameterValue=vpc-12345678 \
               ParameterKey=SubnetIds,ParameterValue="subnet-12345678,subnet-87654321" \
  --capabilities CAPABILITY_IAM \
  --region us-east-1
```

## AWS EKS (Kubernetes)

Deploy MockForge on Amazon EKS using Helm.

### Prerequisites

- `eksctl` installed
- `kubectl` installed
- `helm` installed

### Step 1: Create EKS Cluster

```bash
eksctl create cluster \
  --name mockforge-cluster \
  --region us-east-1 \
  --nodegroup-name standard-workers \
  --node-type t3.medium \
  --nodes 3 \
  --nodes-min 3 \
  --nodes-max 10 \
  --managed
```

### Step 2: Configure kubectl

```bash
aws eks update-kubeconfig \
  --name mockforge-cluster \
  --region us-east-1
```

### Step 3: Install MockForge with Helm

```bash
# From local chart
helm install mockforge ./helm/mockforge \
  --set image.repository=ghcr.io/saasy-solutions/mockforge \
  --set image.tag=latest \
  --set ingress.enabled=true \
  --set ingress.className=alb

# Or from repository (when published)
helm repo add mockforge https://charts.mockforge.dev
helm install mockforge mockforge/mockforge
```

### Step 4: Set up AWS Load Balancer Controller

```bash
# Install AWS Load Balancer Controller
helm repo add eks https://aws.github.io/eks-charts
helm install aws-load-balancer-controller eks/aws-load-balancer-controller \
  -n kube-system \
  --set clusterName=mockforge-cluster
```

## AWS App Runner

AWS App Runner is the simplest option for running MockForge.

```bash
# Create App Runner service
aws apprunner create-service \
  --service-name mockforge \
  --source-configuration '{
    "ImageRepository": {
      "ImageIdentifier": "ghcr.io/saasy-solutions/mockforge:latest",
      "ImageConfiguration": {
        "Port": "3000",
        "RuntimeEnvironmentVariables": {
          "MOCKFORGE_HTTP_PORT": "3000",
          "MOCKFORGE_ADMIN_ENABLED": "true"
        }
      },
      "ImageRepositoryType": "ECR_PUBLIC"
    },
    "AutoDeploymentsEnabled": false
  }' \
  --instance-configuration '{
    "Cpu": "1 vCPU",
    "Memory": "2 GB"
  }' \
  --region us-east-1
```

## AWS EC2

For traditional VM deployment:

```bash
# Launch EC2 instance
aws ec2 run-instances \
  --image-id ami-0c55b159cbfafe1f0 \
  --instance-type t3.medium \
  --key-name my-key-pair \
  --security-group-ids sg-12345678 \
  --subnet-id subnet-12345678 \
  --user-data file://user-data.sh \
  --region us-east-1
```

User data script (`user-data.sh`):

```bash
#!/bin/bash
yum update -y
yum install -y docker
systemctl start docker
systemctl enable docker

docker run -d \
  --name mockforge \
  --restart unless-stopped \
  -p 80:3000 \
  -p 9080:9080 \
  -e MOCKFORGE_HTTP_PORT=3000 \
  -e MOCKFORGE_ADMIN_ENABLED=true \
  ghcr.io/saasy-solutions/mockforge:latest
```

## Cost Estimation

### ECS Fargate (3 tasks, 0.5 vCPU, 1 GB RAM)
- **Compute:** ~$30/month per task = $90/month
- **Load Balancer:** ~$20/month
- **Data Transfer:** ~$10-50/month
- **Total:** ~$120-160/month

### EKS (3 nodes, t3.medium)
- **Control Plane:** $73/month
- **Worker Nodes:** ~$30/month per node = $90/month
- **Load Balancer:** ~$20/month
- **Total:** ~$183/month

### App Runner (1 vCPU, 2 GB RAM)
- **Compute:** ~$0.064/hour = $47/month
- **Memory:** ~$0.007/GB-hour = $10/month
- **Total:** ~$57/month (cheapest option)

### EC2 (t3.medium)
- **Instance:** ~$30/month
- **Storage:** ~$5/month
- **Total:** ~$35/month (requires more management)

## Best Practices

1. **Use AWS Secrets Manager** for sensitive configuration
2. **Enable CloudWatch Logs** for centralized logging
3. **Set up CloudWatch Alarms** for monitoring
4. **Use Auto Scaling** for production workloads
5. **Enable AWS X-Ray** for distributed tracing
6. **Use VPC endpoints** to reduce data transfer costs
7. **Implement backup strategies** for persistent data

## Monitoring

### CloudWatch Dashboard

Create a custom dashboard:

```bash
aws cloudwatch put-dashboard \
  --dashboard-name MockForge \
  --dashboard-body file://dashboard.json
```

### CloudWatch Alarms

```bash
# High CPU alarm
aws cloudwatch put-metric-alarm \
  --alarm-name mockforge-high-cpu \
  --alarm-description "Alert when CPU exceeds 80%" \
  --metric-name CPUUtilization \
  --namespace AWS/ECS \
  --statistic Average \
  --period 300 \
  --evaluation-periods 2 \
  --threshold 80 \
  --comparison-operator GreaterThanThreshold
```

## Troubleshooting

### View ECS logs

```bash
aws logs tail /ecs/mockforge --follow --region us-east-1
```

### Check service health

```bash
aws ecs describe-services \
  --cluster mockforge-cluster \
  --services mockforge-service \
  --region us-east-1
```

### Debug task failures

```bash
aws ecs describe-tasks \
  --cluster mockforge-cluster \
  --tasks <task-arn> \
  --region us-east-1
```

## Additional Resources

- [AWS ECS Documentation](https://docs.aws.amazon.com/ecs/)
- [AWS EKS Documentation](https://docs.aws.amazon.com/eks/)
- [AWS App Runner Documentation](https://docs.aws.amazon.com/apprunner/)
