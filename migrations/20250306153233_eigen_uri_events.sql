-- Create table for AVS metadata
CREATE TABLE eigen_avs_metadata (
    id SERIAL PRIMARY KEY,
    -- Event data
    address CHAR(42) NOT NULL, -- Ethereum address with 0x prefix
    block_number BIGINT NOT NULL,
    log_index INTEGER NOT NULL,
    metadata_uri TEXT NOT NULL,
    
    -- Metadata content
    name TEXT,
    description TEXT,
    website TEXT,
    logo TEXT,
    twitter TEXT,
    
    -- Timestamps
    created_at TIMESTAMP NOT NULL,
    
    -- Create a unique constraint for address and block_number and log_index
    UNIQUE(address, block_number, log_index)
);

-- Create an index to quickly find the latest metadata for an address
CREATE INDEX idx_eigen_avs_metadata_address_block_log ON eigen_avs_metadata (address, block_number DESC, log_index DESC);
CREATE INDEX idx_eigen_avs_metadata_address ON eigen_avs_metadata (address DESC);