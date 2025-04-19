# Server Port Log

## UncivServer.kt -> unciv_server.rs

### **Source**: orig_src/server/src/com/unciv/app/server/UncivServer.kt
- **Destination**: rust_port/src/server/unciv_server.rs
- **Status**: Completed

### Key Changes

1. **Framework Migration**:
   - Replaced Ktor with Actix-web for the HTTP server
   - Replaced Clikt with Clap for command-line argument parsing
   - Replaced Kotlin coroutines with Rust async/await

2. **Authentication System**:
   - Implemented the same authentication mechanism using Base64 encoding
   - Maintained the same file-based storage for auth credentials
   - Preserved the same validation logic

3. **File Operations**:
   - Converted Java/Kotlin file operations to Rust equivalents
   - Used async file operations with tokio
   - Implemented proper error handling with Result types

4. **API Endpoints**:
   - Maintained the same API endpoints:
     - `/isalive` - Server status check
     - `/files/{fileName}` - File retrieval
     - `/files/{fileName}` (PUT) - File upload
     - `/auth` - Authentication status
     - `/auth` (PUT) - Set authentication password

5. **Configuration**:
   - Preserved the same command-line options:
     - `-p/--port` - Server port
     - `-f/--folder` - Multiplayer files folder
     - `-a/--auth` - Enable authentication
     - `-i/--Identify` - Display operator IPs

### Integration

- Added server module to the main application
- Added command-line option to start the server
- Updated Cargo.toml with necessary dependencies

### TODOs

- [ ] Add more comprehensive error handling
- [ ] Implement graceful shutdown
- [ ] Add unit tests for server functionality
- [ ] Consider using a more robust authentication system
- [ ] Add configuration file support