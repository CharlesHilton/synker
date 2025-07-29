# Synker Server Installation Guide

## Prerequisites

### Install Rust

#### Windows
1. Download and run the Rust installer from https://rustup.rs/
2. Follow the installation instructions
3. Restart your command prompt/PowerShell
4. Verify installation: `cargo --version`

#### Linux/macOS
```bash
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh
source ~/.cargo/env
cargo --version
```

## Quick Setup

### 1. Build the Server

#### Windows PowerShell:
```powershell
cd u:\Development\synker
cargo build --release
```

#### Linux/macOS:
```bash
cd /path/to/synker
cargo build --release
```

### 2. Configure the Server

1. Edit `config.toml` with your MyCloud device settings:
```toml
[mycloud]
api_endpoint = "http://192.168.1.100"  # Your MyCloud IP
admin_username = "admin"
admin_password = "your-mycloud-admin-password"
```

2. Update other settings as needed (paths, ports, etc.)

### 3. Initialize Database

#### Windows:
```powershell
.\target\release\synker-server.exe --init-db
```

#### Linux/macOS:
```bash
./target/release/synker-server --init-db
```

### 4. Create Admin User

#### Windows:
```powershell
.\target\release\synker-server.exe --create-admin
```

#### Linux/macOS:
```bash
./target/release/synker-server --create-admin
```

### 5. Start the Server

#### Windows:
```powershell
.\target\release\synker-server.exe
```

#### Linux/macOS:
```bash
./target/release/synker-server
```

The server will start on `http://0.0.0.0:8080` by default.

## Deployment on MyCloud Device

### 1. Cross-compile for MyCloud (ARM64)

```bash
# Add the target
rustup target add aarch64-unknown-linux-gnu

# Install cross-compilation tools
sudo apt-get install gcc-aarch64-linux-gnu

# Build for ARM64
export CC_aarch64_unknown_linux_gnu=aarch64-linux-gnu-gcc
export CXX_aarch64_unknown_linux_gnu=aarch64-linux-gnu-g++
cargo build --release --target aarch64-unknown-linux-gnu
```

### 2. Copy to MyCloud Device

```bash
# Copy binary
scp target/aarch64-unknown-linux-gnu/release/synker-server root@192.168.1.100:/usr/local/bin/

# Copy configuration
scp config.toml root@192.168.1.100:/usr/local/etc/synker/

# Copy service file
scp synker-server.service root@192.168.1.100:/etc/systemd/system/
```

### 3. Setup on MyCloud

SSH into your MyCloud device:

```bash
ssh root@192.168.1.100

# Create synker user
adduser --system --group synker

# Create directories
mkdir -p /opt/synker
mkdir -p /opt/synker/storage
mkdir -p /opt/synker/temp
chown -R synker:synker /opt/synker

# Move files
mv /usr/local/bin/synker-server /opt/synker/
mv /usr/local/etc/synker/config.toml /opt/synker/
chmod +x /opt/synker/synker-server

# Initialize database
cd /opt/synker
sudo -u synker ./synker-server --init-db

# Enable and start service
systemctl enable synker-server
systemctl start synker-server
systemctl status synker-server
```

## Testing the Installation

### 1. Check Server Health

```bash
curl http://localhost:8080/health
```

Should return: `OK`

### 2. Test API

```bash
# Get server info
curl http://localhost:8080/

# Login (replace with your MyCloud credentials)
curl -X POST http://localhost:8080/api/v1/auth/login \
  -H "Content-Type: application/json" \
  -d '{
    "username": "your-username",
    "password": "your-password"
  }'
```

### 3. Upload Test File

```bash
# Get JWT token from login response above
TOKEN="your-jwt-token-here"

# Upload a file
curl -X POST http://localhost:8080/api/v1/files/upload?path=/ \
  -H "Authorization: Bearer $TOKEN" \
  -F "file=@/path/to/test-file.txt"
```

## Troubleshooting

### Common Issues

1. **Cargo not found**: Make sure Rust is installed and in your PATH
2. **Permission denied**: Ensure the synker user has proper permissions
3. **MyCloud connection failed**: Check IP address and credentials in config.toml
4. **Port already in use**: Change the port in config.toml
5. **Database locked**: Stop any running instances before initializing

### Log Files

Check logs for debugging:
```bash
journalctl -u synker-server -f
```

### Performance Tuning

For better performance on MyCloud devices:

1. Increase file limits:
```bash
echo "synker soft nofile 65536" >> /etc/security/limits.conf
echo "synker hard nofile 65536" >> /etc/security/limits.conf
```

2. Optimize SQLite settings in `config.toml`:
```toml
[database]
max_connections = 5  # Lower for ARM devices
connection_timeout_seconds = 60
```

3. Adjust file size limits based on available storage:
```toml
[filesystem]
max_file_size_mb = 512  # Reduce for limited storage
```

## Next Steps

After successful installation:

1. **Configure HTTPS**: Set up SSL/TLS certificates for secure access
2. **Firewall Rules**: Configure port forwarding for external access
3. **Client Applications**: Install client apps on your devices
4. **Backup Strategy**: Set up regular backups of the database and config
5. **Monitoring**: Set up log monitoring and alerting

## Support

If you encounter issues:

1. Check the troubleshooting section above
2. Review log files for error messages
3. Verify MyCloud API connectivity
4. Test with minimal configuration first
5. Create an issue on the GitHub repository with detailed error logs
