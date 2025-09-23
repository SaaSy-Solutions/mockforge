# Crypto Template Plugin

A MockForge template plugin that provides cryptographic functions for encryption, decryption, hashing, and secure random data generation in response templates.

## Features

- **AES-256-GCM Encryption**: Secure symmetric encryption with authenticated encryption
- **Password-Based Key Derivation**: Argon2 key derivation for secure password handling
- **Secure Random Generation**: Cryptographically secure random bytes and strings
- **Argon2 Hashing**: Memory-hard hashing for passwords and data integrity
- **Base64 Encoding**: Safe encoding for binary data in templates

## Installation

```bash
# Build the plugin
cargo build --target wasm32-wasi --release

# Install to MockForge
mockforge plugin install ./target/wasm32-wasi/release/mockforge_plugin_template_crypto.wasm
```

## Functions

### encrypt(data, [password], [salt])

Encrypt data using AES-256-GCM encryption.

```javascript
// Using default key
{{crypto.encrypt("sensitive data")}}

{{crypto.encrypt("secret message")}}
```

```javascript
// Using password-based encryption
{{crypto.encrypt("my data", "mypassword")}}

{{crypto.encrypt("confidential", "strongpassword", "somesalt")}}
```

**Parameters:**
- `data` (string): Data to encrypt
- `password` (string, optional): Password for key derivation
- `salt` (string, optional): Salt for key derivation

**Returns:** Base64-encoded encrypted data

### decrypt(encrypted_data, [password], [salt])

Decrypt AES-256-GCM encrypted data.

```javascript
// Decrypt data
{{crypto.decrypt("encrypted_base64_string")}}

{{crypto.decrypt(encrypted_data, "mypassword")}}
```

**Parameters:**
- `encrypted_data` (string): Base64-encoded encrypted data
- `password` (string, optional): Password for key derivation
- `salt` (string, optional): Salt for key derivation

**Returns:** Decrypted plaintext string

### random_bytes([length])

Generate cryptographically secure random bytes.

```javascript
// Generate 32 random bytes (default)
{{crypto.random_bytes()}}

// Generate 64 random bytes
{{crypto.random_bytes(64)}}
```

**Parameters:**
- `length` (integer, optional): Number of bytes (default: 32, max: 1024)

**Returns:** Base64-encoded random bytes

### random_string([length], [charset])

Generate a random string with specified charset.

```javascript
// Generate 16-character alphanumeric string (default)
{{crypto.random_string()}}

// Generate 32-character string
{{crypto.random_string(32)}}

// Generate hex string
{{crypto.random_string(16, "0123456789abcdef")}}
```

**Parameters:**
- `length` (integer, optional): String length (default: 16, max: 256)
- `charset` (string, optional): Character set to use

**Returns:** Random string

### hash(data, [salt])

Hash data using Argon2 password hashing.

```javascript
// Hash with auto-generated salt
{{crypto.hash("mypassword")}}

{{crypto.hash("data", "customsalt")}}
```

**Parameters:**
- `data` (string): Data to hash
- `salt` (string, optional): Salt for hashing

**Returns:** Argon2 hash string in PHC format

## Configuration

```yaml
plugins:
  template-crypto:
    default_key: "00112233445566778899aabbccddeeff00112233445566778899aabbccddeeff"  # 32 bytes hex
    key_derivation:
      algorithm: "argon2id"
      memory_cost_kib: 65536  # 64MB
      time_cost: 3           # iterations
      parallelism: 4         # threads
    random_limits:
      max_bytes: 1024
      max_string_length: 256
```

## Usage Examples

### Encrypted API Responses

```json
{
  "id": "{{uuid}}",
  "encrypted_data": "{{crypto.encrypt(user.secret_data, user.password)}}",
  "verification_hash": "{{crypto.hash(user.email)}}",
  "session_token": "{{crypto.random_string(32, 'ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789')}}"
}
```

### Secure Token Generation

```json
{
  "access_token": "{{crypto.random_bytes(32)}}",
  "refresh_token": "{{crypto.random_string(64)}}",
  "password_hash": "{{crypto.hash(user_input_password)}}"
}
```

### Encrypted User Data

```json
{
  "user_id": "{{faker.uuid}}",
  "encrypted_ssn": "{{crypto.encrypt(faker.ssn, 'app-secret')}}",
  "verification_token": "{{crypto.random_string(16, '0123456789')}}"
}
```

## Security Considerations

### Key Management
- **Never hardcode keys** in templates or configuration
- **Use environment variables** for sensitive keys
- **Rotate keys regularly** according to your security policy
- **Store keys securely** using proper key management systems

### Password Handling
- **Use strong passwords** with sufficient entropy
- **Salt passwords** properly for key derivation
- **Configure appropriate work factors** for your security requirements
- **Monitor performance impact** of cryptographic operations

### Resource Limits
- **Configure appropriate limits** based on your use case
- **Monitor resource usage** to prevent DoS attacks
- **Set reasonable timeouts** for cryptographic operations

## Performance Notes

- **Key derivation** (Argon2) is computationally expensive by design
- **Encryption/decryption** has moderate CPU overhead
- **Random generation** is fast but may block on entropy
- **Hash operations** scale with configured parameters

## Error Handling

The plugin returns appropriate error messages for common issues:

- **Invalid key**: Key derivation or decryption key issues
- **Resource limits**: Exceeded memory or CPU limits
- **Invalid input**: Malformed data or parameters
- **Crypto errors**: Underlying cryptographic operation failures

## Dependencies

This plugin uses the following cryptographic libraries:
- `aes-gcm`: AES-256-GCM authenticated encryption
- `argon2`: Password hashing and key derivation
- `rand`: Cryptographically secure random number generation
- `base64`: Safe encoding for binary data

## License

MIT OR Apache-2.0
