# FTP Examples

This section provides complete examples of using MockForge FTP for various testing scenarios.

## Basic FTP Server

### Starting a Simple Server

```bash
# Start FTP server on default port 2121
mockforge ftp serve

# Start on custom port
mockforge ftp serve --port 2122

# Start with custom host
mockforge ftp serve --host 0.0.0.0 --port 2121
```

### Connecting with FTP Clients

#### Using lftp

```bash
# Connect to the server
lftp localhost:2121

# List files
lftp localhost:2121:~> ls

# Download a file
lftp localhost:2121:~> get test.txt

# Upload a file
lftp localhost:2121:~> put localfile.txt

# Exit
lftp localhost:2121:~> quit
```

#### Using curl

```bash
# List directory
curl ftp://localhost:2121/

# Download file
curl ftp://localhost:2121/test.txt -o downloaded.txt

# Upload file
curl -T localfile.txt ftp://localhost:2121/
```

#### Using Python

```python
import ftplib

# Connect to FTP server
ftp = ftplib.FTP('localhost', 'anonymous', '')

# List files
files = ftp.nlst()
print("Files:", files)

# Download file
with open('downloaded.txt', 'wb') as f:
    ftp.retrbinary('RETR test.txt', f.write)

# Upload file
with open('localfile.txt', 'rb') as f:
    ftp.storbinary('STOR uploaded.txt', f)

ftp.quit()
```

## File Management Examples

### Adding Static Files

```bash
# Add a simple text file
mockforge ftp vfs add /hello.txt --content "Hello, FTP World!"

# Add a JSON file
mockforge ftp vfs add /config.json --content '{"server": "mockforge", "port": 2121}'

# Add a larger file
echo "This is a test file with multiple lines." > test.txt
mockforge ftp vfs add /multiline.txt --content "$(cat test.txt)"
```

### Adding Template Files

```bash
# Add a dynamic JSON response
mockforge ftp vfs add /user.json --template '{"id": "{{uuid}}", "name": "{{faker.name}}", "created": "{{now}}"}'

# Add a log file with timestamps
mockforge ftp vfs add /server.log --template '[{{timestamp}}] Server started at {{time}}'

# Add a status file
mockforge ftp vfs add /status.xml --template '<?xml version="1.0"?><status><server>MockForge</server><time>{{now}}</time></status>'
```

### Adding Generated Files

```bash
# Add a random binary file (1KB)
mockforge ftp vfs add /random.bin --generate random --size 1024

# Add a file filled with zeros (512 bytes)
mockforge ftp vfs add /zeros.dat --generate zeros --size 512

# Add an incremental pattern file
mockforge ftp vfs add /pattern.bin --generate incremental --size 256
```

### Managing Files

```bash
# List all files
mockforge ftp vfs list /

# Get file information
mockforge ftp vfs info /hello.txt

# Remove a file
mockforge ftp vfs remove /old-file.txt
```

## Configuration Examples

### Basic Configuration File

```yaml
# ftp-config.yaml
ftp:
  host: "127.0.0.1"
  port: 2121
  virtual_root: "/"
  fixtures:
    - name: "basic_files"
      description: "Basic test files"
      virtual_files:
        - path: "/readme.txt"
          content:
            type: "static"
            content: "Welcome to MockForge FTP Server"
          permissions: "644"
          owner: "ftp"
          group: "ftp"
      upload_rules:
        - path_pattern: "/uploads/.*"
          auto_accept: true
          storage:
            type: "memory"
```

### Advanced Configuration

```yaml
# advanced-ftp-config.yaml
ftp:
  host: "0.0.0.0"
  port: 2121
  virtual_root: "/ftp"
  fixtures:
    - name: "api_test_files"
      description: "Files for API testing"
      virtual_files:
        # Static files
        - path: "/api/v1/users"
          content:
            type: "static"
            content: '[{"id": 1, "name": "Alice"}, {"id": 2, "name": "Bob"}]'
          permissions: "644"
          owner: "api"
          group: "users"

        # Template files
        - path: "/api/v1/status"
          content:
            type: "template"
            template: '{"status": "ok", "timestamp": "{{now}}", "version": "1.0.0"}'
          permissions: "644"
          owner: "api"
          group: "system"

        # Generated test data
        - path: "/test/data.bin"
          content:
            type: "generated"
            size: 1048576  # 1MB
            pattern: "random"
          permissions: "644"
          owner: "test"
          group: "data"

      upload_rules:
        # General uploads
        - path_pattern: "/uploads/.*"
          auto_accept: true
          validation:
            max_size_bytes: 10485760  # 10MB
          storage:
            type: "memory"

        # Image uploads
        - path_pattern: "/images/.*"
          auto_accept: true
          validation:
            max_size_bytes: 5242880  # 5MB
            allowed_extensions: ["jpg", "png", "gif"]
          storage:
            type: "file"
            path: "/tmp/ftp/images"

        # Log files (accepted but discarded)
        - path_pattern: "/logs/.*"
          auto_accept: true
          storage:
            type: "discard"
```

## Testing Scenarios

### File Upload Testing

```bash
# Start server with upload configuration
mockforge ftp serve --config upload-config.yaml

# Test file upload with curl
echo "Test file content" > test.txt
curl -T test.txt ftp://localhost:2121/uploads/

# Test large file upload
dd if=/dev/zero of=large.bin bs=1M count=5
curl -T large.bin ftp://localhost:2121/uploads/

# Test invalid file type
echo "invalid content" > invalid.exe
curl -T invalid.exe ftp://localhost:2121/uploads/  # Should fail
```

### Load Testing

```bash
# Start server
mockforge ftp serve --port 2121 &

# Simple load test with parallel uploads
for i in {1..10}; do
  echo "File $i content" > "file$i.txt"
  curl -T "file$i.txt" "ftp://localhost:2121/uploads/file$i.txt" &
done
wait
```

### Integration Testing

#### With pytest

```python
# test_ftp_integration.py
import ftplib
import pytest
import tempfile
import os

class TestFTPIntegration:
    @pytest.fixture(scope="class")
    def ftp_client(self):
        # Connect to MockForge FTP server
        ftp = ftplib.FTP('localhost', 'anonymous', '')
        yield ftp
        ftp.quit()

    def test_list_files(self, ftp_client):
        files = ftp_client.nlst()
        assert len(files) >= 0  # At least empty directory

    def test_download_file(self, ftp_client):
        # Assuming server has a test file
        with tempfile.NamedTemporaryFile(delete=False) as tmp:
            try:
                ftp_client.retrbinary('RETR test.txt', tmp.write)
                assert os.path.getsize(tmp.name) > 0
            finally:
                os.unlink(tmp.name)

    def test_upload_file(self, ftp_client):
        # Create test file
        with tempfile.NamedTemporaryFile(mode='w', delete=False) as tmp:
            tmp.write("Test upload content")
            tmp_path = tmp.name

        try:
            # Upload file
            with open(tmp_path, 'rb') as f:
                ftp_client.storbinary('STOR uploaded.txt', f)

            # Verify upload (if server supports listing uploads)
            files = ftp_client.nlst()
            assert 'uploaded.txt' in [os.path.basename(f) for f in files]
        finally:
            os.unlink(tmp_path)
```

#### With Java

```java
// FtpIntegrationTest.java
import org.apache.commons.net.ftp.FTPClient;
import org.junit.jupiter.api.*;
import java.io.*;

class FtpIntegrationTest {
    private FTPClient ftpClient;

    @BeforeEach
    void setup() throws IOException {
        ftpClient = new FTPClient();
        ftpClient.connect("localhost", 2121);
        ftpClient.login("anonymous", "");
        ftpClient.enterLocalPassiveMode();
    }

    @AfterEach
    void teardown() throws IOException {
        if (ftpClient.isConnected()) {
            ftpClient.disconnect();
        }
    }

    @Test
    void testFileDownload() throws IOException {
        // Download a file
        File tempFile = File.createTempFile("downloaded", ".txt");
        try (FileOutputStream fos = new FileOutputStream(tempFile)) {
            boolean success = ftpClient.retrieveFile("test.txt", fos);
            Assertions.assertTrue(success, "File download should succeed");
            Assertions.assertTrue(tempFile.length() > 0, "Downloaded file should not be empty");
        } finally {
            tempFile.delete();
        }
    }

    @Test
    void testFileUpload() throws IOException {
        // Create test file
        File tempFile = File.createTempFile("upload", ".txt");
        try (FileWriter writer = new FileWriter(tempFile)) {
            writer.write("Test upload content");
        }

        // Upload file
        try (FileInputStream fis = new FileInputStream(tempFile)) {
            boolean success = ftpClient.storeFile("uploaded.txt", fis);
            Assertions.assertTrue(success, "File upload should succeed");
        } finally {
            tempFile.delete();
        }
    }

    @Test
    void testDirectoryListing() throws IOException {
        FTPFile[] files = ftpClient.listFiles();
        Assertions.assertNotNull(files, "Directory listing should not be null");
        // Additional assertions based on expected files
    }
}
```

## Docker Integration

### Running in Docker

```dockerfile
# Dockerfile
FROM mockforge:latest

# Copy FTP configuration
COPY ftp-config.yaml /app/config/

# Expose FTP port
EXPOSE 2121

# Start FTP server
CMD ["mockforge", "ftp", "serve", "--config", "/app/config/ftp-config.yaml"]
```

```bash
# Build and run
docker build -t mockforge-ftp .
docker run -p 2121:2121 mockforge-ftp
```

### Docker Compose

```yaml
# docker-compose.yml
version: '3.8'
services:
  ftp-server:
    image: mockforge:latest
    command: ["mockforge", "ftp", "serve", "--host", "0.0.0.0"]
    ports:
      - "2121:2121"
    volumes:
      - ./ftp-config.yaml:/app/config/ftp-config.yaml
      - ./uploads:/tmp/uploads
    environment:
      - RUST_LOG=info
```

## CI/CD Integration

### GitHub Actions Example

```yaml
# .github/workflows/ftp-test.yml
name: FTP Integration Tests

on: [push, pull_request]

jobs:
  ftp-test:
    runs-on: ubuntu-latest

    steps:
    - uses: actions/checkout@v3

    - name: Setup Rust
      uses: actions-rust-lang/setup-rust-toolchain@v1

    - name: Build MockForge
      run: cargo build --release

    - name: Start FTP Server
      run: |
        ./target/release/mockforge ftp serve --port 2121 &
        sleep 2

    - name: Run FTP Tests
      run: |
        # Test with lftp
        sudo apt-get update && sudo apt-get install -y lftp
        echo "Test file content" > test.txt
        lftp -c "open localhost:2121; put test.txt; ls; get test.txt -o downloaded.txt; quit"

        # Verify files
        test -f downloaded.txt
        grep -q "Test file content" downloaded.txt
```

### Jenkins Pipeline

```groovy
// Jenkinsfile
pipeline {
    agent any

    stages {
        stage('FTP Integration Test') {
            steps {
                sh 'cargo build --release'

                // Start FTP server in background
                sh './target/release/mockforge ftp serve --port 2121 &'
                sh 'sleep 3'

                // Run tests
                sh '''
                # Install FTP client
                apt-get update && apt-get install -y lftp

                # Create test file
                echo "Integration test content" > test.txt

                # Test FTP operations
                lftp -c "
                  open localhost:2121
                  put test.txt
                  ls
                  get test.txt -o downloaded.txt
                  quit
                "

                # Verify
                grep -q "Integration test content" downloaded.txt
                '''
            }
        }
    }
}
```

## Troubleshooting

### Common Issues

#### Connection Refused
```bash
# Check if server is running
netstat -tlnp | grep 2121

# Check server logs
mockforge ftp serve --port 2121 2>&1
```

#### Passive Mode Issues
```bash
# FTP clients may need passive mode
curl --ftp-pasv ftp://localhost:2121/
```

#### File Permission Issues
```bash
# Check file permissions in VFS
mockforge ftp vfs info /problematic-file.txt

# Check upload rules
mockforge ftp fixtures validate config.yaml
```

#### Memory Issues
```bash
# Monitor memory usage
ps aux | grep mockforge

# Use file storage for large files
# Configure storage type in upload rules
```

This completes the FTP implementation for MockForge. The server provides comprehensive FTP mocking capabilities with virtual file systems, template rendering, and configurable upload handling.