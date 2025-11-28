# MockForge Ansible Playbooks

Automated deployment playbooks for MockForge on various platforms using Ansible.

## Prerequisites

1. Ansible 2.9+ installed
2. SSH access to target servers
3. Python 3.6+ on target servers

## Installation

```bash
# Install Ansible
pip install ansible

# Or via package manager
# Ubuntu/Debian:
sudo apt install ansible

# macOS:
brew install ansible
```

## Available Playbooks

### 1. Docker Deployment (`docker.yml`)
Deploys MockForge using Docker on a single server.

### 2. Kubernetes Deployment (`kubernetes.yml`)
Deploys MockForge on an existing Kubernetes cluster.

### 3. Systemd Service (`systemd.yml`)
Deploys MockForge as a systemd service on Linux.

## Quick Start

### Docker Deployment

```bash
# Create inventory file
cat > inventory.ini << EOF
[servers]
your-server ansible_host=1.2.3.4 ansible_user=ubuntu
EOF

# Run playbook
ansible-playbook -i inventory.ini docker.yml
```

### Configuration

Create `group_vars/all.yml`:

```yaml
mockforge_version: "latest"
mockforge_http_port: 3000
mockforge_admin_port: 9080
mockforge_config_path: "/etc/mockforge/config.yaml"
```

## Inventory Structure

```ini
[servers]
server1 ansible_host=1.2.3.4 ansible_user=ubuntu
server2 ansible_host=5.6.7.8 ansible_user=ubuntu

[servers:vars]
ansible_ssh_private_key_file=~/.ssh/id_rsa
```

## Variables

See `group_vars/all.yml.example` for all available variables.

## Usage Examples

### Deploy to Single Server
```bash
ansible-playbook -i inventory.ini docker.yml --limit server1
```

### Deploy with Custom Config
```bash
ansible-playbook -i inventory.ini docker.yml \
  -e mockforge_config_path=/custom/path/config.yaml
```

### Update Only
```bash
ansible-playbook -i inventory.ini docker.yml --tags update
```

## Troubleshooting

### Connection Issues
- Verify SSH access: `ansible all -i inventory.ini -m ping`
- Check SSH keys and permissions
- Verify Python is installed on targets

### Permission Issues
- Use `--become` flag for sudo operations
- Check sudo permissions on target servers

## Support

For issues, see:
- [Ansible Documentation](https://docs.ansible.com/)
- [MockForge Deployment Docs](../../docs/deployment/)
