# Deployment Guide

This guide covers deploying the serial_mcp_agent in production environments.

## Pre-Deployment Checklist

- [ ] All CI/CD checks pass (lint, test, security, build)
- [ ] Version bumped in `Cargo.toml`
- [ ] Documentation updated
- [ ] CHANGELOG.md updated (if applicable)
- [ ] Security audit clean (`cargo deny check` and `cargo audit`)
- [ ] Performance tested under load
- [ ] Tested on target platforms

## Deployment Strategies

### 1. Binary Distribution (Recommended for Simplicity)

Download pre-built binaries from GitHub Releases.

#### Linux
```bash
# Download latest release
wget https://github.com/YOUR_USERNAME/rust-comm/releases/latest/download/serial_mcp_agent-linux-x86_64

# Make executable
chmod +x serial_mcp_agent-linux-x86_64

# Move to PATH
sudo mv serial_mcp_agent-linux-x86_64 /usr/local/bin/serial_mcp_agent

# Verify installation
serial_mcp_agent --version
```

#### Windows
```powershell
# Download from GitHub Releases page
# Or use PowerShell
Invoke-WebRequest -Uri "https://github.com/YOUR_USERNAME/rust-comm/releases/latest/download/serial_mcp_agent-windows-x86_64.exe" -OutFile "serial_mcp_agent.exe"

# Add to PATH or run from current directory
.\serial_mcp_agent.exe --version
```

#### macOS
```bash
# Download for your architecture
# Intel
wget https://github.com/YOUR_USERNAME/rust-comm/releases/latest/download/serial_mcp_agent-macos-x86_64

# Apple Silicon
wget https://github.com/YOUR_USERNAME/rust-comm/releases/latest/download/serial_mcp_agent-macos-aarch64

# Make executable and install
chmod +x serial_mcp_agent-macos-*
sudo mv serial_mcp_agent-macos-* /usr/local/bin/serial_mcp_agent

# Verify
serial_mcp_agent --version
```

### 2. Docker Deployment

Create a Dockerfile for containerized deployment.

#### Dockerfile Example
```dockerfile
# Multi-stage build for minimal image size
FROM rust:1.75-slim as builder

WORKDIR /app

# Copy manifests
COPY Cargo.toml Cargo.lock ./

# Copy source code
COPY src ./src

# Build release binary
RUN cargo build --release --all-features

# Runtime stage
FROM debian:bookworm-slim

# Install runtime dependencies
RUN apt-get update && \
    apt-get install -y --no-install-recommends \
    ca-certificates \
    libudev-dev && \
    rm -rf /var/lib/apt/lists/*

# Copy binary from builder
COPY --from=builder /app/target/release/serial_mcp_agent /usr/local/bin/serial_mcp_agent

# Create non-root user
RUN useradd -m -u 1000 mcp && \
    chown -R mcp:mcp /usr/local/bin/serial_mcp_agent

USER mcp

EXPOSE 8080

CMD ["serial_mcp_agent"]
```

#### Docker Compose
```yaml
version: '3.8'

services:
  serial-mcp:
    image: serial_mcp_agent:latest
    build:
      context: .
      dockerfile: Dockerfile
    ports:
      - "8080:8080"
    devices:
      - /dev/ttyUSB0:/dev/ttyUSB0  # Serial port access
    environment:
      - RUST_LOG=info
    volumes:
      - ./config:/app/config
      - serial-data:/app/data
    restart: unless-stopped

volumes:
  serial-data:
```

#### Build and Run
```bash
# Build image
docker build -t serial_mcp_agent:latest .

# Run container
docker run -d \
  --name serial-mcp \
  -p 8080:8080 \
  --device=/dev/ttyUSB0 \
  -e RUST_LOG=info \
  serial_mcp_agent:latest

# View logs
docker logs -f serial-mcp

# Or use docker-compose
docker-compose up -d
```

### 3. Systemd Service (Linux)

Run as a system service for automatic startup.

#### Create Service File
```bash
sudo nano /etc/systemd/system/serial-mcp.service
```

```ini
[Unit]
Description=Serial MCP Agent
After=network.target

[Service]
Type=simple
User=mcp
Group=dialout
WorkingDirectory=/opt/serial-mcp
ExecStart=/usr/local/bin/serial_mcp_agent
Restart=on-failure
RestartSec=5s

# Environment
Environment="RUST_LOG=info"

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/serial-mcp/data

[Install]
WantedBy=multi-user.target
```

#### Enable and Start
```bash
# Create service user
sudo useradd -r -s /bin/false mcp
sudo usermod -a -G dialout mcp

# Create working directory
sudo mkdir -p /opt/serial-mcp/data
sudo chown -R mcp:mcp /opt/serial-mcp

# Reload systemd
sudo systemctl daemon-reload

# Enable service
sudo systemctl enable serial-mcp.service

# Start service
sudo systemctl start serial-mcp.service

# Check status
sudo systemctl status serial-mcp.service

# View logs
sudo journalctl -u serial-mcp.service -f
```

### 4. Windows Service

Use NSSM (Non-Sucking Service Manager) or sc.exe.

#### Using NSSM
```powershell
# Download NSSM from https://nssm.cc/download

# Install service
nssm install SerialMCP "C:\Program Files\SerialMCP\serial_mcp_agent.exe"

# Configure service
nssm set SerialMCP AppDirectory "C:\Program Files\SerialMCP"
nssm set SerialMCP AppEnvironmentExtra RUST_LOG=info
nssm set SerialMCP DisplayName "Serial MCP Agent"
nssm set SerialMCP Description "Serial port MCP server for LLM agents"
nssm set SerialMCP Start SERVICE_AUTO_START

# Start service
nssm start SerialMCP

# Check status
nssm status SerialMCP
```

## Environment Configuration

### Environment Variables
```bash
# Logging level
RUST_LOG=info  # Options: trace, debug, info, warn, error

# Server configuration (if using REST API)
SERVER_HOST=0.0.0.0
SERVER_PORT=8080

# Database path (if applicable)
DATABASE_URL=sqlite:///opt/serial-mcp/data/mcp.db
```

### Configuration File
Create `config.toml` for runtime configuration:

```toml
[server]
host = "127.0.0.1"
port = 8080
timeout = 30

[serial]
default_baud_rate = 9600
default_timeout_ms = 1000

[logging]
level = "info"
file = "/var/log/serial-mcp/app.log"
```

## Monitoring and Logging

### Health Checks
```bash
# HTTP health check (if REST API enabled)
curl http://localhost:8080/health

# Expected response
{"status":"healthy","version":"3.1.0"}
```

### Logging

#### systemd/journald (Linux)
```bash
# View logs
journalctl -u serial-mcp.service -f

# Filter by priority
journalctl -u serial-mcp.service -p err

# View logs since boot
journalctl -u serial-mcp.service -b
```

#### File-based Logging
```bash
# Configure file output
RUST_LOG=info,serial_mcp_agent=debug

# Tail logs
tail -f /var/log/serial-mcp/app.log

# Rotate logs (using logrotate)
sudo nano /etc/logrotate.d/serial-mcp
```

```
/var/log/serial-mcp/*.log {
    daily
    rotate 7
    compress
    delaycompress
    missingok
    notifempty
    create 0640 mcp mcp
}
```

### Metrics (Optional)

Integrate with Prometheus or similar monitoring.

```rust
// Example: Add metrics endpoint
// In your application code
async fn metrics_handler() -> impl IntoResponse {
    // Export metrics
}
```

## Zero-Downtime Deployment

### Blue-Green Deployment

1. **Prepare new version**
   ```bash
   # Download new binary
   wget https://github.com/.../serial_mcp_agent-linux-x86_64 -O serial_mcp_agent.new
   chmod +x serial_mcp_agent.new
   ```

2. **Start new instance on different port**
   ```bash
   SERVER_PORT=8081 ./serial_mcp_agent.new &
   ```

3. **Health check new instance**
   ```bash
   curl http://localhost:8081/health
   ```

4. **Switch traffic** (using load balancer or proxy)
   ```nginx
   # Update nginx config
   upstream serial_mcp {
       server localhost:8081;  # New instance
   }
   ```

5. **Graceful shutdown old instance**
   ```bash
   kill -SIGTERM <old_pid>
   ```

### Rolling Deployment (Multiple Instances)

Use orchestration tools like Kubernetes or Docker Swarm.

## Security Considerations

### Permissions
```bash
# Linux: Add user to dialout group for serial port access
sudo usermod -a -G dialout $USER

# Set proper file permissions
chmod 755 /usr/local/bin/serial_mcp_agent
```

### Firewall Rules
```bash
# Linux (ufw)
sudo ufw allow 8080/tcp

# Linux (iptables)
sudo iptables -A INPUT -p tcp --dport 8080 -j ACCEPT
```

### TLS/HTTPS (Recommended for Production)
Use a reverse proxy like nginx or Caddy:

```nginx
server {
    listen 443 ssl http2;
    server_name serial-mcp.example.com;

    ssl_certificate /etc/letsencrypt/live/serial-mcp.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/serial-mcp.example.com/privkey.pem;

    location / {
        proxy_pass http://localhost:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
    }
}
```

## Backup and Recovery

### Database Backup
```bash
# SQLite backup
cp /opt/serial-mcp/data/mcp.db /backup/mcp-$(date +%Y%m%d).db

# Automated daily backup
cat > /etc/cron.daily/serial-mcp-backup <<'EOF'
#!/bin/bash
cp /opt/serial-mcp/data/mcp.db /backup/mcp-$(date +%Y%m%d).db
find /backup -name "mcp-*.db" -mtime +7 -delete
EOF
chmod +x /etc/cron.daily/serial-mcp-backup
```

### Configuration Backup
```bash
# Backup config
tar -czf config-backup.tar.gz /opt/serial-mcp/config/
```

## Troubleshooting

### Service Won't Start
```bash
# Check logs
journalctl -u serial-mcp.service -n 50

# Check permissions
ls -l /usr/local/bin/serial_mcp_agent

# Check serial port access
ls -l /dev/ttyUSB0
groups  # Verify dialout group membership
```

### High Memory Usage
```bash
# Monitor memory
top -p $(pgrep serial_mcp_agent)

# Check for leaks with valgrind
valgrind --leak-check=full ./serial_mcp_agent
```

### Performance Issues
```bash
# Enable debug logging
RUST_LOG=debug systemctl restart serial-mcp.service

# Profile with perf
perf record -g ./serial_mcp_agent
perf report
```

## Rollback Procedure

1. **Stop current version**
   ```bash
   sudo systemctl stop serial-mcp.service
   ```

2. **Restore previous binary**
   ```bash
   sudo cp /backup/serial_mcp_agent.backup /usr/local/bin/serial_mcp_agent
   ```

3. **Restore database** (if needed)
   ```bash
   cp /backup/mcp-YYYYMMDD.db /opt/serial-mcp/data/mcp.db
   ```

4. **Restart service**
   ```bash
   sudo systemctl start serial-mcp.service
   ```

5. **Verify**
   ```bash
   curl http://localhost:8080/health
   sudo systemctl status serial-mcp.service
   ```

## Production Best Practices

1. **Always test in staging** before production deployment
2. **Use specific version tags**, not `latest`
3. **Monitor logs and metrics** continuously
4. **Set up alerts** for failures and anomalies
5. **Regular backups** (database, config, binaries)
6. **Document your deployment** specific to your environment
7. **Keep dependencies updated** (run `cargo audit` regularly)
8. **Use automation** (Ansible, Terraform) for consistent deployments
9. **Implement health checks** and automatic restarts
10. **Plan rollback strategy** before deploying

## Support

For deployment issues:
- Check logs first
- Review this guide
- Search GitHub Issues
- Create new issue with deployment details
