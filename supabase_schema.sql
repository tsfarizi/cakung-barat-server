-- Supabase schema matching the current rocksDB structure

-- Create assets table
CREATE TABLE IF NOT EXISTS assets (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT NOT NULL,
    filename TEXT NOT NULL,
    url TEXT NOT NULL,
    description TEXT,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create postings table
CREATE TABLE IF NOT EXISTS postings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    judul TEXT NOT NULL,
    tanggal DATE NOT NULL,
    detail TEXT NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create posting_assets junction table for many-to-many relationship
CREATE TABLE IF NOT EXISTS posting_assets (
    posting_id UUID REFERENCES postings(id) ON DELETE CASCADE,
    asset_id UUID REFERENCES assets(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (posting_id, asset_id)
);

-- Create folders table for asset organization
CREATE TABLE IF NOT EXISTS folders (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name TEXT UNIQUE NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW()
);

-- Create asset_folders junction table for many-to-many relationship
CREATE TABLE IF NOT EXISTS asset_folders (
    asset_id UUID REFERENCES assets(id) ON DELETE CASCADE,
    folder_id UUID REFERENCES folders(id) ON DELETE CASCADE,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    PRIMARY KEY (asset_id, folder_id)
);

-- Create indexes for better performance
CREATE INDEX IF NOT EXISTS idx_postings_created_at ON postings(created_at);
CREATE INDEX IF NOT EXISTS idx_assets_filename ON assets(filename);
CREATE INDEX IF NOT EXISTS idx_posting_assets_posting_id ON posting_assets(posting_id);
CREATE INDEX IF NOT EXISTS idx_posting_assets_asset_id ON posting_assets(asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_folders_asset_id ON asset_folders(asset_id);
CREATE INDEX IF NOT EXISTS idx_asset_folders_folder_id ON asset_folders(folder_id);

-- Create updated_at trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Create triggers to update updated_at column
CREATE TRIGGER update_assets_updated_at 
    BEFORE UPDATE ON assets 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_postings_updated_at 
    BEFORE UPDATE ON postings 
    FOR EACH ROW 
    EXECUTE FUNCTION update_updated_at_column();