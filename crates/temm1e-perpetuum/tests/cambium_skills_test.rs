//! Live verification of Phase 3: skill-layer cambium growth.
//!
//! This is the end-to-end proof that the grow_skills() loop works:
//!
//!   1. Synthetic activity is recorded in the Perpetuum store
//!   2. A mock LlmCaller returns a fixed JSON skill suggestion
//!   3. execute_self_work() is invoked with SelfWorkKind::CambiumSkills
//!   4. We verify a cambium-*.md file is written to a controlled tempdir
//!      (via the TEMM1E_CAMBIUM_SKILLS_DIR env var override)
//!   5. We verify the file is parseable by SkillRegistry
//!   6. We verify the rate-limit timestamp is recorded
//!   7. A second invocation within 24h is rate-limited
//!
//! No real LLM is called. No real ~/.temm1e/ files are touched. The test is
//! deterministic and can be re-run any number of times.

use std::sync::Arc;

use async_trait::async_trait;
use temm1e_core::types::error::Temm1eError;
use temm1e_perpetuum::cognitive::LlmCaller;
use temm1e_perpetuum::conscience::SelfWorkKind;
use temm1e_perpetuum::self_work::execute_self_work;
use temm1e_perpetuum::store::Store;
use temm1e_skills::SkillRegistry;
use tempfile::tempdir;
use tokio::sync::Mutex;

// Process-wide async mutex serializes the env-var-using tests inside this
// binary. We use tokio::sync::Mutex (not std::sync::Mutex) because the lock
// is held across .await points; std mutexes would trigger
// clippy::await_holding_lock.
static TEST_MUTEX: Mutex<()> = Mutex::const_new(());

/// Mock LlmCaller that returns a fixed JSON response counting calls.
struct MockCaller {
    response: String,
    call_count: std::sync::Mutex<usize>,
}

impl MockCaller {
    fn new(response: &str) -> Self {
        Self {
            response: response.to_string(),
            call_count: std::sync::Mutex::new(0),
        }
    }

    fn call_count(&self) -> usize {
        *self.call_count.lock().unwrap()
    }
}

#[async_trait]
impl LlmCaller for MockCaller {
    async fn call(&self, _system: Option<&str>, _prompt: &str) -> Result<String, Temm1eError> {
        *self.call_count.lock().unwrap() += 1;
        Ok(self.response.clone())
    }
}

/// Helper: build a temp Store with synthetic activity notes.
async fn store_with_activity(notes: &[&str]) -> Arc<Store> {
    let store = Arc::new(Store::new("sqlite::memory:").await.unwrap());
    for note in notes {
        store
            .save_volition_note(note, "test_activity")
            .await
            .unwrap();
    }
    store
}

/// Helper: clear and set the env override, return guard that resets on drop.
struct EnvGuard {
    key: &'static str,
}
impl EnvGuard {
    fn set(key: &'static str, value: &std::path::Path) -> Self {
        std::env::set_var(key, value);
        Self { key }
    }
}
impl Drop for EnvGuard {
    fn drop(&mut self) {
        std::env::remove_var(self.key);
    }
}

#[tokio::test]
async fn grow_skills_writes_valid_skill_file() {
    let _lock = TEST_MUTEX.lock().await;
    // Setup: tempdir for skills output
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    // Setup: store with 5 activity notes simulating a recurring user need
    let store = store_with_activity(&[
        "user asked how to monitor kubernetes pods",
        "user wanted to check kubernetes deployment status",
        "user asked about kubernetes pod restart counts",
        "user wanted kubernetes namespace listing",
        "user requested kubernetes service health check",
    ])
    .await;

    // Setup: mock LLM returning a valid skill suggestion
    let mock_response = r#"[
        {
            "name": "kubernetes-monitoring",
            "description": "Monitor Kubernetes pods, deployments, and services",
            "capabilities": ["kubernetes", "k8s", "monitoring", "pods"],
            "instructions": "Step 1: Run kubectl get pods.\nStep 2: Check restart counts.\nStep 3: Verify service health."
        }
    ]"#;
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new(mock_response));

    // Run: execute the self-work
    let started = std::time::Instant::now();
    let result = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller)).await;
    let elapsed = started.elapsed();

    // Verify: result is OK and reports a written skill
    assert!(result.is_ok(), "execute_self_work failed: {result:?}");
    let summary = result.unwrap();
    println!("Result: {summary}");
    println!("Elapsed: {elapsed:?}");
    assert!(
        summary.contains("wrote 1 skill"),
        "expected '1 skill' in summary: {summary}"
    );

    // Verify: skill file exists at expected path
    let expected_path = skills_dir.join("cambium-kubernetes-monitoring.md");
    assert!(
        expected_path.exists(),
        "skill file not created at {}",
        expected_path.display()
    );

    // Verify: file content is valid TEMM1E skill format
    let content = std::fs::read_to_string(&expected_path).unwrap();
    println!("---SKILL FILE---\n{content}\n---END---");
    assert!(content.starts_with("---\n"), "missing frontmatter open");
    assert!(
        content.contains("name: kubernetes-monitoring"),
        "missing name"
    );
    assert!(
        content.contains("description: Monitor Kubernetes"),
        "missing description"
    );
    assert!(content.contains("capabilities:"), "missing capabilities");
    assert!(content.contains("- kubernetes"), "missing capability item");
    assert!(content.contains("version: 1.0.0"), "missing version");

    // Verify: SkillRegistry can load the file Tem just wrote.
    // Place the test workspace one level up from the skills dir so the
    // workspace_path/skills/ resolves correctly.
    let mut registry = SkillRegistry::new(tmp.path().to_path_buf());
    registry.load_skills().await.unwrap();
    assert!(
        !registry.list_skills().is_empty(),
        "registry did not load the grown skill"
    );
    let loaded = registry.get_skill("kubernetes-monitoring");
    assert!(loaded.is_some(), "registry could not find grown skill");
    let skill = loaded.unwrap();
    assert_eq!(skill.name, "kubernetes-monitoring");
    assert_eq!(skill.version, "1.0.0");
    assert!(!skill.capabilities.is_empty());
    assert!(skill.instructions.contains("kubectl"));

    // Verify: registry can find skill via capability matching.
    let relevant = registry.find_relevant_skills("how do I monitor my kubernetes pods?");
    assert!(
        !relevant.is_empty(),
        "capability matching failed for grown skill"
    );
    assert_eq!(relevant[0].name, "kubernetes-monitoring");

    println!("OK: skill grown, file valid, registry loaded, capability matched");
}

#[tokio::test]
async fn grow_skills_rate_limits_subsequent_calls() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = store_with_activity(&["first activity", "second activity", "third activity"]).await;

    let mock_response = r#"[]"#; // empty suggestions = no files written but timestamp recorded
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new(mock_response));

    // First call: should run, record timestamp
    let r1 = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller))
        .await
        .unwrap();
    println!("Call 1: {r1}");
    assert!(r1.contains("no skill opportunities") || r1.contains("wrote"));

    // Second call: should be rate-limited
    let r2 = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller))
        .await
        .unwrap();
    println!("Call 2: {r2}");
    assert!(r2.contains("rate limited"), "expected rate limit on call 2");
}

#[tokio::test]
async fn grow_skills_with_no_activity_returns_early() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = Arc::new(Store::new("sqlite::memory:").await.unwrap());
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new("[]"));

    let result = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller))
        .await
        .unwrap();
    println!("Result: {result}");
    assert!(result.contains("no recent activity"));

    // No skill files should exist
    assert!(!skills_dir.exists() || std::fs::read_dir(&skills_dir).unwrap().count() == 0);
}

#[tokio::test]
async fn grow_skills_with_unparseable_llm_response_skips_gracefully() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = store_with_activity(&["some activity"]).await;

    // Garbage LLM response
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new("not json at all 🤷"));

    let result = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller)).await;
    assert!(result.is_ok());
    let summary = result.unwrap();
    println!("Result: {summary}");
    assert!(summary.contains("unparseable"));

    // No files written
    assert!(!skills_dir.exists() || std::fs::read_dir(&skills_dir).unwrap().count() == 0);
}

#[tokio::test]
async fn grow_skills_writes_multiple_skills_in_one_session() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = store_with_activity(&["user wanted X", "user wanted Y", "user wanted Z"]).await;

    let mock_response = r#"[
        {
            "name": "skill-one",
            "description": "First skill",
            "capabilities": ["one"],
            "instructions": "Do thing one."
        },
        {
            "name": "skill-two",
            "description": "Second skill",
            "capabilities": ["two"],
            "instructions": "Do thing two."
        }
    ]"#;
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new(mock_response));

    let result = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller))
        .await
        .unwrap();
    println!("Result: {result}");
    assert!(result.contains("wrote 2 skill"));

    // Both files written to the controlled tempdir
    let entries: Vec<_> = std::fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(
        entries.len(),
        2,
        "expected 2 files in skills_dir, found {}",
        entries.len()
    );
    let names: Vec<String> = entries
        .iter()
        .filter_map(|e| e.file_name().to_str().map(|s| s.to_string()))
        .collect();
    assert!(names.contains(&"cambium-skill-one.md".to_string()));
    assert!(names.contains(&"cambium-skill-two.md".to_string()));
}

#[tokio::test]
async fn grow_skills_sanitizes_dangerous_filenames() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = store_with_activity(&["activity"]).await;

    // LLM returns a name with path-traversal characters
    let mock_response = r#"[
        {
            "name": "../../../etc/passwd",
            "description": "Bad",
            "capabilities": ["bad"],
            "instructions": "evil"
        }
    ]"#;
    let caller: Arc<dyn LlmCaller> = Arc::new(MockCaller::new(mock_response));

    let result = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller))
        .await
        .unwrap();
    println!("Result: {result}");

    // CRITICAL: the path-traversal name must NOT escape the skills_dir.
    // Verify exactly one file was created and it lives inside skills_dir.
    let entries: Vec<_> = std::fs::read_dir(&skills_dir)
        .unwrap()
        .filter_map(|e| e.ok())
        .collect();
    assert_eq!(entries.len(), 1, "expected exactly 1 file in skills_dir");

    let entry = &entries[0];
    let name = entry.file_name().to_string_lossy().to_string();
    println!("Sanitized filename: {name}");

    // The sanitized name must NOT contain / or . or .. segments.
    assert!(!name.contains('/'), "filename contains slash: {name}");
    assert!(!name.contains(".."), "filename contains parent ref: {name}");
    assert!(name.starts_with("cambium-"), "missing prefix: {name}");
    assert!(name.ends_with(".md"), "missing extension: {name}");
    // The dangerous path component should be flattened to dashes.
    assert!(
        name.contains("etc-passwd") || name.contains("etcpasswd"),
        "expected etc and passwd to survive sanitization: {name}"
    );

    // The file must live inside the controlled tempdir, not at /etc/.
    let full = entry.path();
    assert!(
        full.starts_with(&skills_dir),
        "PATH TRAVERSAL: file at {} is not under {}",
        full.display(),
        skills_dir.display()
    );
}

#[tokio::test]
async fn grow_skills_records_call_count() {
    let _lock = TEST_MUTEX.lock().await;
    let tmp = tempdir().unwrap();
    let skills_dir = tmp.path().join("skills");
    let _guard = EnvGuard::set("TEMM1E_CAMBIUM_SKILLS_DIR", &skills_dir);

    let store = store_with_activity(&["a", "b", "c"]).await;
    let caller = Arc::new(MockCaller::new("[]"));
    let caller_dyn: Arc<dyn LlmCaller> = caller.clone();

    let _ = execute_self_work(&SelfWorkKind::CambiumSkills, &store, Some(&caller_dyn))
        .await
        .unwrap();

    // Exactly one LLM call per session.
    assert_eq!(
        caller.call_count(),
        1,
        "expected exactly 1 LLM call per session"
    );
}
