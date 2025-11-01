# PayTrust Deployment Guide

**Last Updated**: 2025-11-02  
**Version**: 0.1.0

## Prerequisites

- Linux server (Ubuntu 22.04+ or similar)
- MySQL 8.0+ installed and running
- Rust 1.91.0+ (for building from source)
- TLS certificates (Let's Encrypt recommended)
- Root or sudo access for initial setup

## Production Deployment

### 1. System Preparation

```bash
# Update system packages
sudo apt update && sudo apt upgrade -y

# Install MySQL 8.0
sudo apt install mysql-server -y

# Start and enable MySQL
sudo systemctl start mysql
sudo systemctl enable mysql

# Install build tools (if building from source)
sudo apt install build-essential pkg-config libssl-dev -y

# Install Rust (if not already installed)
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source $HOME/.cargo/env
rustup update
```

### 2. MySQL Database Setup

```bash
# Login to MySQL as root
sudo mysql -u root -p

# Create database and user
CREATE DATABASE paytrust CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci;

CREATE USER 'paytrust'@'localhost' IDENTIFIED BY 'STRONG_PASSWORD_HERE';

GRANT ALL PRIVILEGES ON paytrust.* TO 'paytrust'@'localhost';

FLUSH PRIVILEGES;

EXIT;
```

### 3. Application Deployment

```bash
# Create application directory
sudo mkdir -p /opt/paytrust
sudo chown $USER:$USER /opt/paytrust

# Clone repository
cd /opt/paytrust
git clone <repository-url> .
git checkout main  # or specific release tag

# Build release binary
cargo build --release

# Verify binary
./target/release/paytrust --version
```

### 4. Environment Configuration

```bash
# Create production environment file
sudo nano /opt/paytrust/.env
```

**Production `.env` configuration**:

```env
# Application
APP_ENV=production
APP_HOST=0.0.0.0
APP_PORT=8080
LOG_LEVEL=info

# Database
DATABASE_URL=mysql://paytrust:STRONG_PASSWORD_HERE@localhost:3306/paytrust
DATABASE_POOL_SIZE=20
DATABASE_MAX_CONNECTIONS=50

# Payment Gateways
XENDIT_API_KEY=xnd_production_YOUR_KEY_HERE
XENDIT_WEBHOOK_SECRET=your_xendit_webhook_secret
XENDIT_BASE_URL=https://api.xendit.co

MIDTRANS_SERVER_KEY=YOUR_MIDTRANS_PRODUCTION_KEY
MIDTRANS_WEBHOOK_SECRET=your_midtrans_webhook_secret
MIDTRANS_BASE_URL=https://api.midtrans.com

# Security
API_KEY_SECRET=CHANGE_THIS_TO_RANDOM_64_CHAR_STRING
RATE_LIMIT_PER_MINUTE=1000

# Defaults
DEFAULT_INVOICE_EXPIRY_HOURS=24
```

**Generate secure secrets**:

```bash
# Generate API_KEY_SECRET
openssl rand -hex 32

# Generate webhook secrets
openssl rand -hex 32
```

### 5. Run Database Migrations

```bash
cd /opt/paytrust

# Install sqlx-cli if not already installed
cargo install sqlx-cli --no-default-features --features mysql

# Run migrations
sqlx migrate run
```

### 6. Systemd Service Setup

Create systemd service file:

```bash
sudo nano /etc/systemd/system/paytrust.service
```

**Service configuration**:

```ini
[Unit]
Description=PayTrust Payment Orchestration Platform
After=network.target mysql.service
Requires=mysql.service

[Service]
Type=simple
User=www-data
Group=www-data
WorkingDirectory=/opt/paytrust
Environment="RUST_LOG=paytrust=info,actix_web=info"
ExecStart=/opt/paytrust/target/release/paytrust
Restart=on-failure
RestartSec=10
KillMode=mixed
KillSignal=SIGTERM
TimeoutStopSec=30

# Security hardening
NoNewPrivileges=true
PrivateTmp=true
ProtectSystem=strict
ProtectHome=true
ReadWritePaths=/opt/paytrust/logs

# Resource limits
LimitNOFILE=65535
LimitNPROC=4096

[Install]
WantedBy=multi-user.target
```

**Enable and start service**:

```bash
# Reload systemd
sudo systemctl daemon-reload

# Enable service to start on boot
sudo systemctl enable paytrust

# Start service
sudo systemctl start paytrust

# Check status
sudo systemctl status paytrust

# View logs
sudo journalctl -u paytrust -f
```

### 7. Nginx Reverse Proxy (Recommended)

```bash
# Install Nginx
sudo apt install nginx -y

# Create Nginx configuration
sudo nano /etc/nginx/sites-available/paytrust
```

**Nginx configuration**:

```nginx
upstream paytrust_backend {
    server 127.0.0.1:8080;
    keepalive 32;
}

# HTTP -> HTTPS redirect
server {
    listen 80;
    listen [::]:80;
    server_name api.paytrust.example.com;

    return 301 https://$server_name$request_uri;
}

# HTTPS configuration
server {
    listen 443 ssl http2;
    listen [::]:443 ssl http2;
    server_name api.paytrust.example.com;

    # SSL certificates (Let's Encrypt)
    ssl_certificate /etc/letsencrypt/live/api.paytrust.example.com/fullchain.pem;
    ssl_certificate_key /etc/letsencrypt/live/api.paytrust.example.com/privkey.pem;

    # SSL configuration
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    ssl_prefer_server_ciphers on;
    ssl_session_cache shared:SSL:10m;
    ssl_session_timeout 10m;

    # Security headers
    add_header Strict-Transport-Security "max-age=31536000; includeSubDomains" always;
    add_header X-Frame-Options "DENY" always;
    add_header X-Content-Type-Options "nosniff" always;
    add_header X-XSS-Protection "1; mode=block" always;

    # Request size limits
    client_max_body_size 10M;
    client_body_timeout 30s;
    client_header_timeout 30s;

    # Proxy settings
    location / {
        proxy_pass http://paytrust_backend;
        proxy_http_version 1.1;

        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;

        # Timeouts
        proxy_connect_timeout 30s;
        proxy_send_timeout 30s;
        proxy_read_timeout 60s;

        # Keepalive
        proxy_set_header Connection "";
    }

    # Health check endpoint (no rate limiting)
    location /health {
        proxy_pass http://paytrust_backend;
        access_log off;
    }

    # Readiness probe endpoint
    location /ready {
        proxy_pass http://paytrust_backend;
        access_log off;
    }

    # Access and error logs
    access_log /var/log/nginx/paytrust_access.log;
    error_log /var/log/nginx/paytrust_error.log;
}
```

**Enable site and restart Nginx**:

```bash
# Enable site
sudo ln -s /etc/nginx/sites-available/paytrust /etc/nginx/sites-enabled/

# Test configuration
sudo nginx -t

# Restart Nginx
sudo systemctl restart nginx
```

### 8. TLS Certificates (Let's Encrypt)

```bash
# Install Certbot
sudo apt install certbot python3-certbot-nginx -y

# Obtain certificate
sudo certbot --nginx -d api.paytrust.example.com

# Test auto-renewal
sudo certbot renew --dry-run
```

### 9. Firewall Configuration

```bash
# Allow SSH, HTTP, HTTPS
sudo ufw allow 22/tcp
sudo ufw allow 80/tcp
sudo ufw allow 443/tcp

# Block direct access to application port
sudo ufw deny 8080/tcp

# Enable firewall
sudo ufw enable

# Check status
sudo ufw status
```

## Health Checks

### Manual Health Verification

```bash
# Liveness check
curl https://api.paytrust.example.com/health

# Expected response:
# {
#   "status": "healthy",
#   "timestamp": "2025-11-02T10:00:00Z",
#   "checks": {
#     "database": "not_checked",
#     "application": "healthy"
#   }
# }

# Readiness check
curl https://api.paytrust.example.com/ready

# Expected response (if ready):
# {
#   "ready": true,
#   "checks": {
#     "database": true,
#     "application": true
#   }
# }
```

### Kubernetes Health Probes (Optional)

If deploying to Kubernetes, configure probes:

```yaml
livenessProbe:
  httpGet:
    path: /health
    port: 8080
  initialDelaySeconds: 30
  periodSeconds: 10
  timeoutSeconds: 5
  failureThreshold: 3

readinessProbe:
  httpGet:
    path: /ready
    port: 8080
  initialDelaySeconds: 10
  periodSeconds: 5
  timeoutSeconds: 3
  failureThreshold: 3
```

## Monitoring & Logging

### Application Logs

```bash
# Real-time logs
sudo journalctl -u paytrust -f

# Last 100 lines
sudo journalctl -u paytrust -n 100

# Logs since timestamp
sudo journalctl -u paytrust --since "2025-11-02 10:00:00"

# Filter by log level
sudo journalctl -u paytrust | grep ERROR
```

### Log Rotation

Create logrotate configuration:

```bash
sudo nano /etc/logrotate.d/paytrust
```

```
/var/log/nginx/paytrust_*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    create 0640 www-data adm
    sharedscripts
    postrotate
        [ -f /var/run/nginx.pid ] && kill -USR1 `cat /var/run/nginx.pid`
    endscript
}
```

## Database Backup

### Automated Backup Script

```bash
# Create backup script
sudo nano /opt/paytrust/backup.sh
```

```bash
#!/bin/bash
BACKUP_DIR="/opt/paytrust/backups"
DATE=$(date +%Y%m%d_%H%M%S)
MYSQL_USER="paytrust"
MYSQL_PASS="STRONG_PASSWORD_HERE"
DB_NAME="paytrust"

mkdir -p $BACKUP_DIR

mysqldump -u $MYSQL_USER -p$MYSQL_PASS $DB_NAME | gzip > $BACKUP_DIR/paytrust_$DATE.sql.gz

# Keep only last 7 days
find $BACKUP_DIR -name "paytrust_*.sql.gz" -mtime +7 -delete

echo "Backup completed: paytrust_$DATE.sql.gz"
```

```bash
# Make executable
chmod +x /opt/paytrust/backup.sh

# Add to crontab (daily at 2 AM)
crontab -e
```

```cron
0 2 * * * /opt/paytrust/backup.sh >> /var/log/paytrust_backup.log 2>&1
```

## Performance Tuning

### MySQL Optimization

```bash
sudo nano /etc/mysql/mysql.conf.d/mysqld.cnf
```

```ini
[mysqld]
# Connection settings
max_connections = 200
wait_timeout = 600
interactive_timeout = 600

# Buffer pool (set to 70-80% of available RAM)
innodb_buffer_pool_size = 4G
innodb_log_file_size = 512M

# Query cache
query_cache_size = 0  # Disabled in MySQL 8.0+

# Performance Schema
performance_schema = ON
```

### Application Tuning

Adjust in `.env`:

```env
# Increase for high load
DATABASE_POOL_SIZE=30
DATABASE_MAX_CONNECTIONS=100
RATE_LIMIT_PER_MINUTE=2000
```

## Troubleshooting

### Service Won't Start

```bash
# Check logs
sudo journalctl -u paytrust -n 50

# Check environment variables
sudo -u www-data env

# Test binary manually
cd /opt/paytrust
sudo -u www-data ./target/release/paytrust
```

### Database Connection Errors

```bash
# Test MySQL connection
mysql -u paytrust -p paytrust -e "SELECT 1"

# Check MySQL status
sudo systemctl status mysql

# Review MySQL logs
sudo tail -f /var/log/mysql/error.log
```

### High Memory Usage

```bash
# Check memory usage
free -h
ps aux | grep paytrust

# Reduce connection pool size in .env
DATABASE_POOL_SIZE=10
DATABASE_MAX_CONNECTIONS=20
```

### Rate Limiting Issues

```bash
# Check rate limit logs
sudo journalctl -u paytrust | grep "rate limit"

# Adjust rate limit in .env
RATE_LIMIT_PER_MINUTE=500
```

## Scaling

### Horizontal Scaling

For multiple instances:

1. Use external MySQL database (not localhost)
2. Use Redis for distributed rate limiting (requires code changes)
3. Load balance with Nginx:

```nginx
upstream paytrust_backend {
    server 10.0.1.10:8080;
    server 10.0.1.11:8080;
    server 10.0.1.12:8080;
    keepalive 32;
}
```

### Vertical Scaling

- Increase server resources (CPU, RAM)
- Adjust `DATABASE_POOL_SIZE` and `DATABASE_MAX_CONNECTIONS`
- Increase MySQL `max_connections`

## Security Checklist

- [x] MySQL user has minimal privileges
- [x] Strong passwords for database and API keys
- [x] TLS certificates installed and auto-renewing
- [x] Firewall configured (ports 80, 443 only)
- [x] Application runs as non-root user (www-data)
- [x] Security headers configured in Nginx
- [x] Rate limiting enabled (1000 req/min default)
- [x] API keys hashed with argon2
- [ ] Regular security updates (`apt update && apt upgrade`)
- [ ] Database backups automated and tested
- [ ] Monitoring and alerting configured

## Production Checklist

- [ ] MySQL 8.0+ installed and secured
- [ ] Application built in release mode
- [ ] Environment variables configured in `.env`
- [ ] Database migrations applied
- [ ] Systemd service created and enabled
- [ ] Nginx reverse proxy configured
- [ ] TLS certificates installed
- [ ] Firewall rules configured
- [ ] Health checks responding (GET /health, /ready)
- [ ] Logs rotation configured
- [ ] Database backups automated
- [ ] Payment gateway credentials (production keys)
- [ ] Webhook endpoints configured in gateway dashboards
- [ ] DNS records pointing to server
- [ ] Monitoring tools configured (optional)

## Support

For deployment issues:

1. Check application logs: `sudo journalctl -u paytrust -f`
2. Check Nginx logs: `sudo tail -f /var/log/nginx/paytrust_error.log`
3. Verify health endpoints: `curl http://localhost:8080/health`
4. Test database connectivity: `mysql -u paytrust -p paytrust -e "SELECT 1"`

---

**Deployment Complete!** Your PayTrust instance should now be running in production.
