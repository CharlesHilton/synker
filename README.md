# Synker Server

A self-hosted cloud storage server for MyCloud OS5 that provides OneDrive-like functionality with native MyCloud authentication.

## Features

- **MyCloud Authentication**: Integrates with MyCloud OS5 user management
- **File Upload/Download**: Support for large files with resumable transfers
- **File Synchronization**: Real-time sync across multiple devices
- **Folder Management**: Create, delete, and organize folders
- **File Sharing**: Generate secure share links with expiration and download limits
- **User Permissions**: Granular access control based on MyCloud user groups
- **REST API**: Complete RESTful API for client applications
- **WebSocket Support**: Real-time notifications and live sync
- **Cross-Platform**: Supports Windows, Mac, Linux, and Android clients

## Quick Start

### Prerequisites

- Western Digital MyCloud PR4100 running MyCloud OS 5
- Rust 1.70+ installed
- Network access to your MyCloud device

### Installation

1. Clone the repository:
```bash
git clone https://github.com/your-username/synker.git
cd synker
```

2. Build the server:
```bash
cargo build --release
```

3. Copy the binary to your MyCloud device:
```bash
scp target/release/synker-server root@your-mycloud-ip:/usr/local/bin/
```

### Configuration

1. Edit the configuration file:
```bash
nano config.toml
```

2. Update the MyCloud settings:
```toml
[mycloud]
api_endpoint = "http://192.168.1.100"  # Your MyCloud IP
admin_username = "admin"
admin_password = "your-admin-password"
```

3. Initialize the database:
```bash
./synker-server --init-db
```

4. Create the initial admin user:
```bash
./synker-server --create-admin
```

### Running the Server

```bash
./synker-server
```

The server will start on `http://0.0.0.0:8080` by default.

## API Documentation

### Authentication

#### Login
```http
POST /api/v1/auth/login
Content-Type: application/json

{
    "username": "your-mycloud-username",
    "password": "your-mycloud-password",
    "device_id": "optional-device-id",
    "device_name": "optional-device-name"
}
```

Response:
```json
{
    "success": true,
    "data": {
        "token": "jwt-token-here",
        "user": { ... },
        "expires_at": "2025-07-29T12:00:00Z"
    }
}
```

### File Operations

#### Upload File
```http
POST /api/v1/files/upload?path=/folder/
Authorization: Bearer your-jwt-token
Content-Type: multipart/form-data

file: [binary data]
```

#### Download File
```http
GET /api/v1/files/download/path/to/file.txt
Authorization: Bearer your-jwt-token
```

#### List Files
```http
GET /api/v1/files/list?path=/folder/
Authorization: Bearer your-jwt-token
```

#### Delete File
```http
DELETE /api/v1/files/delete/path/to/file.txt
Authorization: Bearer your-jwt-token
```

### Folder Operations

#### Create Folder
```http
POST /api/v1/folders/create
Authorization: Bearer your-jwt-token
Content-Type: application/json

{
    "path": "/parent/folder/",
    "name": "new-folder"
}
```

### Synchronization

#### Sync Files
```http
POST /api/v1/sync
Authorization: Bearer your-jwt-token
Content-Type: application/json

{
    "folders": ["/Documents/", "/Photos/"],
    "last_sync": "2025-07-28T12:00:00Z"
}
```

### File Sharing

#### Create Share Link
```http
POST /api/v1/share/file-uuid-here?expires_in_hours=24&max_downloads=10
Authorization: Bearer your-jwt-token
```

## Architecture

The Synker Server is built with:

- **Rust**: High-performance, memory-safe systems programming
- **Axum**: Modern async web framework
- **SQLite**: Embedded database for metadata
- **JWT**: Secure token-based authentication
- **MyCloud API**: Integration with MyCloud OS5

### Project Structure

```
server/
├── synker_server.rs    # Main application entry point
├── types.rs           # Data structures and API types
├── database.rs        # Database operations and queries
├── auth.rs           # Authentication and JWT handling
├── filesystem.rs     # File system operations
├── handlers.rs       # HTTP request handlers
├── config.rs         # Configuration management
└── mycloud.rs        # MyCloud OS5 integration
```

## Development

### Building from Source

```bash
# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Clone and build
git clone https://github.com/your-username/synker.git
cd synker
cargo build --release
```

### Running Tests

```bash
cargo test
```

### Development Mode

```bash
cargo run -- --debug
```

## License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.