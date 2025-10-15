# Getting Started with FTP Mocking

MockForge provides comprehensive FTP server mocking capabilities, allowing you to simulate FTP file transfers for testing and development purposes.

## Quick Start

### Starting an FTP Server

```bash
# Start a basic FTP server on port 2121
mockforge ftp serve --port 2121

# Start with custom configuration
mockforge ftp serve --host 0.0.0.0 --port 2121 --virtual-root /ftp
```

### Connecting with an FTP Client

Once the server is running, you can connect using any FTP client:

```bash
# Using lftp
lftp ftp://localhost:2121

# Using curl
curl ftp://localhost:2121/

# Using FileZilla or other GUI clients
# Host: localhost
# Port: 2121
# Username: (leave blank for anonymous)
# Password: (leave blank)
```

## Basic Concepts

### Virtual File System

MockForge FTP uses an in-memory virtual file system that supports:

- **Static files**: Pre-defined content
- **Template files**: Dynamic content generation using Handlebars
- **Generated files**: Synthetic content (random, zeros, patterns)
- **Upload handling**: Configurable validation and storage rules

### File Content Types

#### Static Content
```bash
# Add a static file
mockforge ftp vfs add /hello.txt --content "Hello, World!"
```

#### Template Content
```bash
# Add a template file with dynamic content
mockforge ftp vfs add /user.json --template '{"name": "{{faker.name}}", "id": "{{uuid}}", "timestamp": "{{now}}"}'
```

#### Generated Content
```bash
# Add a file with random content
mockforge ftp vfs add /random.bin --generate random --size 1024

# Add a file filled with zeros
mockforge ftp vfs add /zeros.bin --generate zeros --size 1024
```

## FTP Commands Supported

MockForge supports standard FTP commands:

- `LIST` - Directory listing
- `RETR` - Download files
- `STOR` - Upload files
- `DELE` - Delete files
- `PWD` - Print working directory
- `SIZE` - Get file size
- `CWD` - Change directory (limited support)

## Example Session

```bash
$ mockforge ftp serve --port 2121 &
$ lftp localhost:2121
lftp localhost:2121:~> ls
-rw-r--r-- 1 mockforge ftp          0 Jan 01 00:00 test.txt
lftp localhost:2121:~> put localfile.txt
lftp localhost:2121:~> get test.txt
lftp localhost:2121:~> quit
```

## Next Steps

- [Configuration](configuration.md) - Advanced server configuration
- [Fixtures](fixtures.md) - Pre-configured file structures
- [Examples](examples.md) - Complete usage examples