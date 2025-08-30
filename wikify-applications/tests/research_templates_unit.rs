//! Unit tests for research template system

use std::collections::HashMap;
use tempfile::TempDir;
use wikify_applications::{
    auth::permissions::{PermissionMode, ResourceLimits},
    research::{
        types::{ResearchConfig, ResearchContext},
        FileResearchHistoryStorage, ResearchHistoryFilters, ResearchHistoryRecord,
        ResearchHistoryStorage, ResearchMetadata, ResearchStatus, ResearchTemplateManager,
    },
    Permission, PermissionContext, ResearchCategory, UserIdentity,
};

/// Create test permission context
fn create_test_context(user_id: &str, permissions: Vec<Permission>) -> PermissionContext {
    let identity = UserIdentity::registered(
        user_id.to_string(),
        Some(format!("User {}", user_id)),
        Some(format!("{}@test.com", user_id)),
    );

    PermissionContext::new(
        Some(identity),
        PermissionMode::Restricted,
        permissions.into_iter().collect(),
        ResourceLimits::default(),
    )
}

/// Create admin permission context
fn create_admin_context() -> PermissionContext {
    create_test_context("admin", vec![Permission::Admin, Permission::Query])
}

/// Create regular user permission context
fn create_user_context(user_id: &str) -> PermissionContext {
    create_test_context(user_id, vec![Permission::Query])
}

#[tokio::test]
async fn test_research_template_manager() {
    let manager = ResearchTemplateManager::new();

    // Test listing all templates
    let templates = manager.list_templates();
    assert!(!templates.is_empty());

    // Check for expected built-in templates
    let template_ids: Vec<&str> = templates.iter().map(|t| t.id.as_str()).collect();
    assert!(template_ids.contains(&"technical-analysis"));
    assert!(template_ids.contains(&"architecture-assessment"));
    assert!(template_ids.contains(&"security-analysis"));
    assert!(template_ids.contains(&"documentation-extraction"));

    println!("✅ Found {} built-in templates", templates.len());
}

#[tokio::test]
async fn test_get_specific_template() {
    let manager = ResearchTemplateManager::new();

    // Test getting existing template
    let template = manager.get_template("technical-analysis");
    assert!(template.is_some());

    let template = template.unwrap();
    assert_eq!(template.id, "technical-analysis");
    assert_eq!(template.name, "Technical Analysis");
    assert_eq!(template.category, ResearchCategory::Technical);
    assert!(!template.initial_questions.is_empty());

    // Test getting non-existent template
    let template = manager.get_template("nonexistent");
    assert!(template.is_none());

    println!("✅ Template retrieval working correctly");
}

#[tokio::test]
async fn test_templates_by_category() {
    let manager = ResearchTemplateManager::new();

    // Test technical category
    let technical_templates = manager.list_templates_by_category(&ResearchCategory::Technical);
    assert!(!technical_templates.is_empty());

    for template in &technical_templates {
        assert_eq!(template.category, ResearchCategory::Technical);
    }

    // Test security category
    let security_templates = manager.list_templates_by_category(&ResearchCategory::Security);
    assert!(!security_templates.is_empty());

    for template in &security_templates {
        assert_eq!(template.category, ResearchCategory::Security);
    }

    // Test empty category
    let custom_templates = manager.list_templates_by_category(&ResearchCategory::Custom);
    assert!(custom_templates.is_empty()); // No custom templates by default

    println!("✅ Template category filtering working correctly");
}

#[tokio::test]
async fn test_file_research_history_storage() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileResearchHistoryStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Create test context
    let context = ResearchContext {
        session_id: "test-session-001".to_string(),
        topic: "Test Research Topic".to_string(),
        config: ResearchConfig::default(),
        current_iteration: 0,
        questions: Vec::new(),
        findings: Vec::new(),
        metadata: HashMap::new(),
        iterations: vec![],
        start_time: chrono::Utc::now(),
    };

    // Create test record
    let now = chrono::Utc::now();
    let record = ResearchHistoryRecord {
        session_id: "test-session-001".to_string(),
        template_id: Some("technical-analysis".to_string()),
        topic: "Test Research Topic".to_string(),
        status: ResearchStatus::InProgress,
        context,
        iterations: vec![],
        summary: None,
        created_at: now,
        updated_at: now,
        completed_at: None,
        metadata: ResearchMetadata {
            total_iterations: 0,
            total_questions: 0,
            total_sources: 0,
            duration_seconds: None,
            user_id: Some("user123".to_string()),
            repository_context: None,
        },
    };

    // Test saving record
    storage.save_record(&record).await.unwrap();

    // Test loading record
    let loaded_record = storage.load_record("test-session-001").await.unwrap();
    assert!(loaded_record.is_some());

    let loaded_record = loaded_record.unwrap();
    assert_eq!(loaded_record.session_id, "test-session-001");
    assert_eq!(
        loaded_record.template_id,
        Some("technical-analysis".to_string())
    );
    assert_eq!(loaded_record.topic, "Test Research Topic");

    // Test updating record
    let mut updated_record = loaded_record.clone();
    updated_record.status = ResearchStatus::Completed;
    updated_record.completed_at = Some(chrono::Utc::now());
    storage.update_record(&updated_record).await.unwrap();

    let loaded_record = storage
        .load_record("test-session-001")
        .await
        .unwrap()
        .unwrap();
    assert_eq!(loaded_record.status, ResearchStatus::Completed);

    // Test deleting record
    storage.delete_record("test-session-001").await.unwrap();
    let loaded_record = storage.load_record("test-session-001").await.unwrap();
    assert!(loaded_record.is_none());

    println!("✅ File research history storage working correctly");
}

#[tokio::test]
async fn test_research_history_filtering() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileResearchHistoryStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Helper function to create test context
    let create_test_context = |session_id: &str, topic: &str| ResearchContext {
        session_id: session_id.to_string(),
        topic: topic.to_string(),
        config: ResearchConfig::default(),
        current_iteration: 0,
        questions: Vec::new(),
        findings: Vec::new(),
        metadata: HashMap::new(),
        iterations: vec![],
        start_time: chrono::Utc::now(),
    };

    let now = chrono::Utc::now();

    // Create multiple test records
    let records = vec![
        ResearchHistoryRecord {
            session_id: "filter-test-1".to_string(),
            template_id: Some("technical-analysis".to_string()),
            topic: "Technical Test 1".to_string(),
            status: ResearchStatus::Completed,
            context: create_test_context("filter-test-1", "Technical Test 1"),
            iterations: vec![],
            summary: None,
            created_at: now - chrono::Duration::hours(2),
            updated_at: now - chrono::Duration::hours(1),
            completed_at: Some(now - chrono::Duration::hours(1)),
            metadata: ResearchMetadata {
                total_iterations: 1,
                total_questions: 3,
                total_sources: 5,
                duration_seconds: Some(3600),
                user_id: Some("user1".to_string()),
                repository_context: None,
            },
        },
        ResearchHistoryRecord {
            session_id: "filter-test-2".to_string(),
            template_id: Some("security-analysis".to_string()),
            topic: "Security Test 1".to_string(),
            status: ResearchStatus::InProgress,
            context: create_test_context("filter-test-2", "Security Test 1"),
            iterations: vec![],
            summary: None,
            created_at: now - chrono::Duration::hours(1),
            updated_at: now,
            completed_at: None,
            metadata: ResearchMetadata {
                total_iterations: 0,
                total_questions: 2,
                total_sources: 3,
                duration_seconds: None,
                user_id: Some("user2".to_string()),
                repository_context: None,
            },
        },
        ResearchHistoryRecord {
            session_id: "filter-test-3".to_string(),
            template_id: Some("technical-analysis".to_string()),
            topic: "Technical Test 2".to_string(),
            status: ResearchStatus::InProgress,
            context: create_test_context("filter-test-3", "Technical Test 2"),
            iterations: vec![],
            summary: None,
            created_at: now,
            updated_at: now,
            completed_at: None,
            metadata: ResearchMetadata {
                total_iterations: 0,
                total_questions: 1,
                total_sources: 2,
                duration_seconds: None,
                user_id: Some("user1".to_string()),
                repository_context: None,
            },
        },
    ];

    // Save all records
    for record in &records {
        storage.save_record(record).await.unwrap();
    }

    // Test filtering by template_id
    let mut filters = ResearchHistoryFilters::default();
    filters.template_id = Some("technical-analysis".to_string());

    let filtered_records = storage.list_records(&filters).await.unwrap();
    assert_eq!(filtered_records.len(), 2);

    // Test filtering by status
    let mut filters = ResearchHistoryFilters::default();
    filters.status = Some(ResearchStatus::Completed);

    let filtered_records = storage.list_records(&filters).await.unwrap();
    assert_eq!(filtered_records.len(), 1);
    assert_eq!(filtered_records[0].status, ResearchStatus::Completed);

    // Test filtering by user_id
    let mut filters = ResearchHistoryFilters::default();
    filters.user_id = Some("user1".to_string());

    let filtered_records = storage.list_records(&filters).await.unwrap();
    assert_eq!(filtered_records.len(), 2);

    // Test limit
    let mut filters = ResearchHistoryFilters::default();
    filters.limit = Some(1);

    let filtered_records = storage.list_records(&filters).await.unwrap();
    assert_eq!(filtered_records.len(), 1);

    println!("✅ Research history filtering working correctly");
}

#[tokio::test]
async fn test_research_statistics() {
    let temp_dir = TempDir::new().unwrap();
    let storage = FileResearchHistoryStorage::new(temp_dir.path().to_path_buf()).unwrap();

    // Helper function to create test context
    let create_test_context = |session_id: &str| ResearchContext {
        session_id: session_id.to_string(),
        topic: "Statistics Test".to_string(),
        config: ResearchConfig::default(),
        current_iteration: 0,
        questions: Vec::new(),
        findings: Vec::new(),
        metadata: HashMap::new(),
        iterations: vec![],
        start_time: chrono::Utc::now(),
    };

    // Create test records with different statuses
    let records = vec![
        ("stats-test-1", ResearchStatus::Completed),
        ("stats-test-2", ResearchStatus::Completed),
        ("stats-test-3", ResearchStatus::InProgress),
        (
            "stats-test-4",
            ResearchStatus::Failed("Test error".to_string()),
        ),
    ];

    let now = chrono::Utc::now();

    for (session_id, status) in &records {
        let record = ResearchHistoryRecord {
            session_id: session_id.to_string(),
            template_id: Some("technical-analysis".to_string()),
            topic: "Statistics Test".to_string(),
            status: status.clone(),
            context: create_test_context(session_id),
            iterations: vec![],
            summary: None,
            created_at: now,
            updated_at: now,
            completed_at: if *status == ResearchStatus::Completed {
                Some(now)
            } else {
                None
            },
            metadata: ResearchMetadata {
                total_iterations: 1,
                total_questions: 2,
                total_sources: 3,
                duration_seconds: Some(1800),
                user_id: Some("stats_user".to_string()),
                repository_context: None,
            },
        };
        storage.save_record(&record).await.unwrap();
    }

    // Get statistics
    let stats = storage.get_statistics().await.unwrap();

    assert_eq!(stats.total_sessions, 4);
    assert_eq!(stats.completed_sessions, 2);
    assert_eq!(stats.in_progress_sessions, 1);
    assert_eq!(stats.failed_sessions, 1); // cancelled is treated as failed

    println!("✅ Research statistics calculation working correctly");
}
