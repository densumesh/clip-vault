use clip_vault_core::{ClipboardItem, SqliteVault, Vault};
use sha2::{Digest, Sha256};
use std::path::PathBuf;
use tempfile::TempDir;

/// Helper to create a temporary vault for testing
fn create_test_vault() -> (TempDir, SqliteVault) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let db_path = temp_dir.path().join("test.db");
    let vault = SqliteVault::open(&db_path, "test_password").expect("Failed to create vault");
    (temp_dir, vault)
}

/// Helper to hash content for duplicate detection
fn hash_content(content: &str) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(content.as_bytes());
    hasher.finalize().into()
}

#[cfg(test)]
mod vault_tests {
    use super::*;

    #[test]
    fn test_empty_vault() {
        let (_temp_dir, vault) = create_test_vault();

        assert_eq!(vault.len().unwrap(), 0);
        assert!(vault.is_empty().unwrap());
        assert!(vault.latest().unwrap().is_none());
        assert!(vault.list(None).unwrap().is_empty());
        assert!(vault.list(Some(5)).unwrap().is_empty());
    }

    #[test]
    fn test_insert_and_retrieve_single_item() {
        let (_temp_dir, vault) = create_test_vault();
        let content = "Hello, world!";
        let item = ClipboardItem::Text(content.to_string());
        let hash = hash_content(content);

        vault.insert(hash, &item).unwrap();

        assert_eq!(vault.len().unwrap(), 1);
        assert!(!vault.is_empty().unwrap());

        let latest = vault.latest().unwrap().unwrap();
        assert_eq!(latest, item);

        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 1);
        assert_eq!(all_items[0].item, item);
    }

    #[test]
    fn test_insert_duplicate_items() {
        let (_temp_dir, vault) = create_test_vault();
        let content = "Duplicate content";
        let item = ClipboardItem::Text(content.to_string());
        let hash = hash_content(content);

        // Insert same item twice
        vault.insert(hash, &item).unwrap();
        vault.insert(hash, &item).unwrap();

        // Should still only have one item due to PRIMARY KEY constraint
        assert_eq!(vault.len().unwrap(), 1);
        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 1);
        assert_eq!(all_items[0].item, item);
    }

    #[test]
    fn test_multiple_items_ordering() {
        let (_temp_dir, vault) = create_test_vault();

        let items = vec![
            ("First item", ClipboardItem::Text("First item".to_string())),
            (
                "Second item",
                ClipboardItem::Text("Second item".to_string()),
            ),
            ("Third item", ClipboardItem::Text("Third item".to_string())),
        ];

        // Insert items in order
        for (content, item) in &items {
            let hash = hash_content(content);
            vault.insert(hash, item).unwrap();
            // Small delay to ensure different timestamps
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(vault.len().unwrap(), 3);

        // Latest should be the most recent (third item)
        let latest = vault.latest().unwrap().unwrap();
        assert_eq!(latest, items[2].1);

        // List all should return in reverse chronological order (newest first)
        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 3);
        assert_eq!(all_items[0].item, items[2].1); // Third (newest)
        assert_eq!(all_items[1].item, items[1].1); // Second
        assert_eq!(all_items[2].item, items[0].1); // First (oldest)
    }

    #[test]
    fn test_list_with_limit() {
        let (_temp_dir, vault) = create_test_vault();

        // Insert 5 items
        for i in 1..=5 {
            let content = format!("Item {}", i);
            let item = ClipboardItem::Text(content.clone());
            let hash = hash_content(&content);
            vault.insert(hash, &item).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        assert_eq!(vault.len().unwrap(), 5);

        // Test various limits
        let limit_0 = vault.list(Some(0)).unwrap();
        assert_eq!(limit_0.len(), 0);

        let limit_2 = vault.list(Some(2)).unwrap();
        assert_eq!(limit_2.len(), 2);
        assert_eq!(limit_2[0].item, ClipboardItem::Text("Item 5".to_string())); // Most recent
        assert_eq!(limit_2[1].item, ClipboardItem::Text("Item 4".to_string()));

        let limit_10 = vault.list(Some(10)).unwrap(); // More than available
        assert_eq!(limit_10.len(), 5); // Should return all 5

        let no_limit = vault.list(None).unwrap();
        assert_eq!(no_limit.len(), 5);
        // Compare the items, not the full structs with timestamps
        for (i, item) in limit_10.iter().enumerate() {
            assert_eq!(item.item, no_limit[i].item);
        }
    }

    #[test]
    fn test_persistence_across_connections() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("persistence_test.db");
        let password = "test_password";

        // Create first connection and insert data
        {
            let vault1 = SqliteVault::open(&db_path, password).unwrap();
            let item = ClipboardItem::Text("Persistent data".to_string());
            let hash = hash_content("Persistent data");
            vault1.insert(hash, &item).unwrap();
            assert_eq!(vault1.len().unwrap(), 1);
        } // vault1 goes out of scope, connection closed

        // Create second connection and verify data persists
        {
            let vault2 = SqliteVault::open(&db_path, password).unwrap();
            assert_eq!(vault2.len().unwrap(), 1);
            let latest = vault2.latest().unwrap().unwrap();
            assert_eq!(latest, ClipboardItem::Text("Persistent data".to_string()));
        }
    }

    #[test]
    fn test_wrong_password_fails() {
        let temp_dir = TempDir::new().expect("Failed to create temp directory");
        let db_path = temp_dir.path().join("password_test.db");
        let correct_password = "correct_password";
        let wrong_password = "wrong_password";

        // Create vault with correct password
        {
            let _vault = SqliteVault::open(&db_path, correct_password).unwrap();
        }

        // Try to open with wrong password should fail
        let result = SqliteVault::open(&db_path, wrong_password);
        assert!(result.is_err());
    }
}

#[cfg(test)]
mod search_tests {
    use super::*;

    #[test]
    fn test_search_empty_vault() {
        let (_temp_dir, vault) = create_test_vault();

        let results = vault.search("anything", None).unwrap();
        assert!(results.is_empty());

        let results = vault.search("anything", Some(5)).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_exact_match() {
        let (_temp_dir, vault) = create_test_vault();

        let content = "Hello, world!";
        let item = ClipboardItem::Text(content.to_string());
        let hash = hash_content(content);
        vault.insert(hash, &item).unwrap();

        // Exact match
        let results = vault.search("Hello, world!", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item, item);

        // Partial match
        let results = vault.search("Hello", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item, item);

        // Case insensitive - should match (FTS5 is case insensitive)
        let results = vault.search("hello", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].item, item);
    }

    #[test]
    fn test_search_multiple_matches() {
        let (_temp_dir, vault) = create_test_vault();

        let items = [
            "Hello world",
            "world peace",
            "Another world entry",
            "Different content",
            "world of programming",
        ];

        // Insert items with small delays to ensure ordering
        for content in &items {
            let item = ClipboardItem::Text(content.to_string());
            let hash = hash_content(content);
            vault.insert(hash, &item).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Search for "world" - should find 4 matches in reverse chronological order
        let results = vault.search("world", None).unwrap();
        assert_eq!(results.len(), 4);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text("world of programming".to_string())
        ); // Most recent
        assert_eq!(
            results[1].item,
            ClipboardItem::Text("Another world entry".to_string())
        );
        assert_eq!(
            results[2].item,
            ClipboardItem::Text("world peace".to_string())
        );
        assert_eq!(
            results[3].item,
            ClipboardItem::Text("Hello world".to_string())
        ); // Oldest

        // Search for non-existent pattern
        let results = vault.search("nonexistent", None).unwrap();
        assert!(results.is_empty());
    }

    #[test]
    fn test_search_with_limit() {
        let (_temp_dir, vault) = create_test_vault();

        // Insert 5 items containing "test"
        for i in 1..=5 {
            let content = format!("test item {}", i);
            let item = ClipboardItem::Text(content.clone());
            let hash = hash_content(&content);
            vault.insert(hash, &item).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Search with limit 0
        let results = vault.search("test", Some(0)).unwrap();
        assert_eq!(results.len(), 0);

        // Search with limit 2
        let results = vault.search("test", Some(2)).unwrap();
        assert_eq!(results.len(), 2);
        // Results are in relevance order, just check we got the right count
        for result in &results {
            if let ClipboardItem::Text(text) = &result.item {
                assert!(text.contains("test"));
            }
        }

        // Search with limit larger than matches
        let results = vault.search("test", Some(10)).unwrap();
        assert_eq!(results.len(), 5); // All matches

        // Search without limit
        let results_no_limit = vault.search("test", None).unwrap();
        assert_eq!(results_no_limit.len(), 5);
        // All results should contain "test"
        for result in &results_no_limit {
            if let ClipboardItem::Text(text) = &result.item {
                assert!(text.contains("test"));
            }
        }
    }

    #[test]
    fn test_search_special_characters() {
        let (_temp_dir, vault) = create_test_vault();

        let special_content = [
            "https://example.com/path?query=value&other=123",
            "Email: user@domain.com",
            "Code: fn main() { println!(\"Hello!\"); }",
            "SQL: SELECT * FROM table WHERE id = 1;",
            "Math: (a + b) * c = d",
        ];

        for content in &special_content {
            let item = ClipboardItem::Text(content.to_string());
            let hash = hash_content(content);
            vault.insert(hash, &item).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Search for URL components
        let results = vault.search("https://", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text(special_content[0].to_string())
        );

        // Search for email
        let results = vault.search("@domain.com", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text(special_content[1].to_string())
        );

        // Search for code patterns
        let results = vault.search("fn main()", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text(special_content[2].to_string())
        );

        // Search for SQL
        let results = vault.search("SELECT", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text(special_content[3].to_string())
        );

        // Search for parentheses
        let results = vault.search("(a + b)", None).unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(
            results[0].item,
            ClipboardItem::Text(special_content[4].to_string())
        );
    }

    #[test]
    fn test_search_ordering_with_duplicates() {
        let (_temp_dir, vault) = create_test_vault();

        // Insert items where some have same content but different timestamps
        let content1 = "common search term first";
        let content2 = "different content";
        let content3 = "common search term second";

        let item1 = ClipboardItem::Text(content1.to_string());
        let item2 = ClipboardItem::Text(content2.to_string());
        let item3 = ClipboardItem::Text(content3.to_string());

        vault.insert(hash_content(content1), &item1).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        vault.insert(hash_content(content2), &item2).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(10));
        vault.insert(hash_content(content3), &item3).unwrap();

        // Search should return in reverse chronological order
        let results = vault.search("common search term", None).unwrap();
        assert_eq!(results.len(), 2);
        assert_eq!(results[0].item, item3); // Most recent
        assert_eq!(results[1].item, item1); // Oldest matching
    }

    #[test]
    fn test_search_large_dataset() {
        let (_temp_dir, vault) = create_test_vault();

        // Insert 50 items, every 5th contains "special"
        for i in 1..=50 {
            let content = if i % 5 == 0 {
                format!("special item number {}", i)
            } else {
                format!("regular item number {}", i)
            };
            let item = ClipboardItem::Text(content.clone());
            let hash = hash_content(&content);
            vault.insert(hash, &item).unwrap();
        }

        // Search for "special" items
        let results = vault.search("special", None).unwrap();
        assert_eq!(results.len(), 10); // Items 5, 10, 15, ..., 50

        // Check that all results contain "special"
        for result in &results {
            if let ClipboardItem::Text(text) = &result.item {
                assert!(text.contains("special"));
            }
        }

        // Search with limit
        let limited_results = vault.search("special", Some(3)).unwrap();
        assert_eq!(limited_results.len(), 3);
        // Check that all limited results contain "special"
        for result in &limited_results {
            if let ClipboardItem::Text(text) = &result.item {
                assert!(text.contains("special"));
            }
        }
    }
}

#[cfg(test)]
mod integration_tests {
    use super::*;

    #[test]
    fn test_full_clipboard_workflow() {
        let (_temp_dir, vault) = create_test_vault();

        // Simulate a clipboard workflow
        let clipboard_history = [
            "First copied text",
            "https://example.com/link",
            "Some code snippet: fn main() {}",
            "Another piece of text",
            "Final clipboard entry",
        ];

        // Insert clipboard items over time
        for (i, content) in clipboard_history.iter().enumerate() {
            let item = ClipboardItem::Text(content.to_string());
            let hash = hash_content(content);
            vault.insert(hash, &item).unwrap();

            // Verify running state
            assert_eq!(vault.len().unwrap(), i + 1);
            let latest = vault.latest().unwrap().unwrap();
            assert_eq!(latest, ClipboardItem::Text(content.to_string()));

            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Test final state
        assert_eq!(vault.len().unwrap(), 5);

        // Test latest
        let latest = vault.latest().unwrap().unwrap();
        assert_eq!(
            latest,
            ClipboardItem::Text("Final clipboard entry".to_string())
        );

        // Test list all (should be in reverse order)
        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 5);
        assert_eq!(
            all_items[0].item,
            ClipboardItem::Text("Final clipboard entry".to_string())
        );
        assert_eq!(
            all_items[4].item,
            ClipboardItem::Text("First copied text".to_string())
        );

        // Test list with limits
        let last_3 = vault.list(Some(3)).unwrap();
        assert_eq!(last_3.len(), 3);
        assert_eq!(
            last_3[0].item,
            ClipboardItem::Text("Final clipboard entry".to_string())
        );
        assert_eq!(
            last_3[1].item,
            ClipboardItem::Text("Another piece of text".to_string())
        );
        assert_eq!(
            last_3[2].item,
            ClipboardItem::Text("Some code snippet: fn main() {}".to_string())
        );
    }

    #[test]
    fn test_duplicate_detection_workflow() {
        let (_temp_dir, vault) = create_test_vault();

        let content = "Repeated clipboard content";
        let item = ClipboardItem::Text(content.to_string());
        let hash = hash_content(content);

        // Insert the same content multiple times (simulating user copying same thing)
        for _ in 0..5 {
            vault.insert(hash, &item).unwrap();
            std::thread::sleep(std::time::Duration::from_millis(10));
        }

        // Should only have one entry due to duplicate detection
        assert_eq!(vault.len().unwrap(), 1);
        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 1);
        assert_eq!(all_items[0].item, item);
    }

    #[test]
    fn test_large_clipboard_history() {
        let (_temp_dir, vault) = create_test_vault();

        // Insert 100 items
        for i in 1..=100 {
            let content = format!("Clipboard item number {}", i);
            let item = ClipboardItem::Text(content.clone());
            let hash = hash_content(&content);
            vault.insert(hash, &item).unwrap();
        }

        assert_eq!(vault.len().unwrap(), 100);

        // Test various list operations on large dataset
        let latest = vault.latest().unwrap().unwrap();
        assert_eq!(
            latest,
            ClipboardItem::Text("Clipboard item number 100".to_string())
        );

        let last_10 = vault.list(Some(10)).unwrap();
        assert_eq!(last_10.len(), 10);
        assert_eq!(
            last_10[0].item,
            ClipboardItem::Text("Clipboard item number 100".to_string())
        );
        assert_eq!(
            last_10[9].item,
            ClipboardItem::Text("Clipboard item number 91".to_string())
        );

        let all_items = vault.list(None).unwrap();
        assert_eq!(all_items.len(), 100);
        assert_eq!(
            all_items[0].item,
            ClipboardItem::Text("Clipboard item number 100".to_string())
        );
        assert_eq!(
            all_items[99].item,
            ClipboardItem::Text("Clipboard item number 1".to_string())
        );
    }
}

#[cfg(test)]
mod error_handling_tests {
    use super::*;

    #[test]
    fn test_invalid_database_path() {
        // Try to create vault in non-existent directory without creating parent dirs
        let invalid_path = PathBuf::from("/non/existent/directory/test.db");
        let result = SqliteVault::open(&invalid_path, "test_password");
        assert!(result.is_err());
    }

    #[test]
    fn test_concurrent_access() {
        let temp_dir = TempDir::new().unwrap();
        let db_path = temp_dir.path().join("concurrent_test.db");
        let password = "test_password";

        // Create initial vault and keep it open
        let _vault1 = SqliteVault::open(&db_path, password).unwrap();

        // Try to open second connection - should work (SQLite supports multiple readers)
        let vault2 = SqliteVault::open(&db_path, password);
        assert!(vault2.is_ok());
    }
}
