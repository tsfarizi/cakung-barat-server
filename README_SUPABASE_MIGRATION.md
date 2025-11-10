# Cakung Barat Server - Supabase Migration

This project has been migrated from rocksDB to Supabase for both database and file storage.

## Environment Variables

Create a `.env` file based on `.env.example` and set the following values:

- `SUPABASE_URL`: Your Supabase project URL (e.g., https://your-project.supabase.co)
- `SUPABASE_ANON_KEY`: Your Supabase anon key
- `SUPABASE_SERVICE_ROLE_KEY`: Your Supabase service role key (for server-side operations)
- `SUPABASE_DATABASE_URL`: Your PostgreSQL connection string for direct database access
- `BUCKET_NAME`: The name of your Supabase storage bucket (default: cakung-barat-supabase-bucket)

## Supabase Setup

1. Create a Supabase project at [supabase.com](https://supabase.com)
2. Create a storage bucket named `cakung-barat-supabase-bucket`
3. Configure the database by running the schema from `supabase_schema.sql`
4. Set up your environment variables as described above

## Database Schema

The schema is defined in `supabase_schema.sql` and includes:

- `assets` table: Store asset information (id, name, filename, url, description)
- `postings` table: Store posting information (id, judul, tanggal, detail)
- `posting_assets` table: Junction table for many-to-many relationship between postings and assets
- `folders` table: Store folder information for asset organization
- `asset_folders` table: Junction table for many-to-many relationship between assets and folders

## Key Changes from RocksDB

- Database operations now use PostgreSQL via Supabase
- File storage now uses Supabase Storage instead of local filesystem
- All API endpoints remain the same for frontend compatibility
- Asset serving now redirects to Supabase storage public URLs

## Running the Application

1. Ensure your Supabase project is set up and configured
2. Install dependencies: `cargo build`
3. Set up your environment variables
4. Run the application: `cargo run`

## Migrating Existing Data

If you have existing data in rocksDB that needs to be migrated to Supabase, you'll need to write a custom migration script to extract data from the rocksDB files and insert it into Supabase.

## SSL/TLS Configuration

If you encounter SSL certificate verification errors (especially on Windows or in certain deployment environments), you can configure the TLS verification behavior:

- `TLS_VERIFY`: Set to `true` (default) to enable SSL certificate verification, or `false` to disable it (not recommended for production)

This setting helps resolve issues like "certificate verify failed" errors when connecting to Supabase storage.

For production environments, ensure that your deployment environment has proper CA certificates installed. On Windows systems, the application will automatically attempt to locate system certificates using the `openssl-probe` crate.