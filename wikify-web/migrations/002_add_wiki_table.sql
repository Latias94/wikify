-- Add wiki storage table for persistent wiki content
-- This migration adds support for storing generated wiki content in the database

-- Wiki content table
CREATE TABLE wikis (
    id TEXT PRIMARY KEY,
    repository_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT,
    content TEXT NOT NULL, -- Full wiki content (markdown or JSON)
    format TEXT DEFAULT 'markdown' CHECK (format IN ('markdown', 'json')),
    structure TEXT, -- JSON representation of the full WikiStructure
    generated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    updated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    version INTEGER DEFAULT 1,
    metadata TEXT DEFAULT '{}', -- JSON format metadata (generation config, stats, etc.)
    FOREIGN KEY (repository_id) REFERENCES repositories(id) ON DELETE CASCADE
);

-- Wiki pages table (for structured wiki content)
CREATE TABLE wiki_pages (
    id TEXT PRIMARY KEY,
    wiki_id TEXT NOT NULL,
    page_id TEXT NOT NULL, -- Page ID within the wiki structure
    title TEXT NOT NULL,
    content TEXT NOT NULL,
    description TEXT,
    importance TEXT DEFAULT 'Medium' CHECK (importance IN ('Critical', 'High', 'Medium', 'Low')),
    file_paths TEXT DEFAULT '[]', -- JSON array of relevant file paths
    related_pages TEXT DEFAULT '[]', -- JSON array of related page IDs
    parent_section TEXT, -- Parent section ID if any
    tags TEXT DEFAULT '[]', -- JSON array of tags
    reading_time INTEGER DEFAULT 1, -- Estimated reading time in minutes
    generated_at DATETIME DEFAULT CURRENT_TIMESTAMP,
    source_documents TEXT DEFAULT '[]', -- JSON array of source documents
    FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE
);

-- Wiki sections table (for hierarchical organization)
CREATE TABLE wiki_sections (
    id TEXT PRIMARY KEY,
    wiki_id TEXT NOT NULL,
    section_id TEXT NOT NULL, -- Section ID within the wiki structure
    title TEXT NOT NULL,
    description TEXT,
    pages TEXT DEFAULT '[]', -- JSON array of page IDs in this section
    subsections TEXT DEFAULT '[]', -- JSON array of subsection IDs
    importance TEXT DEFAULT 'Medium' CHECK (importance IN ('Critical', 'High', 'Medium', 'Low')),
    order_index INTEGER DEFAULT 0,
    FOREIGN KEY (wiki_id) REFERENCES wikis(id) ON DELETE CASCADE
);

-- Indexes for performance
CREATE INDEX idx_wikis_repository_id ON wikis(repository_id);
CREATE INDEX idx_wikis_generated_at ON wikis(generated_at DESC);
CREATE INDEX idx_wikis_updated_at ON wikis(updated_at DESC);

CREATE INDEX idx_wiki_pages_wiki_id ON wiki_pages(wiki_id);
CREATE INDEX idx_wiki_pages_page_id ON wiki_pages(page_id);
CREATE INDEX idx_wiki_pages_importance ON wiki_pages(importance);

CREATE INDEX idx_wiki_sections_wiki_id ON wiki_sections(wiki_id);
CREATE INDEX idx_wiki_sections_section_id ON wiki_sections(section_id);
CREATE INDEX idx_wiki_sections_order ON wiki_sections(order_index);

-- Unique constraints
CREATE UNIQUE INDEX idx_wikis_repository_unique ON wikis(repository_id);
CREATE UNIQUE INDEX idx_wiki_pages_unique ON wiki_pages(wiki_id, page_id);
CREATE UNIQUE INDEX idx_wiki_sections_unique ON wiki_sections(wiki_id, section_id);
