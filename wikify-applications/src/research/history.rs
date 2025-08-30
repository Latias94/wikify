//! Research history storage and management

use super::types::{ResearchContext, ResearchIteration, ResearchSummary};
use crate::{ApplicationError, ApplicationResult};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use tracing::{debug, info};

#[cfg(feature = "openapi")]
use utoipa::ToSchema;

/// Research history record
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchHistoryRecord {
    /// Research session ID
    pub session_id: String,
    /// Research topic
    pub topic: String,
    /// Template used (if any)
    pub template_id: Option<String>,
    /// Research context
    pub context: ResearchContext,
    /// All iterations performed
    pub iterations: Vec<ResearchIteration>,
    /// Final summary (if completed)
    pub summary: Option<ResearchSummary>,
    /// Research status
    pub status: ResearchStatus,
    /// Creation timestamp
    pub created_at: DateTime<Utc>,
    /// Last updated timestamp
    pub updated_at: DateTime<Utc>,
    /// Completion timestamp (if completed)
    pub completed_at: Option<DateTime<Utc>>,
    /// Research metadata
    pub metadata: ResearchMetadata,
}

/// Research status enumeration
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub enum ResearchStatus {
    /// Research is in progress
    InProgress,
    /// Research completed successfully
    Completed,
    /// Research was cancelled
    Cancelled,
    /// Research failed with error
    Failed(String),
}

/// Research metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchMetadata {
    /// Total number of iterations
    pub total_iterations: usize,
    /// Total number of questions asked
    pub total_questions: usize,
    /// Total number of sources consulted
    pub total_sources: usize,
    /// Research duration in seconds
    pub duration_seconds: Option<u64>,
    /// User who initiated the research
    pub user_id: Option<String>,
    /// Repository or session context
    pub repository_context: Option<String>,
}

/// Research history storage interface
#[allow(async_fn_in_trait)]
pub trait ResearchHistoryStorage: Send + Sync {
    /// Save research record
    async fn save_record(&self, record: &ResearchHistoryRecord) -> ApplicationResult<()>;

    /// Load research record by session ID
    async fn load_record(
        &self,
        session_id: &str,
    ) -> ApplicationResult<Option<ResearchHistoryRecord>>;

    /// Update research record
    async fn update_record(&self, record: &ResearchHistoryRecord) -> ApplicationResult<()>;

    /// List research records with optional filters
    async fn list_records(
        &self,
        filters: &ResearchHistoryFilters,
    ) -> ApplicationResult<Vec<ResearchHistoryRecord>>;

    /// Delete research record
    async fn delete_record(&self, session_id: &str) -> ApplicationResult<()>;

    /// Get research statistics
    async fn get_statistics(&self) -> ApplicationResult<ResearchStatistics>;
}

/// Research history filters
#[derive(Debug, Clone, Default)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchHistoryFilters {
    /// Filter by user ID
    pub user_id: Option<String>,
    /// Filter by status
    pub status: Option<ResearchStatus>,
    /// Filter by template ID
    pub template_id: Option<String>,
    /// Filter by date range (start)
    pub date_from: Option<DateTime<Utc>>,
    /// Filter by date range (end)
    pub date_to: Option<DateTime<Utc>>,
    /// Limit number of results
    pub limit: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
}

/// Research statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
#[cfg_attr(feature = "openapi", derive(ToSchema))]
pub struct ResearchStatistics {
    /// Total number of research sessions
    pub total_sessions: usize,
    /// Number of completed sessions
    pub completed_sessions: usize,
    /// Number of in-progress sessions
    pub in_progress_sessions: usize,
    /// Number of failed sessions
    pub failed_sessions: usize,
    /// Average research duration in seconds
    pub average_duration_seconds: Option<f64>,
    /// Most used templates
    pub popular_templates: Vec<(String, usize)>,
    /// Research activity by date
    pub activity_by_date: HashMap<String, usize>, // date -> count
}

/// File-based research history storage
#[derive(Debug, Clone)]
pub struct FileResearchHistoryStorage {
    storage_dir: PathBuf,
}

impl FileResearchHistoryStorage {
    /// Create new file-based storage
    pub fn new<P: AsRef<Path>>(storage_dir: P) -> ApplicationResult<Self> {
        let storage_dir = storage_dir.as_ref().to_path_buf();

        // Create storage directory if it doesn't exist
        if !storage_dir.exists() {
            std::fs::create_dir_all(&storage_dir).map_err(ApplicationError::Io)?;
            info!(
                "Created research history storage directory: {}",
                storage_dir.display()
            );
        }

        Ok(Self { storage_dir })
    }

    /// Get file path for research record
    fn get_record_path(&self, session_id: &str) -> PathBuf {
        self.storage_dir.join(format!("{}.json", session_id))
    }

    /// Get index file path
    fn get_index_path(&self) -> PathBuf {
        self.storage_dir.join("index.json")
    }

    /// Load or create research index
    async fn load_index(&self) -> ApplicationResult<ResearchIndex> {
        let index_path = self.get_index_path();

        if index_path.exists() {
            let content = tokio::fs::read_to_string(&index_path)
                .await
                .map_err(ApplicationError::Io)?;

            serde_json::from_str(&content).map_err(ApplicationError::Serialization)
        } else {
            Ok(ResearchIndex::default())
        }
    }

    /// Save research index
    async fn save_index(&self, index: &ResearchIndex) -> ApplicationResult<()> {
        let index_path = self.get_index_path();
        let content =
            serde_json::to_string_pretty(index).map_err(ApplicationError::Serialization)?;

        tokio::fs::write(&index_path, content)
            .await
            .map_err(ApplicationError::Io)?;

        Ok(())
    }
}

/// Research index for fast lookups
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct ResearchIndex {
    /// Session ID to metadata mapping
    sessions: HashMap<String, ResearchIndexEntry>,
    /// Last updated timestamp
    last_updated: DateTime<Utc>,
}

/// Research index entry
#[derive(Debug, Clone, Serialize, Deserialize)]
struct ResearchIndexEntry {
    /// Session ID
    session_id: String,
    /// Research topic
    topic: String,
    /// Template ID (if any)
    template_id: Option<String>,
    /// Research status
    status: ResearchStatus,
    /// User ID (if any)
    user_id: Option<String>,
    /// Creation timestamp
    created_at: DateTime<Utc>,
    /// Last updated timestamp
    updated_at: DateTime<Utc>,
    /// File size in bytes
    file_size: u64,
}

impl ResearchHistoryStorage for FileResearchHistoryStorage {
    async fn save_record(&self, record: &ResearchHistoryRecord) -> ApplicationResult<()> {
        let record_path = self.get_record_path(&record.session_id);

        // Serialize and save record
        let content =
            serde_json::to_string_pretty(record).map_err(ApplicationError::Serialization)?;

        tokio::fs::write(&record_path, &content)
            .await
            .map_err(ApplicationError::Io)?;

        // Update index
        let mut index = self.load_index().await?;
        let file_size = content.len() as u64;

        index.sessions.insert(
            record.session_id.clone(),
            ResearchIndexEntry {
                session_id: record.session_id.clone(),
                topic: record.topic.clone(),
                template_id: record.template_id.clone(),
                status: record.status.clone(),
                user_id: record.metadata.user_id.clone(),
                created_at: record.created_at,
                updated_at: record.updated_at,
                file_size,
            },
        );

        index.last_updated = Utc::now();
        self.save_index(&index).await?;

        debug!("Saved research record: {}", record.session_id);
        Ok(())
    }

    async fn load_record(
        &self,
        session_id: &str,
    ) -> ApplicationResult<Option<ResearchHistoryRecord>> {
        let record_path = self.get_record_path(session_id);

        if !record_path.exists() {
            return Ok(None);
        }

        let content = tokio::fs::read_to_string(&record_path)
            .await
            .map_err(ApplicationError::Io)?;

        let record: ResearchHistoryRecord =
            serde_json::from_str(&content).map_err(ApplicationError::Serialization)?;

        debug!("Loaded research record: {}", session_id);
        Ok(Some(record))
    }

    async fn update_record(&self, record: &ResearchHistoryRecord) -> ApplicationResult<()> {
        // Same as save_record for file-based storage
        self.save_record(record).await
    }

    async fn list_records(
        &self,
        filters: &ResearchHistoryFilters,
    ) -> ApplicationResult<Vec<ResearchHistoryRecord>> {
        let index = self.load_index().await?;
        let mut matching_entries: Vec<_> = index.sessions.values().collect();

        // Apply filters
        if let Some(ref user_id) = filters.user_id {
            matching_entries.retain(|entry| entry.user_id.as_ref() == Some(user_id));
        }

        if let Some(ref status) = filters.status {
            matching_entries.retain(|entry| &entry.status == status);
        }

        if let Some(ref template_id) = filters.template_id {
            matching_entries.retain(|entry| entry.template_id.as_ref() == Some(template_id));
        }

        if let Some(date_from) = filters.date_from {
            matching_entries.retain(|entry| entry.created_at >= date_from);
        }

        if let Some(date_to) = filters.date_to {
            matching_entries.retain(|entry| entry.created_at <= date_to);
        }

        // Sort by creation date (newest first)
        matching_entries.sort_by(|a, b| b.created_at.cmp(&a.created_at));

        // Apply pagination
        if let Some(offset) = filters.offset {
            if offset < matching_entries.len() {
                matching_entries = matching_entries.into_iter().skip(offset).collect();
            } else {
                matching_entries.clear();
            }
        }

        if let Some(limit) = filters.limit {
            matching_entries.truncate(limit);
        }

        // Load full records
        let mut records = Vec::new();
        for entry in matching_entries {
            if let Ok(Some(record)) = self.load_record(&entry.session_id).await {
                records.push(record);
            }
        }

        Ok(records)
    }

    async fn delete_record(&self, session_id: &str) -> ApplicationResult<()> {
        let record_path = self.get_record_path(session_id);

        if record_path.exists() {
            tokio::fs::remove_file(&record_path)
                .await
                .map_err(ApplicationError::Io)?;
        }

        // Update index
        let mut index = self.load_index().await?;
        index.sessions.remove(session_id);
        index.last_updated = Utc::now();
        self.save_index(&index).await?;

        debug!("Deleted research record: {}", session_id);
        Ok(())
    }

    async fn get_statistics(&self) -> ApplicationResult<ResearchStatistics> {
        let index = self.load_index().await?;

        let total_sessions = index.sessions.len();
        let completed_sessions = index
            .sessions
            .values()
            .filter(|e| e.status == ResearchStatus::Completed)
            .count();
        let in_progress_sessions = index
            .sessions
            .values()
            .filter(|e| e.status == ResearchStatus::InProgress)
            .count();
        let failed_sessions = index
            .sessions
            .values()
            .filter(|e| matches!(e.status, ResearchStatus::Failed(_)))
            .count();

        // Calculate template usage
        let mut template_usage: HashMap<String, usize> = HashMap::new();
        for entry in index.sessions.values() {
            if let Some(ref template_id) = entry.template_id {
                *template_usage.entry(template_id.clone()).or_insert(0) += 1;
            }
        }

        let mut popular_templates: Vec<_> = template_usage.into_iter().collect();
        popular_templates.sort_by(|a, b| b.1.cmp(&a.1));

        // Calculate activity by date
        let mut activity_by_date: HashMap<String, usize> = HashMap::new();
        for entry in index.sessions.values() {
            let date_key = entry.created_at.format("%Y-%m-%d").to_string();
            *activity_by_date.entry(date_key).or_insert(0) += 1;
        }

        Ok(ResearchStatistics {
            total_sessions,
            completed_sessions,
            in_progress_sessions,
            failed_sessions,
            average_duration_seconds: None, // TODO: Calculate from records
            popular_templates,
            activity_by_date,
        })
    }
}
