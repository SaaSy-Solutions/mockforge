# MockForge Cloud Launch Checklist - Budget-Friendly VPS Self-Hosting

**Purpose:** Step-by-step guide for launching MockForge Cloud with marketplaces on a budget
**Last Updated:** 2025-01-27
**Status:** Ready for Launch
**Total Estimated Cost:** $5-10/month (VPS) + $0-21/month (external services) = **$5-31/month**

## Budget Breakdown

- **VPS Hosting:** $5-10/month (Hetzner CPX11: $4.99/month for 2GB RAM, or CPX21: $9.99/month for 4GB RAM, US data center)
- **Domain:** ~$10-15/year (~$1/month)
- **Email Service:** $0 (Resend/SendGrid free tier)
- **DNS/CDN:** $0 (Cloudflare free tier)
- **Object Storage:** $0 (Cloudflare R2 free 10GB, then $0.015/GB/month)
- **Database:** $0 (self-hosted PostgreSQL on VPS)
- **Monitoring:** $0 (self-hosted Prometheus + Grafana)
- **SSL Certificates:** $0 (Let's Encrypt)
- **AI Models:** $0-20/month (OpenAI free tier $5 credit, or Ollama self-hosted)
- **Payment Processing:** 2.9% + $0.30 per transaction (Stripe - only when users pay)

**Total: ~$5-31/month** (depending on VPS size and AI usage)

---

## Table of Contents

1. [Pre-Launch (4-6 weeks before)](#pre-launch-4-6-weeks-before)
2. [Launch Week (1 week before)](#launch-week-1-week-before)
3. [Launch Day](#launch-day)
4. [Post-Launch (First Week)](#post-launch-first-week)
5. [Success Criteria](#success-criteria)
6. [Rollback Plan](#rollback-plan)

---

## Pre-Launch (4-6 weeks before)

### Infrastructure

#### VPS Provider Selection and Setup

**Cost:** $5-10/month (CPX11: $4.99/mo for 2GB RAM, CPX21: $9.99/mo for 4GB RAM)

**Steps:**

1. **Choose VPS Provider** (Recommended: Hetzner for US users)
   ```bash
   # Option 1: Hetzner (Recommended - Best Price/Performance)
   # - Sign up at https://www.hetzner.com/cloud
   # - CPX11: $4.99/month
   #   - 2 vCPU, 2GB RAM, 40GB SSD, 1TB traffic
   #   - Good for light workloads
   # - CPX21: $9.99/month (Recommended for production with 4GB RAM)
   #   - 3 vCPU, 4GB RAM, 80GB SSD, 2TB traffic
   #   - Better for production workloads requiring more memory
   # - US data center: Ashburn, Virginia (us-east)
   # - EU data centers also available (Nuremberg, Falkenstein, Helsinki)
   # - Best value for money
   # - Choose US location for lower latency if serving US users
   # - Note: Hetzner doesn't allow custom RAM - you must choose from predefined instance types

   # Option 2: Vultr (Good US coverage)
   # - Sign up at https://www.vultr.com
   # - Regular Performance: $6/month
   #   - 1 vCPU, 1GB RAM, 25GB SSD
   #   - Multiple US regions: New York, Chicago, Dallas, Seattle, etc.
   #   - Good for US-based users

   # Option 3: DigitalOcean (US-focused)
   # - Sign up at https://www.digitalocean.com
   # - Basic Droplet: $6/month
   #   - 1 vCPU, 1GB RAM, 25GB SSD
   #   - US regions: NYC, San Francisco, etc.
   #   - Good documentation and community

   # Note: For US users, choose US data centers for lower latency.
   # EU data centers are better if you need GDPR compliance for EU users.
   ```

2. **Provision VPS Instance**
   ```bash
   # Hetzner example (using hcloud CLI) - US data center
   # Install hcloud CLI: https://github.com/hetznercloud/cli

   # For 2GB RAM (CPX11):
   hcloud server create \
     --name mockforge-prod \
     --type cpx11 \
     --image ubuntu-24.04 \
     --location ash  \
     --ssh-key your-ssh-key-id

   # For 4GB RAM (CPX21 - recommended for production):
   hcloud server create \
     --name mockforge-prod \
     --type cpx21 \
     --image ubuntu-24.04 \
     --location ash  \
     --ssh-key your-ssh-key-id

   # Location codes:
   # - ash = Ashburn, Virginia, US (recommended for US users)
   # - nbg1 = Nuremberg, Germany, EU
   # - fsn1 = Falkenstein, Germany, EU
   # - hel1 = Helsinki, Finland, EU (hel1 is a data center code, not a typo)

   # Or use web console:
   # 1. Go to Hetzner Cloud Console
   # 2. Create new project: "MockForge"
   # 3. Add new server
   # 4. Select server type:
   #    - CPX11 for 2GB RAM ($4.99/mo) - 2 vCPU, 2GB RAM, 40GB SSD
   #    - CPX21 for 4GB RAM ($9.99/mo) - 3 vCPU, 4GB RAM, 80GB SSD (recommended)
   # 5. Select image: Ubuntu 24.04 LTS
   # 6. Choose location: "Ashburn, Virginia" (for US) or EU location (for GDPR)
   # 7. Add SSH key
   # 8. Create server
   ```

3. **Configure SSH Access**
   ```bash
   # Get server IP
   hcloud server list

   # SSH into server
   ssh root@YOUR_SERVER_IP

   # Or if using key-based auth
   ssh -i ~/.ssh/your-key root@YOUR_SERVER_IP

   # Create non-root user (recommended)
   adduser mockforge
   usermod -aG sudo mockforge
   mkdir -p /home/mockforge/.ssh
   cp ~/.ssh/authorized_keys /home/mockforge/.ssh/
   chown -R mockforge:mockforge /home/mockforge/.ssh
   ```

4. **Set Up Basic Security**
   ```bash
   # Update system
   apt update && apt upgrade -y

   # Install fail2ban
   apt install -y fail2ban
   systemctl enable fail2ban
   systemctl start fail2ban

   # Configure firewall (UFW)
   apt install -y ufw
   ufw allow 22/tcp    # SSH
   ufw allow 80/tcp    # HTTP
   ufw allow 443/tcp   # HTTPS
   ufw enable
   ufw status
   ```

**Verification:**
- [ ] VPS provider account created
- [ ] VPS instance provisioned and accessible
- [ ] SSH access configured
- [ ] Basic security (firewall, fail2ban) configured
- [ ] Server IP address noted

---

#### Domain & DNS (Cloudflare Free Tier)

**Cost:** $0 (Cloudflare free tier)

**Steps:**

1. **Register Domain**
   ```bash
   # Recommended: Cloudflare Registrar or Namecheap
   # - Cloudflare: ~$10-15/year (at-cost pricing)
   # - Namecheap: ~$10-15/year
   # - Register: mockforge.dev (or your chosen domain)
   ```

2. **Add Domain to Cloudflare (Free Tier)**
   ```bash
   # 1. Sign up at https://dash.cloudflare.com (free account)
   # 2. Click "Add a Site"
   # 3. Enter your domain name
   # 4. Select "Free" plan
   # 5. Cloudflare will scan existing DNS records
   # 6. Update nameservers at your registrar to Cloudflare's nameservers
   #    (provided in Cloudflare dashboard)
   ```

3. **Configure DNS Records**
   ```bash
   # In Cloudflare DNS dashboard, add:

   # A record: app.mockforge.dev -> YOUR_VPS_IP (Proxy enabled - orange cloud)
   # A record: api.mockforge.dev -> YOUR_VPS_IP (Proxy enabled - orange cloud)
   # CNAME: www.mockforge.dev -> mockforge.dev (Proxy enabled)
   # CNAME: status.mockforge.dev -> mockforge.dev (Proxy enabled)

   # Wait for DNS propagation (usually 5-30 minutes)
   # Verify: dig app.mockforge.dev
   ```

4. **Configure Cloudflare SSL/TLS**
   ```bash
   # In Cloudflare dashboard:
   # 1. Go to SSL/TLS settings
   # 2. Set encryption mode to "Full (strict)"
   # 3. Enable "Always Use HTTPS"
   # 4. Enable "Automatic HTTPS Rewrites"
   # 5. Cloudflare provides free SSL certificates automatically
   ```

5. **Configure Cloudflare Caching (Optional)**
   ```bash
   # In Cloudflare dashboard:
   # 1. Go to Caching settings
   # 2. Set caching level to "Standard"
   # 3. Configure page rules for static assets if needed
   # 4. Enable "Browser Cache TTL" (4 hours recommended)
   ```

**Verification:**
- [ ] Domain registered
- [ ] Domain added to Cloudflare
- [ ] Nameservers updated at registrar
- [ ] DNS records configured and propagated
- [ ] SSL/TLS mode set to "Full (strict)"
- [ ] Test DNS: `dig app.mockforge.dev`
- [ ] Test HTTPS: `curl -I https://app.mockforge.dev`

---

#### VPS Infrastructure Deployment (Docker Compose)

**Cost:** $0 (self-hosted on VPS)

**Steps:**

1. **Install Docker and Docker Compose on VPS**
   ```bash
   # SSH into your VPS
   ssh mockforge@YOUR_SERVER_IP

   # Install Docker
   curl -fsSL https://get.docker.com -o get-docker.sh
   sudo sh get-docker.sh

   # Add user to docker group
   sudo usermod -aG docker mockforge
   newgrp docker  # Or logout/login

   # Install Docker Compose
   sudo curl -L "https://github.com/docker/compose/releases/latest/download/docker-compose-$(uname -s)-$(uname -m)" -o /usr/local/bin/docker-compose
   sudo chmod +x /usr/local/bin/docker-compose

   # Verify installation
   docker --version
   docker-compose --version
   ```

2. **Create Project Directory Structure**
   ```bash
   # Create project directory
   sudo mkdir -p /opt/mockforge
   sudo chown mockforge:mockforge /opt/mockforge
   cd /opt/mockforge

   # Create directories
   mkdir -p {data,logs,config,nginx,ssl,backups}
   ```

3. **Create Docker Compose Configuration**
   ```bash
   # Create docker-compose.yml
   cat > docker-compose.yml << 'EOF'
   version: '3.8'

   services:
     # PostgreSQL Database
     postgres:
       image: postgres:15-alpine
       container_name: mockforge-postgres
       environment:
         POSTGRES_DB: mockforge
         POSTGRES_USER: mockforge
         POSTGRES_PASSWORD: ${POSTGRES_PASSWORD}
       volumes:
         - postgres_data:/var/lib/postgresql/data
       networks:
         - mockforge-network
       restart: unless-stopped
       healthcheck:
         test: ["CMD-SHELL", "pg_isready -U mockforge"]
         interval: 10s
         timeout: 5s
         retries: 5

     # MockForge Registry Server
     registry-server:
       image: ghcr.io/saasy-solutions/mockforge-registry-server:latest
       container_name: mockforge-registry-server
       depends_on:
         postgres:
           condition: service_healthy
       environment:
         # Database
         DATABASE_URL: postgresql://mockforge:${POSTGRES_PASSWORD}@postgres:5432/mockforge
         # Object Storage (Cloudflare R2 - S3-compatible)
         S3_ENDPOINT: ${S3_ENDPOINT}
         S3_ACCESS_KEY: ${S3_ACCESS_KEY}
         S3_SECRET_KEY: ${S3_SECRET_KEY}
         S3_BUCKET: ${S3_BUCKET}
         S3_REGION: ${S3_REGION}
         S3_USE_SSL: "true"
         # Application
         RUST_LOG: info
         APP_BASE_URL: https://app.mockforge.dev
         API_BASE_URL: https://api.mockforge.dev
         # JWT
         JWT_SECRET: ${JWT_SECRET}
         # Email (Resend free tier)
         EMAIL_PROVIDER: resend
         RESEND_API_KEY: ${RESEND_API_KEY}
         # AI Models (OpenAI/Anthropic/Ollama)
         MOCKFORGE_RAG_ENABLED: ${MOCKFORGE_RAG_ENABLED:-false}
         MOCKFORGE_RAG_PROVIDER: ${MOCKFORGE_RAG_PROVIDER}
         MOCKFORGE_RAG_API_KEY: ${MOCKFORGE_RAG_API_KEY}
         MOCKFORGE_RAG_API_ENDPOINT: ${MOCKFORGE_RAG_API_ENDPOINT}
         MOCKFORGE_RAG_MODEL: ${MOCKFORGE_RAG_MODEL}
         # Stripe (for billing)
         STRIPE_SECRET_KEY: ${STRIPE_SECRET_KEY}
         STRIPE_WEBHOOK_SECRET: ${STRIPE_WEBHOOK_SECRET}
       volumes:
         - ./config:/app/config:ro
         - ./logs:/app/logs
       networks:
         - mockforge-network
       restart: unless-stopped
       healthcheck:
         test: ["CMD", "curl", "-f", "http://localhost:3000/health"]
         interval: 30s
         timeout: 10s
         retries: 3
         start_period: 40s

     # Nginx Reverse Proxy
     nginx:
       image: nginx:alpine
       container_name: mockforge-nginx
       depends_on:
         - registry-server
       ports:
         - "80:80"
         - "443:443"
       volumes:
         - ./nginx/nginx.conf:/etc/nginx/nginx.conf:ro
         - ./nginx/ssl:/etc/nginx/ssl:ro
         - certbot-etc:/etc/letsencrypt
         - certbot-var:/var/lib/letsencrypt
       networks:
         - mockforge-network
       restart: unless-stopped

     # Certbot for SSL certificates
     certbot:
       image: certbot/certbot
       container_name: mockforge-certbot
       volumes:
         - certbot-etc:/etc/letsencrypt
         - certbot-var:/var/lib/letsencrypt
         - ./nginx/ssl:/etc/nginx/ssl
       entrypoint: "/bin/sh -c 'trap exit TERM; while :; do certbot renew; sleep 12h & wait $${!}; done;'"

     # Prometheus (Monitoring)
     prometheus:
       image: prom/prometheus:latest
       container_name: mockforge-prometheus
       volumes:
         - ./config/prometheus.yml:/etc/prometheus/prometheus.yml:ro
         - prometheus_data:/prometheus
       command:
         - '--config.file=/etc/prometheus/prometheus.yml'
         - '--storage.tsdb.path=/prometheus'
       networks:
         - mockforge-network
       restart: unless-stopped

     # Grafana (Monitoring Dashboard)
     grafana:
       image: grafana/grafana:latest
       container_name: mockforge-grafana
       environment:
         GF_SECURITY_ADMIN_PASSWORD: ${GRAFANA_PASSWORD}
         GF_SERVER_ROOT_URL: https://grafana.mockforge.dev
       volumes:
         - grafana_data:/var/lib/grafana
       networks:
         - mockforge-network
       restart: unless-stopped

   volumes:
     postgres_data:
     prometheus_data:
     grafana_data:
     certbot-etc:
     certbot-var:

   networks:
     mockforge-network:
       driver: bridge
   EOF
   ```

4. **Create Environment File**
   ```bash
   # Create .env file with secrets
   # Note: This is a template - actual values will be generated below

   # Generate actual passwords
   echo "POSTGRES_PASSWORD=$(openssl rand -base64 32)" > .env
   echo "JWT_SECRET=$(openssl rand -base64 64)" >> .env
   echo "GRAFANA_PASSWORD=$(openssl rand -base64 32)" >> .env
   echo "" >> .env
   echo "# Object Storage (Cloudflare R2)" >> .env
   echo "S3_ENDPOINT=" >> .env
   echo "S3_ACCESS_KEY=" >> .env
   echo "S3_SECRET_KEY=" >> .env
   echo "S3_BUCKET=mockforge-marketplace" >> .env
   echo "S3_REGION=auto" >> .env
   echo "" >> .env
   echo "# AI Models (optional)" >> .env
   echo "MOCKFORGE_RAG_ENABLED=false" >> .env
   echo "MOCKFORGE_RAG_PROVIDER=" >> .env
   echo "MOCKFORGE_RAG_API_KEY=" >> .env
   echo "MOCKFORGE_RAG_API_ENDPOINT=" >> .env
   echo "MOCKFORGE_RAG_MODEL=" >> .env
   echo "" >> .env
   echo "# Add your API keys:" >> .env
   echo "RESEND_API_KEY=" >> .env
   echo "STRIPE_SECRET_KEY=" >> .env
   echo "STRIPE_WEBHOOK_SECRET=" >> .env

   # Secure the file
   chmod 600 .env
   ```

5. **Create Nginx Configuration**
   ```bash
   # Create nginx config
   cat > nginx/nginx.conf << 'EOF'
   events {
       worker_connections 1024;
   }

   http {
       upstream registry_server {
           server registry-server:3000;
       }

       # Redirect HTTP to HTTPS
       server {
           listen 80;
           server_name app.mockforge.dev api.mockforge.dev;
           return 301 https://$server_name$request_uri;
       }

       # Main application
       server {
           listen 443 ssl http2;
           server_name app.mockforge.dev api.mockforge.dev;

           ssl_certificate /etc/letsencrypt/live/app.mockforge.dev/fullchain.pem;
           ssl_certificate_key /etc/letsencrypt/live/app.mockforge.dev/privkey.pem;

           ssl_protocols TLSv1.2 TLSv1.3;
           ssl_ciphers HIGH:!aNULL:!MD5;
           ssl_prefer_server_ciphers on;

           location / {
               proxy_pass http://registry_server;
               proxy_set_header Host $host;
               proxy_set_header X-Real-IP $remote_addr;
               proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
               proxy_set_header X-Forwarded-Proto $scheme;
           }
       }
   }
   EOF
   ```

6. **Run Database Migrations**
   ```bash
   # Start registry server temporarily to run migrations
   docker-compose run --rm registry-server \
     /app/migrations/run-migrations.sh

   # Or if migrations are built into the image:
   docker-compose up -d registry-server
   # Check logs: docker-compose logs registry-server
   ```

7. **Obtain SSL Certificates**
   ```bash
   # Stop nginx temporarily
   docker-compose stop nginx

   # Get certificate
   docker run --rm -it \
     -v certbot-etc:/etc/letsencrypt \
     -v certbot-var:/var/lib/letsencrypt \
     -p 80:80 \
     certbot/certbot certonly --standalone \
     -d app.mockforge.dev \
     -d api.mockforge.dev \
     --email your-email@example.com \
     --agree-tos \
     --non-interactive

   # Update nginx config to use certificates
   # (already configured in nginx.conf)

   # Start all services
   docker-compose up -d
   ```

8. **Verify Deployment**
   ```bash
   # Check all services are running
   docker-compose ps

   # Check logs
   docker-compose logs -f registry-server

   # Test health endpoint
   curl http://localhost/health
   curl https://api.mockforge.dev/health

   # Test API endpoints
   curl https://api.mockforge.dev/api/v1/plugins
   ```

**Verification:**
- [ ] Docker and Docker Compose installed
- [ ] All services running (`docker-compose ps`)
- [ ] Database migrations applied
- [ ] SSL certificates obtained and working
- [ ] Health checks passing
- [ ] API endpoints responding
- [ ] Object storage (Cloudflare R2) configured

---

### Security

#### Security Audit

**Steps:**

1. **Run Penetration Testing**
   ```bash
   # Using OWASP ZAP
   docker run -t owasp/zap2docker-stable zap-baseline.py \
     -t https://api.mockforge.dev

   # Using Nuclei
   nuclei -u https://api.mockforge.dev -t cves/

   # Using SQLMap (for SQL injection testing)
   sqlmap -u "https://api.mockforge.dev/api/v1/plugins?q=test" \
     --batch --level=3 --risk=2
   ```

2. **Vulnerability Scanning**
   ```bash
   # Scan Docker images
   trivy image ghcr.io/saasy-solutions/mockforge:latest

   # Scan dependencies
   cargo audit
   cargo deny check

   # Scan infrastructure
   checkov -d deploy/terraform/
   ```

3. **Security Review**
   - Review authentication/authorization flows
   - Verify input validation on all endpoints
   - Check rate limiting configuration
   - Review secrets management
   - Audit logging configuration

4. **Address Remediation Items**
   - Document all findings in `compliance/security-audit-YYYY-MM-DD.md`
   - Prioritize critical/high findings
   - Create tickets for each remediation
   - Verify fixes before launch

**Verification:**
- [ ] Penetration testing completed
- [ ] Vulnerability scanning passed (no critical/high)
- [ ] Security review documented
- [ ] All remediation items addressed
- [ ] Security audit report signed off

---

#### Access Control (VPS Security)

**Cost:** $0 (self-hosted)

**Steps:**

1. **Configure SSH Key Authentication**
   ```bash
   # On your local machine, generate SSH key if needed
   ssh-keygen -t ed25519 -C "mockforge-admin"

   # Copy public key to VPS
   ssh-copy-id -i ~/.ssh/id_ed25519.pub mockforge@YOUR_SERVER_IP

   # Disable password authentication (on VPS)
   sudo nano /etc/ssh/sshd_config
   # Set: PasswordAuthentication no
   # Set: PermitRootLogin no
   sudo systemctl restart sshd
   ```

2. **Set Up Fail2Ban**
   ```bash
   # Already installed in initial setup, configure it:
   sudo nano /etc/fail2ban/jail.local

   # Add configuration:
   [sshd]
   enabled = true
   port = 22
   filter = sshd
   logpath = /var/log/auth.log
   maxretry = 3
   bantime = 3600

   sudo systemctl restart fail2ban
   sudo fail2ban-client status sshd
   ```

3. **Configure Secrets Management**
   ```bash
   # Use .env file (already created in deployment)
   # Ensure it's secured:
   chmod 600 /opt/mockforge/.env

   # Use Docker secrets for sensitive data (optional)
   # Create secrets:
   echo "your-secret" | docker secret create postgres_password -
   echo "your-secret" | docker secret create jwt_secret -

   # Reference in docker-compose.yml:
   # secrets:
   #   - postgres_password
   #   - jwt_secret
   ```

4. **Enable Audit Logging**
   ```bash
   # Configure log rotation
   sudo nano /etc/logrotate.d/mockforge

   # Add:
   /opt/mockforge/logs/*.log {
       daily
       rotate 30
       compress
       delaycompress
       missingok
       notifempty
       create 0640 mockforge mockforge
   }

   # Monitor system logs
   sudo journalctl -u docker -f
   ```

**Verification:**
- [ ] SSH key authentication configured
- [ ] Password authentication disabled
- [ ] Fail2Ban configured and active
- [ ] Secrets stored securely (.env file with 600 permissions)
- [ ] Log rotation configured
- [ ] System logs monitored

---

#### Compliance

**Steps:**

1. **Publish Privacy Policy**
   - Create `docs/legal/privacy-policy.md`
   - Host at `https://mockforge.dev/privacy`
   - Include GDPR compliance statements
   - Add link in footer of all pages

2. **Publish Terms of Service**
   - Create `docs/legal/terms-of-service.md`
   - Host at `https://mockforge.dev/terms`
   - Include service level agreements
   - Add link in footer of all pages

3. **Verify GDPR Compliance**
   - Document data processing activities
   - Implement data subject rights (access, deletion, portability)
   - Add cookie consent banner
   - Document data retention policies

4. **Define Data Retention Policies**
   ```yaml
   # config/data-retention.yaml
   data_retention:
     user_data: 90_days_after_account_deletion
     audit_logs: 7_years
     marketplace_items: indefinite_until_deleted
     hosted_mock_logs: 30_days
     metrics: 90_days
   ```

**Verification:**
- [ ] Privacy policy published and accessible
- [ ] Terms of service published and accessible
- [ ] GDPR compliance verified
- [ ] Data retention policies defined and documented
- [ ] Legal review completed

---

### Monitoring & Operations

#### Monitoring Setup (Self-Hosted Prometheus + Grafana)

**Cost:** $0 (self-hosted on VPS)

**Steps:**

1. **Configure Prometheus** (Already in docker-compose.yml)
   ```bash
   # Create Prometheus configuration
   cat > config/prometheus.yml << 'EOF'
   global:
     scrape_interval: 15s
     evaluation_interval: 15s

   scrape_configs:
     - job_name: 'mockforge-registry-server'
       static_configs:
         - targets: ['registry-server:3000']
       metrics_path: '/metrics'

     - job_name: 'postgres'
       static_configs:
         - targets: ['postgres-exporter:9187']

     - job_name: 'node'
       static_configs:
         - targets: ['node-exporter:9100']
   EOF

   # Add node-exporter to docker-compose.yml (optional)
   # Add postgres-exporter to docker-compose.yml (optional)
   ```

2. **Configure Grafana** (Already in docker-compose.yml)
   ```bash
   # Access Grafana
   # URL: https://grafana.mockforge.dev (after DNS setup)
   # Default login: admin / (password from .env file)

   # Add Prometheus as data source:
   # 1. Go to Configuration > Data Sources
   # 2. Add Prometheus
   # 3. URL: http://prometheus:9090
   # 4. Save & Test
   ```

3. **Set Up External Uptime Monitoring (Free Tier)**
   ```bash
   # Option 1: UptimeRobot (Free - 50 monitors)
   # - Sign up at https://uptimerobot.com
   # - Add monitor:
   #   - Type: HTTPS
   #   - URL: https://api.mockforge.dev/health
   #   - Interval: 5 minutes
   #   - Alert contacts: Email

   # Option 2: Better Uptime (Free tier)
   # - Self-hosted or use their free tier
   # - Sign up at https://betteruptime.com
   ```

4. **Configure Log Aggregation** (Optional)
   ```bash
   # Add Loki to docker-compose.yml for log aggregation
   # Or use simple log rotation (already configured)

   # View logs:
   docker-compose logs -f registry-server
   docker-compose logs -f postgres
   docker-compose logs -f nginx
   ```

**Verification:**
- [ ] Prometheus scraping metrics from `/metrics` endpoint
- [ ] Grafana dashboards created and accessible
- [ ] External uptime monitoring configured (UptimeRobot/Better Uptime)
- [ ] Log rotation configured
- [ ] Logs accessible via `docker-compose logs`

---

#### Alerting (Email-Based)

**Cost:** $0 (using email notifications)

**Steps:**

1. **Configure Prometheus Alertmanager** (Optional - for advanced alerting)
   ```bash
   # Add Alertmanager to docker-compose.yml
   # Or use simple email alerts via Grafana

   # Configure Grafana email notifications:
   # 1. Go to Administration > Settings > Notifications
   # 2. Configure SMTP settings (use Resend SMTP)
   # 3. Test email delivery
   ```

2. **Set Up Email Alerts via Grafana**
   ```bash
   # In Grafana:
   # 1. Go to Alerting > Notification channels
   # 2. Add email channel
   # 3. Configure SMTP (Resend SMTP settings)
   # 4. Create alert rules
   # 5. Assign notification channel
   ```

3. **Use UptimeRobot for Uptime Alerts** (Free tier)
   ```bash
   # Already configured in monitoring setup
   # UptimeRobot sends email alerts automatically
   # Configure alert contacts in UptimeRobot dashboard
   ```

4. **Create Alert Runbooks**
   - Document each alert in `cloud-service/operations/runbook.md`
   - Include investigation steps
   - Document resolution procedures
   - Add escalation paths

**Verification:**
- [ ] Grafana email notifications configured
- [ ] UptimeRobot alerts configured
- [ ] Test alerts sent and verified
- [ ] Alert runbooks created

---

#### Backup & Recovery (Self-Hosted)

**Cost:** $0 (local backups) or ~$0.50/month (external backup storage)

**Steps:**

1. **Configure Automated Database Backups**
   ```bash
   # Create backup script
   cat > /opt/mockforge/backups/backup-db.sh << 'EOF'
   #!/bin/bash
   BACKUP_DIR="/opt/mockforge/backups"
   DATE=$(date +%Y%m%d_%H%M%S)
   BACKUP_FILE="$BACKUP_DIR/db_backup_$DATE.sql.gz"

   # Backup PostgreSQL
   docker-compose exec -T postgres pg_dump -U mockforge mockforge | gzip > "$BACKUP_FILE"

   # Keep only last 30 days of backups
   find "$BACKUP_DIR" -name "db_backup_*.sql.gz" -mtime +30 -delete

   echo "Backup completed: $BACKUP_FILE"
   EOF

   chmod +x /opt/mockforge/backups/backup-db.sh

   # Add to crontab (daily at 2 AM)
   (crontab -l 2>/dev/null; echo "0 2 * * * /opt/mockforge/backups/backup-db.sh >> /opt/mockforge/logs/backup.log 2>&1") | crontab -
   ```

2. **Configure Object Storage Backups (Cloudflare R2)**
   ```bash
   # Cloudflare R2 has built-in durability and redundancy
   # However, you can still create backups if needed

   # Option 1: Use Cloudflare R2 versioning (recommended)
   # - Enable versioning in R2 bucket settings
   # - Automatic versioning of all objects
   # - No additional backup script needed
   # - Go to R2 dashboard > Bucket > Settings > Enable versioning

   # Option 2: Manual backup script (if needed)
   # Create backup script for R2 bucket using rclone
   cat > /opt/mockforge/backups/backup-r2.sh << 'EOF'
   #!/bin/bash
   # This script uses rclone to backup R2 bucket
   # Install rclone: apt install rclone
   # Configure: rclone config (use S3 provider with R2 endpoint)

   BACKUP_DIR="/opt/mockforge/backups/r2"
   DATE=$(date +%Y%m%d_%H%M%S)
   BACKUP_FILE="$BACKUP_DIR/r2_backup_$DATE.tar.gz"

   mkdir -p "$BACKUP_DIR"

   # Sync R2 bucket to local backup (if rclone is configured)
   # rclone sync r2:mockforge-marketplace "$BACKUP_DIR/current"
   # tar czf "$BACKUP_FILE" -C "$BACKUP_DIR/current" .

   # Keep only last 7 days of backups
   find "$BACKUP_DIR" -name "r2_backup_*.tar.gz" -mtime +7 -delete

   echo "R2 backup completed: $BACKUP_FILE"
   EOF

   chmod +x /opt/mockforge/backups/backup-r2.sh

   # Note: R2 has built-in redundancy, so backups are optional
   # Enable versioning in R2 dashboard instead (recommended)
   ```

3. **Set Up External Backup Storage (Optional - Recommended)**
   ```bash
   # Option 1: Backblaze B2 (Free 10GB, then $5/TB/month)
   # - Sign up at https://www.backblaze.com/b2/sign-up.html
   # - Create bucket: mockforge-backups
   # - Install B2 CLI
   pip install b2
   b2 authorize-account YOUR_ACCOUNT_ID YOUR_APPLICATION_KEY

   # Update backup script to upload to B2:
   b2 upload-file mockforge-backups "$BACKUP_FILE" "db_backup_$DATE.sql.gz"

   # Option 2: rsync to another server (if you have one)
   rsync -avz /opt/mockforge/backups/ user@backup-server:/backups/mockforge/
   ```

4. **Test Backup Restoration**
   ```bash
   # Test database restore
   # Stop services
   docker-compose stop registry-server

   # Restore database
   gunzip < /opt/mockforge/backups/db_backup_YYYYMMDD_HHMMSS.sql.gz | \
     docker-compose exec -T postgres psql -U mockforge mockforge

   # Verify data integrity
   docker-compose exec postgres psql -U mockforge -d mockforge -c "SELECT COUNT(*) FROM users;"

   # Restart services
   docker-compose start registry-server
   ```

5. **Document Disaster Recovery Plan**
   - Create `cloud-service/operations/disaster-recovery.md`
   - Document RTO (Recovery Time Objective): 4 hours
   - Document RPO (Recovery Point Objective): 24 hours (daily backups)
   - Include step-by-step recovery procedures

**Verification:**
- [ ] Automated database backups configured and running
- [ ] R2 versioning enabled (or backup script configured)
- [ ] External backup storage configured (optional but recommended)
- [ ] Backup restoration tested successfully
- [ ] Disaster recovery plan documented
- [ ] Backup retention policy defined (30 days for DB, R2 versioning for objects)

---

#### Email Service Setup (Free Tier)

**Cost:** $0 (Resend or SendGrid free tier)

**Steps:**

1. **Choose Email Provider** (Recommended: Resend)
   ```bash
   # Option 1: Resend (Recommended)
   # - Sign up at https://resend.com
   # - Free tier: 3,000 emails/month, 100 emails/day
   # - Best for transactional emails
   # - Good deliverability

   # Option 2: SendGrid
   # - Sign up at https://sendgrid.com
   # - Free tier: 100 emails/day forever
   # - Good for getting started
   ```

2. **Get API Key**
   ```bash
   # Resend:
   # 1. Go to https://resend.com/api-keys
   # 2. Create new API key
   # 3. Copy the key (starts with "re_")

   # SendGrid:
   # 1. Go to Settings > API Keys
   # 2. Create API key
   # 3. Copy the key
   ```

3. **Configure in Application**
   ```bash
   # Add to .env file
   echo "RESEND_API_KEY=re_your_api_key_here" >> /opt/mockforge/.env

   # Or for SendGrid:
   echo "SENDGRID_API_KEY=SG.your_api_key_here" >> /opt/mockforge/.env
   echo "EMAIL_PROVIDER=sendgrid" >> /opt/mockforge/.env

   # Restart registry server to apply changes
   docker-compose restart registry-server
   ```

4. **Verify Email Configuration**
   ```bash
   # Test email sending via API
   curl -X POST https://api.mockforge.dev/api/v1/auth/password/reset-request \
     -H "Content-Type: application/json" \
     -d '{"email":"your-email@example.com"}'

   # Check email was received
   # Check application logs for email sending status
   docker-compose logs registry-server | grep -i email
   ```

5. **Configure Email Templates** (Already in codebase)
   - Email templates are already implemented in `crates/mockforge-registry-server/src/email.rs`
   - Templates include: welcome, password reset, deployment status, etc.
   - No additional configuration needed

**Verification:**
- [ ] Email provider account created
- [ ] API key obtained and added to .env
- [ ] Email sending tested and verified
- [ ] Email templates working correctly
- [ ] Email logs monitored

---

#### Object Storage Setup (Cloudflare R2 - Free Tier)

**Cost:** $0 (free 10GB, then $0.015/GB/month)

**Steps:**

1. **Create Cloudflare R2 Bucket**
   ```bash
   # 1. Go to Cloudflare Dashboard > R2
   # 2. Create bucket: "mockforge-marketplace"
   # 3. Get R2 API token:
   #    - Go to "Manage R2 API Tokens"
   #    - Create token with "Object Read & Write" permissions
   #    - Save Access Key ID and Secret Access Key
   # 4. Get your Account ID from Cloudflare dashboard (top right)
   ```

2. **Configure in Application**
   ```bash
   # Update .env file with R2 credentials
   # Replace placeholder values:
   nano /opt/mockforge/.env

   # Set these values:
   # S3_ENDPOINT=https://<account-id>.r2.cloudflarestorage.com
   # S3_ACCESS_KEY=<your-r2-access-key>
   # S3_SECRET_KEY=<your-r2-secret-key>
   # S3_BUCKET=mockforge-marketplace
   # S3_REGION=auto

   # Example:
   # S3_ENDPOINT=https://abc123def456.r2.cloudflarestorage.com
   # S3_ACCESS_KEY=your-access-key-id
   # S3_SECRET_KEY=your-secret-access-key
   # S3_BUCKET=mockforge-marketplace
   # S3_REGION=auto
   ```

3. **Verify R2 Access**
   ```bash
   # Restart registry server to apply changes
   docker-compose restart registry-server

   # Test upload via API (after authentication)
   curl -X POST https://api.mockforge.dev/api/v1/plugins \
     -H "Authorization: Bearer $TOKEN" \
     -F "file=@test-plugin.wasm"

   # Check Cloudflare R2 dashboard for uploaded file
   # Verify file appears in bucket
   ```

**Verification:**
- [ ] Cloudflare R2 bucket created
- [ ] R2 API token created with proper permissions
- [ ] S3 credentials added to .env file
- [ ] Test upload successful
- [ ] Files visible in R2 dashboard

---

#### AI Model Configuration (Hybrid Approach)

**Cost:** $0-20/month (depending on usage)

**Steps:**

1. **Choose AI Provider Strategy**
   ```bash
   # Option 1: Cloud AI (Recommended for Production)
   # - OpenAI: $5 free credit, then $0.002/1K tokens (GPT-3.5)
   # - Anthropic: Free tier available, then $0.008/1K tokens (Claude 3 Haiku)
   # - Best for: Production, fast responses, larger models
   # - Estimated cost: $0-20/month for low-medium usage

   # Option 2: Self-Host Ollama (For Development/Privacy)
   # - Free, but requires 2-8GB RAM per model
   # - Best for: Development, privacy-sensitive, no API costs
   # - Note: 4GB VPS can only run small models (llama3.2:1b)
   # - CPU inference is slow (10-30s per request)

   # Option 3: Hybrid (Recommended)
   # - Use cloud AI for production
   # - Use Ollama for development/testing (optional)
   ```

2. **Configure Cloud AI Provider (Recommended)**
   ```bash
   # Option A: OpenAI
   # 1. Sign up at https://platform.openai.com
   # 2. Get API key from https://platform.openai.com/api-keys
   # 3. Add to .env:
   nano /opt/mockforge/.env

   # Add these lines:
   # MOCKFORGE_RAG_ENABLED=true
   # MOCKFORGE_RAG_PROVIDER=openai
   # MOCKFORGE_RAG_API_KEY=sk-your-key-here
   # MOCKFORGE_RAG_MODEL=gpt-3.5-turbo
   # MOCKFORGE_RAG_API_ENDPOINT=https://api.openai.com/v1

   # Option B: Anthropic
   # 1. Sign up at https://console.anthropic.com
   # 2. Get API key
   # 3. Add to .env:
   # MOCKFORGE_RAG_ENABLED=true
   # MOCKFORGE_RAG_PROVIDER=anthropic
   # MOCKFORGE_RAG_API_KEY=sk-ant-your-key-here
   # MOCKFORGE_RAG_MODEL=claude-3-haiku-20240307
   # MOCKFORGE_RAG_API_ENDPOINT=https://api.anthropic.com/v1

   # Restart registry server
   docker-compose restart registry-server
   ```

3. **Self-Host Ollama (Optional - Development Only)**
   ```bash
   # Only recommended if you have spare resources
   # Small models (llama3.2:1b) require ~1-2GB RAM
   # Note: This will compete with other services for resources

   # Add Ollama to docker-compose.yml:
   cat >> /opt/mockforge/docker-compose.yml << 'EOF'

     # Ollama (Optional - for local AI models)
     ollama:
       image: ollama/ollama:latest
       container_name: mockforge-ollama
       volumes:
         - ollama_data:/root/.ollama
       ports:
         - "11434:11434"
       networks:
         - mockforge-network
       restart: unless-stopped
       deploy:
         resources:
           limits:
             memory: 2G  # Reserve 2GB for Ollama
   EOF

   # Add volume to volumes section:
   # volumes:
   #   ollama_data:

   # Start Ollama
   docker-compose up -d ollama

   # Wait for Ollama to start
   sleep 10

   # Pull a small model (requires ~1-2GB RAM)
   docker-compose exec ollama ollama pull llama3.2:1b

   # Configure MockForge to use Ollama
   nano /opt/mockforge/.env
   # Add:
   # MOCKFORGE_RAG_ENABLED=true
   # MOCKFORGE_RAG_PROVIDER=ollama
   # MOCKFORGE_RAG_API_ENDPOINT=http://ollama:11434/api
   # MOCKFORGE_RAG_MODEL=llama3.2:1b

   # Restart registry server
   docker-compose restart registry-server

   # Note: Ollama on CPU is slow (10-30s per request)
   # Only use for development/testing, not production
   ```

4. **Verify AI Configuration**
   ```bash
   # Test AI generation (if enabled)
   # Check logs for AI provider connection
   docker-compose logs registry-server | grep -i "rag\|ai\|llm"

   # Test via API (if AI endpoints are available)
   # curl -X POST https://api.mockforge.dev/api/v1/ai/generate \
   #   -H "Authorization: Bearer $TOKEN" \
   #   -H "Content-Type: application/json" \
   #   -d '{"prompt": "Generate test data"}'
   ```

5. **Monitor AI Usage**
   ```bash
   # Check AI token usage in application logs
   docker-compose logs registry-server | grep -i "tokens\|usage"

   # Monitor costs (if using cloud AI)
   # - OpenAI: Check usage at https://platform.openai.com/usage
   # - Anthropic: Check usage at https://console.anthropic.com/settings/usage
   ```

**Verification:**
- [ ] AI provider configured (cloud or Ollama)
- [ ] API key added to .env (if using cloud)
- [ ] Ollama installed and model pulled (if self-hosting)
- [ ] AI generation tested and working
- [ ] Usage monitoring configured
- [ ] Cost tracking set up (if using cloud AI)

**Performance Notes:**
- **Cloud AI**: Fast (1-3s per request), reliable, pay-per-use
- **Ollama (CPU)**: Slow (10-30s per request), free, privacy-focused
- **Recommendation**: Use cloud AI for production, Ollama only for development/testing

---

### Testing

#### Load Testing

**Steps:**

1. **Execute Load Tests**
   ```bash
   # Run marketplace load tests
   make load-test-marketplace

   # Run all load tests
   make load-test-all

   # Or manually
   cd tests/load
   ./run_marketplace_load.sh
   ```

2. **Verify Performance Benchmarks**
   - P95 latency < 200ms
   - P99 latency < 500ms
   - Error rate < 0.1%
   - Throughput > 1000 req/s

3. **Monitor Resource Usage During Load**
   ```bash
   # Trigger load
   k6 run --vus 50 --duration 5m tests/load/marketplace_load.js

   # Monitor system resources
   docker stats
   htop  # or top
   df -h  # Check disk usage

   # Monitor via Grafana (if configured)
   # Check CPU, memory, disk I/O metrics
   ```

4. **Complete Capacity Planning**
   - Document expected traffic patterns
   - Calculate resource requirements based on VPS specs
   - Plan for 2-3x peak load (may need to upgrade VPS)
   - Document when to scale up VPS size

**Verification:**
- [ ] Load tests executed
- [ ] Performance benchmarks met (adjusted for VPS resources)
- [ ] Resource usage monitored during load
- [ ] Capacity planning completed
- [ ] Load test results documented
- [ ] VPS upgrade path identified if needed

---

#### Integration Testing

**Steps:**

1. **Test All APIs**
   ```bash
   # Run E2E tests
   cd crates/mockforge-registry-server
   cargo test --test marketplace_e2e -- --ignored

   # Test authentication flows
   curl -X POST https://api.mockforge.dev/api/v1/auth/register \
     -H "Content-Type: application/json" \
     -d '{"username":"test","email":"test@example.com","password":"test1234"}'
   ```

2. **Test Payment Processing**
   ```bash
   # Use Stripe test mode
   export STRIPE_SECRET_KEY=sk_test_...

   # Test subscription creation
   curl -X POST https://api.mockforge.dev/api/v1/billing/subscriptions \
     -H "Authorization: Bearer $TOKEN" \
     -d '{"plan":"pro"}'
   ```

3. **Test Email Notifications**
   ```bash
   # Test password reset email
   curl -X POST https://api.mockforge.dev/api/v1/auth/password/reset-request \
     -H "Content-Type: application/json" \
     -d '{"email":"test@example.com"}'

   # Check email service logs
   docker-compose logs registry-server | grep -i email
   ```

4. **Test OAuth Flows**
   - Test GitHub OAuth: `https://api.mockforge.dev/api/v1/auth/github`
   - Test Google OAuth: `https://api.mockforge.dev/api/v1/auth/google`
   - Verify callback handling
   - Test token refresh

**Verification:**
- [ ] All APIs tested and working
- [ ] Payment processing tested (test mode)
- [ ] Email notifications sending correctly
- [ ] OAuth flows working
- [ ] Integration test results documented

---

#### User Acceptance Testing

**Steps:**

1. **Recruit Beta Testers**
   - Reach out to existing MockForge users
   - Post in community forums
   - Offer early access incentives

2. **Complete Beta Testing**
   - Provide beta testers with access
   - Collect feedback via surveys
   - Monitor usage patterns
   - Track bug reports

3. **Collect and Address Feedback**
   - Prioritize feedback by impact
   - Create tickets for improvements
   - Implement critical fixes
   - Communicate updates to testers

4. **Get UAT Sign-Off**
   - Review all feedback
   - Verify critical issues resolved
   - Get approval from stakeholders
   - Document UAT results

**Verification:**
- [ ] Beta testers recruited (minimum 10)
- [ ] Beta testing completed (minimum 2 weeks)
- [ ] Feedback collected and analyzed
- [ ] Critical issues addressed
- [ ] UAT sign-off received

---

## Launch Week (1 week before)

### Documentation

#### User Documentation

**Steps:**

1. **Publish Getting Started Guide**
   - Review `docs/cloud/GETTING_STARTED.md`
   - Publish to `https://docs.mockforge.dev/getting-started`
   - Add screenshots and examples
   - Test all links

2. **Complete API Documentation**
   - Review `docs/cloud/API_REFERENCE.md`
   - Publish to `https://docs.mockforge.dev/api`
   - Add interactive API explorer (Swagger/OpenAPI)
   - Include code examples

3. **Publish FAQ**
   - Create `docs/cloud/FAQ.md`
   - Address common questions
   - Include troubleshooting tips
   - Publish to `https://docs.mockforge.dev/faq`

4. **Create Video Tutorials**
   - Record getting started walkthrough
   - Record marketplace publishing tutorial
   - Record hosted mocks deployment
   - Upload to YouTube/Vimeo
   - Embed in documentation

**Verification:**
- [ ] Getting started guide published
- [ ] API documentation complete and accessible
- [ ] FAQ published
- [ ] Video tutorials created and linked
- [ ] All documentation reviewed and tested

---

#### Internal Documentation

**Steps:**

1. **Complete Operations Runbook**
   - Review `cloud-service/operations/runbook.md`
   - Add common operational tasks
   - Include troubleshooting procedures
   - Document escalation paths

2. **Document Incident Response Procedures**
   - Review `cloud-service/operations/incident-response.md`
   - Test incident response workflow
   - Update contact information
   - Schedule incident response training

3. **Create Support Playbooks**
   - Document common support scenarios
   - Create response templates
   - Define escalation criteria
   - Train support team

4. **Update Architecture Diagrams**
   - Create system architecture diagram
   - Document data flow
   - Update network topology
   - Include deployment architecture

**Verification:**
- [ ] Operations runbook complete
- [ ] Incident response procedures documented
- [ ] Support playbooks created
- [ ] Architecture diagrams updated
- [ ] Team trained on procedures

---

### Marketing & Communication

#### Marketing Materials

**Steps:**

1. **Launch Landing Page**
   - Deploy landing page to `https://mockforge.dev`
   - Include feature highlights
   - Add pricing information
   - Include call-to-action buttons
   - Test on mobile devices

2. **Prepare Product Screenshots**
   - Capture marketplace UI screenshots
   - Capture hosted mocks dashboard
   - Capture plugin/template/scenario views
   - Optimize images for web
   - Add to landing page

3. **Create Demo Video**
   - Record 2-3 minute demo
   - Show key features
   - Include voiceover
   - Add captions
   - Upload to landing page

4. **Prepare Press Release**
   - Write press release
   - Include key features
   - Add quotes from team
   - Include contact information
   - Schedule for launch day

**Verification:**
- [ ] Landing page live and tested
- [ ] Product screenshots ready
- [ ] Demo video created and embedded
- [ ] Press release prepared
- [ ] All marketing materials reviewed

---

#### Communication Channels

**Steps:**

1. **Configure Status Page**
   - Set up status page (e.g., statuspage.io, Better Uptime)
   - Configure service components
   - Set up incident templates
   - Test status updates
   - Link from main site

2. **Set Up Support Email**
   - Create support@mockforge.dev
   - Configure email routing
   - Set up ticketing system (Zendesk, Intercom)
   - Create email templates
   - Test email delivery

3. **Prepare Community Forum**
   - Set up forum (Discourse, GitHub Discussions)
   - Create categories
   - Add welcome posts
   - Configure moderation
   - Test user registration

4. **Create Social Media Accounts**
   - Create Twitter/X account
   - Create LinkedIn company page
   - Create GitHub organization
   - Create Discord/Slack community
   - Schedule launch announcements

**Verification:**
- [ ] Status page configured and accessible
- [ ] Support email set up and tested
- [ ] Community forum ready
- [ ] Social media accounts created
- [ ] All channels tested

---

### Team Preparation

#### Support Team

**Steps:**

1. **Train Support Team**
   - Provide product training
   - Review common issues
   - Practice troubleshooting
   - Test support tools
   - Document support procedures

2. **Configure Support Tools**
   - Set up ticketing system
   - Configure knowledge base
   - Set up chat widget
   - Configure email routing
   - Test all integrations

3. **Define Escalation Procedures**
   - Document escalation criteria
   - Define response times
   - Create escalation paths
   - Set up on-call rotation
   - Test escalation workflow

4. **Publish Support Hours**
   - Define support hours (e.g., 9 AM - 5 PM EST, Mon-Fri)
   - Update website
   - Configure auto-responses
   - Set expectations
   - Communicate to team

**Verification:**
- [ ] Support team trained
- [ ] Support tools configured
- [ ] Escalation procedures defined
- [ ] Support hours published
- [ ] Support team ready

---

#### Engineering Team

**Steps:**

1. **Create On-Call Schedule**
   - Set up on-call rotation
   - Configure PagerDuty/Opsgenie
   - Define on-call responsibilities
   - Schedule training
   - Test alert routing

2. **Identify Incident Response Team**
   - Assign incident commander
   - Assign technical lead
   - Assign communications lead
   - Document roles
   - Schedule training

3. **Document Deployment Procedures**
   - Review deployment process
   - Document rollback procedures
   - Test deployment pipeline
   - Create deployment checklist
   - Train team

4. **Test Rollback Procedures**
   - Simulate deployment failure
   - Execute rollback
   - Measure rollback time
   - Document lessons learned
   - Update procedures

**Verification:**
- [ ] On-call schedule created
- [ ] Incident response team identified
- [ ] Deployment procedures documented
- [ ] Rollback procedures tested
- [ ] Engineering team ready

---

## Launch Day

### Pre-Launch (Morning)

#### Final Checks

**Steps:**

1. **Verify All Systems Operational**
   ```bash
   # Health checks
   curl https://api.mockforge.dev/health
   curl https://app.mockforge.dev/health

   # Database connectivity
   kubectl exec -it deployment/mockforge-registry-server -n mockforge -- \
     psql $DATABASE_URL -c "SELECT 1;"

   # Storage connectivity
   aws s3 ls s3://mockforge-marketplace/
   ```

2. **Review Monitoring Dashboards**
   - Check Grafana dashboards
   - Verify all metrics collecting
   - Review alert status
   - Check log aggregation
   - Verify APM traces

3. **Complete Backup Verification**
   ```bash
   # Verify latest backup
   ls -lh /backups/

   # Test backup restoration (on staging)
   # Verify backup retention policy
   ```

4. **Brief Team**
   - Conduct launch day standup
   - Review launch checklist
   - Assign responsibilities
   - Set up communication channels
   - Confirm everyone ready

**Verification:**
- [ ] All systems operational
- [ ] Monitoring dashboards reviewed
- [ ] Backup verification completed
- [ ] Team briefed and ready

---

#### Communication

**Steps:**

1. **Update Status Page**
   - Post "Launch in progress" status
   - Set maintenance mode if needed
   - Configure status updates

2. **Notify Team**
   - Send launch day reminder
   - Confirm on-call availability
   - Set up Slack/Discord channel
   - Share launch checklist

3. **Inform Stakeholders**
   - Send launch day notification
   - Share launch timeline
   - Provide contact information
   - Set expectations

4. **Schedule Launch Announcement**
   - Schedule social media posts
   - Schedule email campaign
   - Schedule press release
   - Prepare announcement content

**Verification:**
- [ ] Status page updated
- [ ] Team notified
- [ ] Stakeholders informed
- [ ] Launch announcement scheduled

---

### Launch (Afternoon)

#### Service Activation

**Steps:**

1. **Complete DNS Cutover**
   ```bash
   # Update DNS records to point to production
   # Verify DNS propagation
   dig app.mockforge.dev
   dig api.mockforge.dev

   # Test from multiple locations
   ```

2. **Activate SSL Certificates**
   ```bash
   # Verify certificates active
   openssl s_client -connect app.mockforge.dev:443 -servername app.mockforge.dev

   # Test HTTPS access
   curl -I https://app.mockforge.dev
   ```

3. **Configure Load Balancer**
   ```bash
   # Verify load balancer health checks
   aws elbv2 describe-target-health --target-group-arn $TG_ARN

   # Test load balancing
   for i in {1..10}; do curl https://api.mockforge.dev/health; done
   ```

4. **Verify Service Endpoints**
   ```bash
   # Test all endpoints
   curl https://api.mockforge.dev/api/v1/plugins
   curl https://api.mockforge.dev/api/v1/templates
   curl https://api.mockforge.dev/api/v1/scenarios
   curl https://api.mockforge.dev/api/v1/auth/register
   ```

**Verification:**
- [ ] DNS cutover completed
- [ ] SSL certificates active
- [ ] Load balancer configured
- [ ] Service endpoints verified

---

#### Monitoring

**Steps:**

1. **Activate Real-Time Monitoring**
   - Open Grafana dashboards
   - Monitor error rates
   - Watch latency metrics
   - Track request volume
   - Monitor resource usage

2. **Verify Alert Thresholds**
   - Confirm alert rules active
   - Test alert delivery
   - Verify on-call routing
   - Check alert runbooks accessible

3. **Confirm Dashboard Access**
   - Verify team has access
   - Test dashboard loading
   - Confirm all metrics visible
   - Test alert links

4. **Verify Log Aggregation**
   - Check logs streaming
   - Test log search
   - Verify log retention
   - Test log export

**Verification:**
- [ ] Real-time monitoring active
- [ ] Alert thresholds verified
- [ ] Dashboard access confirmed
- [ ] Log aggregation working

---

### Post-Launch (Evening)

#### Verification

**Steps:**

1. **Verify All Endpoints Responding**
   ```bash
   # Test all API endpoints
   ./scripts/test-all-endpoints.sh

   # Verify response times
   # Check error rates
   ```

2. **Test User Registration**
   ```bash
   # Register test user
   curl -X POST https://api.mockforge.dev/api/v1/auth/register \
     -H "Content-Type: application/json" \
     -d '{"username":"testuser","email":"test@example.com","password":"test1234"}'

   # Verify email sent
   # Test login
   ```

3. **Test Payment Processing**
   ```bash
   # Create test subscription
   # Verify Stripe webhook received
   # Check database updated
   # Verify access granted
   ```

4. **Test Email Notifications**
   ```bash
   # Trigger password reset
   # Verify email sent
   # Check email delivery logs
   # Test email templates
   ```

**Verification:**
- [ ] All endpoints responding
- [ ] User registration working
- [ ] Payment processing functional
- [ ] Email notifications sending

---

#### Communication

**Steps:**

1. **Publish Launch Announcement**
   - Post on website
   - Send email to beta users
   - Post on social media
   - Submit to Hacker News/Product Hunt

2. **Publish Social Media Posts**
   - Twitter/X announcement
   - LinkedIn post
   - GitHub release notes
   - Community forum post

3. **Send Email to Beta Users**
   - Thank beta testers
   - Announce general availability
   - Share launch highlights
   - Include upgrade incentives

4. **Distribute Press Release**
   - Send to tech press
   - Post on company blog
   - Share with industry contacts
   - Monitor coverage

**Verification:**
- [ ] Launch announcement published
- [ ] Social media posts live
- [ ] Email to beta users sent
- [ ] Press release distributed

---

## Post-Launch (First Week)

### Daily Checks

#### System Health

**Steps (Daily):**

1. **Check Uptime**
   ```bash
   # Verify uptime > 99.9%
   # Review downtime incidents
   # Check SLA compliance
   ```

2. **Monitor Error Rate**
   ```bash
   # Verify error rate < 0.1%
   # Review error logs
   # Investigate spikes
   ```

3. **Check Latency**
   ```bash
   # Verify P95 latency < 200ms
   # Review slow queries
   # Optimize bottlenecks
   ```

4. **Monitor Resource Usage**
   ```bash
   # Check CPU/memory usage
   # Review database connections
   # Monitor storage usage
   # Plan capacity
   ```

**Verification (Daily):**
- [ ] Uptime > 99.9%
- [ ] Error rate < 0.1%
- [ ] Latency within targets
- [ ] Resource usage normal

---

#### User Activity

**Steps (Daily):**

1. **Track New Signups**
   - Monitor registration rate
   - Analyze signup sources
   - Track conversion funnel
   - Identify drop-off points

2. **Monitor Active Users**
   - Track DAU/MAU
   - Monitor feature usage
   - Identify power users
   - Track engagement metrics

3. **Review Support Tickets**
   - Categorize tickets
   - Track resolution time
   - Identify common issues
   - Update documentation

4. **Collect User Feedback**
   - Monitor reviews
   - Track feature requests
   - Analyze user surveys
   - Prioritize improvements

**Verification (Daily):**
- [ ] New signups tracked
- [ ] Active users monitored
- [ ] Support tickets reviewed
- [ ] User feedback collected

---

### Weekly Review

#### Performance Review

**Steps (Weekly):**

1. **Analyze Metrics**
   - Review performance trends
   - Identify bottlenecks
   - Compare to benchmarks
   - Document findings

2. **Identify Bottlenecks**
   - Review slow queries
   - Analyze API response times
   - Check database performance
   - Review caching effectiveness

3. **Note Optimization Opportunities**
   - Document optimization ideas
   - Prioritize by impact
   - Create improvement tickets
   - Plan optimization sprints

4. **Update Capacity Planning**
   - Review traffic patterns
   - Update growth projections
   - Adjust resource allocation
   - Plan for scaling

**Verification (Weekly):**
- [ ] Metrics analyzed
- [ ] Bottlenecks identified
- [ ] Optimization opportunities noted
- [ ] Capacity planning updated

---

#### Business Metrics

**Steps (Weekly):**

1. **Calculate MRR (Monthly Recurring Revenue)**
   - Sum all active subscriptions
   - Track growth rate
   - Compare to targets
   - Forecast future MRR

2. **Track Churn Rate**
   - Calculate churn percentage
   - Identify churn reasons
   - Implement retention strategies
   - Monitor improvements

3. **Analyze User Growth**
   - Track signup rate
   - Monitor activation rate
   - Analyze retention curves
   - Compare to projections

4. **Review Conversion Rates**
   - Free to Pro conversion
   - Pro to Team conversion
   - Trial to paid conversion
   - Identify optimization opportunities

**Verification (Weekly):**
- [ ] MRR calculated
- [ ] Churn rate tracked
- [ ] User growth analyzed
- [ ] Conversion rates reviewed

---

## Success Criteria

### Technical

- ✅ **99.9% uptime achieved**
- ✅ **P95 latency < 200ms**
- ✅ **Error rate < 0.1%**
- ✅ **Zero data loss**
- ✅ **All backups successful**

### Business

- ✅ **First paying customers**
- ✅ **Positive user feedback**
- ✅ **Support ticket resolution < 24h**
- ✅ **No critical incidents**
- ✅ **Marketing goals met**

---

## Rollback Plan

If critical issues arise:

### Immediate Actions

1. **Identify Issue Severity**
   - Assess impact (users affected, revenue impact)
   - Determine if rollback needed
   - Notify team immediately

2. **Notify Team**
   - Alert on-call engineer
   - Notify incident commander
   - Update status page
   - Set up war room

3. **Update Status Page**
   - Post incident status
   - Set estimated resolution time
   - Provide updates every 15 minutes

4. **Begin Incident Response**
   - Follow incident response procedures
   - Document timeline
   - Assign investigation tasks
   - Communicate updates

### Rollback Decision

1. **Assess Impact**
   - How many users affected?
   - Is data at risk?
   - Can issue be fixed without rollback?
   - What's the cost of rollback?

2. **Determine Rollback Necessity**
   - Critical data loss? → Rollback immediately
   - Service completely down? → Rollback immediately
   - Minor feature issue? → Fix forward
   - Performance degradation? → Assess impact

3. **Execute Rollback if Needed**
   ```bash
   # Rollback Docker image to previous version
   cd /opt/mockforge

   # Pull previous image version
   docker-compose pull registry-server

   # Or use specific image tag
   # Edit docker-compose.yml to use previous image tag
   # Then restart:
   docker-compose up -d registry-server

   # Database rollback (if needed)
   # Restore from backup (see Backup & Recovery section)
   ```

4. **Document Incident**
   - Record timeline
   - Document root cause
   - Note resolution steps
   - Create post-mortem

### Post-Incident

1. **Root Cause Analysis**
   - Investigate root cause
   - Document findings
   - Identify contributing factors
   - Create action items

2. **Remediation Plan**
   - Fix root cause
   - Implement preventive measures
   - Update procedures
   - Test fixes

3. **Process Improvements**
   - Review incident response
   - Update runbooks
   - Improve monitoring
   - Enhance testing

4. **Communication to Users**
   - Post incident summary
   - Apologize for impact
   - Explain what happened
   - Describe improvements

---

## Resources

- [Operations Runbook](./cloud-service/operations/runbook.md)
- [Incident Response](./cloud-service/operations/incident-response.md)
- [Architecture Documentation](./docs/ARCHITECTURE.md)
- [API Reference](./docs/cloud/API_REFERENCE.md)
- [Getting Started Guide](./docs/cloud/GETTING_STARTED.md)
- [Deployment Guide](./deploy/DEPLOYMENT_GUIDE.md)

---

## Quick Reference Commands

### Health Checks
```bash
curl https://api.mockforge.dev/health
curl https://app.mockforge.dev/health
docker-compose ps
```

### Monitoring
```bash
# Grafana (if exposed via nginx)
open https://grafana.mockforge.dev

# Prometheus (if exposed via nginx)
open https://prometheus.mockforge.dev

# Or access locally via SSH tunnel:
ssh -L 3000:localhost:3000 mockforge@YOUR_SERVER_IP
# Then open http://localhost:3000 in browser
```

### Logs
```bash
# All services
docker-compose logs -f

# Specific service
docker-compose logs -f registry-server
docker-compose logs -f postgres
docker-compose logs -f nginx

# Last 100 lines
docker-compose logs --tail=100 registry-server
```

### Database
```bash
# Connect to PostgreSQL
docker-compose exec postgres psql -U mockforge -d mockforge

# Run query
docker-compose exec postgres psql -U mockforge -d mockforge -c "SELECT version();"

# Backup database
docker-compose exec postgres pg_dump -U mockforge mockforge > backup.sql
```

### Service Management
```bash
# Start all services
cd /opt/mockforge
docker-compose up -d

# Stop all services
docker-compose down

# Restart specific service
docker-compose restart registry-server

# View service status
docker-compose ps

# View resource usage
docker stats
```

### Rollback
```bash
cd /opt/mockforge
# Edit docker-compose.yml to use previous image tag
docker-compose pull registry-server
docker-compose up -d registry-server
```

---

**Last Review:** 2025-01-27
**Next Review:** Before launch
**Status:** Ready for Launch ✅
