-- Migration: Fix doc_type column type
-- Date: 2024-09-05
-- Description: Convert doc_type column from enum to TEXT for dynamic type support
-- Note: This migration fixes the type mismatch between Rust String and PostgreSQL enum

-- Fix doc_type column type (convert from enum to TEXT if it exists as enum)
DO $$
BEGIN
    -- Check if doc_type column exists and is an enum
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'documents'
        AND column_name = 'doc_type'
        AND data_type = 'USER-DEFINED'
    ) THEN
        -- Convert enum column to TEXT
        ALTER TABLE documents ALTER COLUMN doc_type TYPE TEXT;
        RAISE NOTICE 'Converted doc_type column from enum to TEXT';
    END IF;

    -- Also check document_sources table
    IF EXISTS (
        SELECT 1 FROM information_schema.columns
        WHERE table_name = 'document_sources'
        AND column_name = 'doc_type'
        AND data_type = 'USER-DEFINED'
    ) THEN
        -- Convert enum column to TEXT
        ALTER TABLE document_sources ALTER COLUMN doc_type TYPE TEXT;
        RAISE NOTICE 'Converted doc_type column in document_sources from enum to TEXT';
    END IF;
EXCEPTION
    WHEN OTHERS THEN
        RAISE NOTICE 'Could not convert doc_type column: %', SQLERRM;
END $$;

