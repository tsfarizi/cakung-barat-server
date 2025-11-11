# Cakung Barat Server

A Rust-based web server built with Actix Web for managing postings and assets with Supabase integration. This server provides REST API endpoints for creating, reading, updating, and deleting postings and assets, with file storage capabilities using Supabase Storage.

## Table of Contents
- [Overview](#overview)
- [Features](#features)
- [Architecture](#architecture)
- [Dependencies](#dependencies)
- [Installation](#installation)
- [Configuration](#configuration)
- [Usage](#usage)
- [API Endpoints](#api-endpoints)
- [Folder Structure](#folder-structure)
- [Environment Variables](#environment-variables)
- [Development](#development)
- [License](#license)

## Overview

The Cakung Barat Server is a backend service designed to manage content postings and their associated media assets. It uses PostgreSQL via Supabase for database operations and Supabase Storage for file management. The application features API documentation through Swagger UI and provides CORS support for web applications.

## Features

- **RESTful API**: Complete CRUD operations for postings and assets
- **File Upload**: Supports multipart file uploads with automatic Supabase Storage integration
- **Folder Organization**: Assets can be organized into folders for better management
- **Asset-Posting Association**: Media assets can be linked to specific postings
- **Structured Response**: Organized asset responses by folders with unassigned assets
- **API Documentation**: Built-in Swagger UI for API exploration
- **CORS Support**: Configured for cross-origin requests from multiple domains
- **Error Handling**: Comprehensive error responses with timestamps
- **UUID Support**: Uses UUIDs for reliable resource identification

## Architecture

The application follows a modular architecture with:

- **Main Module**: Entry point with HTTP server setup and routing
- **Database Module**: PostgreSQL integration with SQLx and connection pooling
- **Asset Module**: Handles asset management, uploads, and folder organization
- **Posting Module**: Manages content postings with associated assets
- **Storage Module**: Supabase integration for file storage operations

## Dependencies

This project relies on several Rust crates:

- `actix-web`: Web framework for building HTTP services
- `sqlx`: Async SQL toolkit for Rust with compile-time verification
- `serde`: Serialization/deserialization framework
- `uuid`: UUID generation and parsing
- `tokio`: Async runtime
- `utoipa`: OpenAPI documentation generation
- `utoipa-swagger-ui`: Swagger UI integration
- `chrono`: Date and time manipulation
- `postgrest`: Supabase PostgREST client
- `supabase_rs`: Supabase Rust bindings
- `reqwest`: HTTP client
- `actix-cors`: CORS middleware for Actix Web
- `dotenvy`: Environment variable loading
- `anyhow`: Error handling
- `actix-multipart`: Multipart form data handling
- `sanitize-filename`: Filename sanitization

## Installation

1. **Prerequisites**:
   - Rust programming language (1.70 or higher)
   - Git
   - Access to a Supabase project

2. **Clone the repository**:
   ```bash
   git clone https://github.com/your-username/cakung-barat-server.git
   cd cakung-barat-server
   ```

3. **Install dependencies**:
   ```bash
   cargo build
   ```

## Configuration

1. **Environment Variables**: Copy `.env.example` to `.env` and fill in your Supabase credentials:
   ```bash
   cp .env.example .env
   ```

2. **Supabase Setup**: Create a Supabase project and configure:
   - Database with schema from `supabase_schema.sql`
   - Storage bucket (default: `cakung-barat-supabase-bucket`)

3. **Database Schema**: Run the schema from `supabase_schema.sql` in your Supabase database

## Usage

1. **Run the development server**:
   ```bash
   cargo run
   ```

2. **Access the application**:
   - API: `http://localhost:8080/api`
   - API Documentation: `http://localhost:8080/swagger-ui/`

3. **Build for production**:
   ```bash
   cargo build --release
   ```

## API Endpoints

### Posting Service
- `GET /api/postings` - Retrieve all postings with associated assets
- `GET /api/postings/{id}` - Retrieve a specific posting by ID
- `POST /api/postings` - Create a new posting
- `PUT /api/postings/{id}` - Update an existing posting
- `DELETE /api/postings/{id}` - Delete a posting

### Asset Service
- `GET /api/assets` - Retrieve all assets organized by folders
- `POST /api/assets` - Upload a new asset
- `GET /api/assets/{id}` - Retrieve a specific asset by ID
- `DELETE /api/assets/{id}` - Delete an asset
- `GET /api/assets/serve/{filename}` - Serve an asset file
- `POST /api/assets/folders` - Create a new folder
- `GET /api/assets/folders/{folder_name}` - List assets in a specific folder

## Folder Structure

```
cakung-barat-server/
├── Cargo.toml          # Rust project configuration and dependencies
├── Cargo.lock          # Dependency lock file
├── README.md           # This file
├── README_SUPABASE_MIGRATION.md  # Migration from rocksDB to Supabase
├── .env.example        # Example environment variables file
├── dockerfile          # Docker configuration
├── supabase_schema.sql # Database schema for Supabase
├── supabase_seed.sql   # Initial database seed data
├── src/
│   ├── main.rs         # Application entry point
│   ├── db.rs           # Database connection and queries
│   ├── schema.rs       # Database schema definitions
│   ├── storage.rs      # File storage operations
│   ├── asset/
│   │   ├── handlers.rs # Asset API endpoints
│   │   └── models.rs   # Asset data models
│   ├── posting/
│   │   ├── handlers.rs # Posting API endpoints
│   │   └── models.rs   # Posting data models
└── target/             # Compiled binaries (gitignored)
```

## Environment Variables

The application requires the following environment variables:

- `SUPABASE_URL`: Your Supabase project URL (e.g., https://your-project.supabase.co)
- `SUPABASE_ANON_KEY`: Your Supabase anon key
- `SUPABASE_SERVICE_ROLE_KEY`: Your Supabase service role key (for server-side operations)
- `SUPABASE_DATABASE_URL`: Your PostgreSQL connection string for direct database access
- `BUCKET_NAME`: The name of your Supabase storage bucket (default: cakung-barat-supabase-bucket)
- `TLS_VERIFY`: Enable SSL certificate verification (default: true)

## Development

### Running Tests
```bash
cargo test
```

### Code Formatting
```bash
cargo fmt
```

### Code Linting
```bash
cargo clippy
```

### Running with Logging
```bash
RUST_LOG=debug cargo run
```

## Advantages

1. **Scalable Architecture**: Built with async Rust for high performance and concurrency
2. **Modern Tech Stack**: Uses Actix Web and Supabase for reliable infrastructure
3. **Type Safety**: Rust's compile-time guarantees prevent many runtime errors
4. **API Documentation**: Auto-generated Swagger UI for easy API exploration
5. **Flexible Asset Management**: Supports folder organization and asset-posting associations
6. **Production Ready**: Includes proper error handling, logging, and CORS configuration
7. **Database Safety**: SQLx provides compile-time SQL verification
8. **Open Source**: Fully open source with MIT license

## License

This project is licensed under the MIT License - see the LICENSE file for details.