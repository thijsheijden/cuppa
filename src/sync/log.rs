use std::fs::{self, File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use crate::sync::ops::SyncOp;

pub const RECORDS_PER_FILE: usize = 500;

/// Tracks sync operations during a session and exports them to .jsonl files.
/// Files are named `<min_seq>.jsonl` and contain up to 500 records each.
pub struct SyncLog {
    dir: PathBuf,
    /// Operations accumulated during the current session (not yet written to disk).
    session_ops: Vec<(u64, SyncOp)>,
    /// The next sequence number to assign.
    next_seq: u64,
}

impl SyncLog {
    pub fn new(dir: impl AsRef<Path>) -> std::io::Result<Self> {
        let dir = dir.as_ref().to_path_buf();
        fs::create_dir_all(&dir)?;

        let next_seq = Self::find_next_seq(&dir)?;

        Ok(Self {
            dir,
            session_ops: Vec::new(),
            next_seq,
        })
    }

    /// Find the next available sequence number by scanning existing files.
    fn find_next_seq(dir: &Path) -> std::io::Result<u64> {
        let mut max_seq: u64 = 0;

        let entries = match fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(_) => return Ok(0),
        };

        for entry in entries.filter_map(|e| e.ok()) {
            let name = entry.file_name();
            let name = name.to_string_lossy();
            if !name.ends_with(".jsonl") {
                continue;
            }

            let min_seq = match name.trim_end_matches(".jsonl").parse::<u64>() {
                Ok(n) => n,
                Err(_) => continue,
            };

            let count = Self::count_lines(entry.path())?;
            let file_max = min_seq + count as u64;

            if file_max > max_seq {
                max_seq = file_max;
            }
        }

        Ok(max_seq)
    }

    fn count_lines(path: impl AsRef<Path>) -> std::io::Result<usize> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let mut count = 0;
        for line in reader.lines() {
            let line = line?;
            if !line.trim().is_empty() {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Track a new operation during this session. Does not write to disk yet.
    pub fn track(&mut self, op: SyncOp) -> u64 {
        let seq = self.next_seq;
        self.session_ops.push((seq, op));
        self.next_seq += 1;
        seq
    }

    /// Export all session operations to the most recent .jsonl file, creating one if needed.
    /// Files are limited to 500 records. If the current file would exceed the limit,
    /// a new file is started with the appropriate min_seq name.
    pub fn export(&mut self) -> std::io::Result<()> {
        if self.session_ops.is_empty() {
            return Ok(());
        }

        // Find the current file to append to, or determine where to start
        let (mut file_path, mut file_min_seq, mut file_count) = self.current_file()?;

        let mut file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&file_path)?;

        for (seq, op) in &self.session_ops {
            // If current file is full, start a new one
            if file_count >= RECORDS_PER_FILE {
                drop(file);
                file_min_seq = *seq;
                file_path = self.file_path(file_min_seq);
                file = OpenOptions::new()
                    .create(true)
                    .append(true)
                    .open(&file_path)?;
                file_count = 0;
            }

            let json = serde_json::to_string(op).map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::InvalidData, e)
            })?;
            writeln!(file, "{}", json)?;
            file_count += 1;
        }

        file.flush()?;
        self.session_ops.clear();

        Ok(())
    }

    /// Get the path for a log file with the given min_seq.
    fn file_path(&self, min_seq: u64) -> PathBuf {
        self.dir.join(format!("{}.jsonl", min_seq))
    }

    /// Find the current file to append to (the one with the most records that isn't full).
    /// Returns (path, min_seq, current_count).
    fn current_file(&self) -> std::io::Result<(PathBuf, u64, usize)> {
        let mut entries = fs::read_dir(&self.dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.ends_with(".jsonl")
            })
            .collect::<Vec<_>>();

        // Sort by filename so we process in order
        entries.sort_by_key(|e| e.file_name());

        // Check the last file (most recent)
        if let Some(last) = entries.last() {
            let name = last.file_name().to_string_lossy().to_string();
            let min_seq = match name.trim_end_matches(".jsonl").parse::<u64>() {
                Ok(n) => n,
                Err(_) => return self.new_file(0),
            };

            let count = Self::count_lines(last.path())?;
            if count < RECORDS_PER_FILE {
                return Ok((last.path(), min_seq, count));
            }

            // File is full, start a new one
            let next_min_seq = min_seq + count as u64;
            return self.new_file(next_min_seq);
        }

        // No files exist yet
        self.new_file(0)
    }

    fn new_file(&self, min_seq: u64) -> std::io::Result<(PathBuf, u64, usize)> {
        let path = self.file_path(min_seq);
        Ok((path, min_seq, 0))
    }

    /// Read all operations from all log files, starting from `from_seq` (inclusive).
    pub fn read_from(&self, from_seq: u64) -> std::io::Result<Vec<(u64, SyncOp)>> {
        let mut results = Vec::new();

        let mut entries = fs::read_dir(&self.dir)?
            .filter_map(|e| e.ok())
            .filter(|e| {
                let name = e.file_name();
                let name = name.to_string_lossy();
                name.ends_with(".jsonl")
            })
            .collect::<Vec<_>>();

        entries.sort_by_key(|e| e.file_name());

        for entry in entries {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().to_string();
            let min_seq = name.trim_end_matches(".jsonl").parse::<u64>()
                .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;

            let file = File::open(&path)?;
            let reader = BufReader::new(file);

            for (i, line) in reader.lines().enumerate() {
                let line = line?;
                if line.trim().is_empty() {
                    continue;
                }
                let seq = min_seq + i as u64;
                if seq < from_seq {
                    continue;
                }
                let op: SyncOp = serde_json::from_str(&line).map_err(|e| {
                    std::io::Error::new(std::io::ErrorKind::InvalidData, e)
                })?;
                results.push((seq, op));
            }
        }

        Ok(results)
    }

    /// Get the number of operations tracked in the current session (not yet exported).
    pub fn session_count(&self) -> usize {
        self.session_ops.len()
    }

    /// Get the directory path.
    pub fn dir(&self) -> &Path {
        &self.dir
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;
    use std::io::Read;
    use tempfile::TempDir;

    #[test]
    fn test_first_file_named_0_jsonl() {
        let tmp = TempDir::new().unwrap();
        let mut log = SyncLog::new(tmp.path()).unwrap();

        log.track(SyncOp::add_drink("Espresso".to_string(), 63, Utc::now()));
        log.export().unwrap();

        let file_0 = tmp.path().join("0.jsonl");
        assert!(file_0.exists(), "First file should be named 0.jsonl");
    }

    #[test]
    fn test_file_limit_500_records() {
        let tmp = TempDir::new().unwrap();
        let mut log = SyncLog::new(tmp.path()).unwrap();

        // Add 600 records in one session
        for i in 0..600 {
            log.track(SyncOp::add_drink(format!("Drink {}", i), 63, Utc::now()));
        }
        log.export().unwrap();

        // Should have two files: 0.jsonl and 500.jsonl
        let file_0 = tmp.path().join("0.jsonl");
        let file_500 = tmp.path().join("500.jsonl");
        assert!(file_0.exists());
        assert!(file_500.exists());

        // 0.jsonl should have 500 lines
        let count_0 = SyncLog::count_lines(&file_0).unwrap();
        assert_eq!(count_0, 500);

        // 500.jsonl should have 100 lines
        let count_500 = SyncLog::count_lines(&file_500).unwrap();
        assert_eq!(count_500, 100);
    }

    #[test]
    fn test_read_from_skips_lines() {
        let tmp = TempDir::new().unwrap();
        let mut log = SyncLog::new(tmp.path()).unwrap();

        for i in 0..10 {
            log.track(SyncOp::add_drink(format!("Drink {}", i), 63, Utc::now()));
        }
        log.export().unwrap();

        // Read from seq 5 should give us 5 records (seqs 5-9)
        let ops = log.read_from(5).unwrap();
        assert_eq!(ops.len(), 5);
        assert_eq!(ops[0].0, 5);
        assert_eq!(ops[4].0, 9);
    }

    #[test]
    fn test_picks_up_existing_files() {
        let tmp = TempDir::new().unwrap();

        // Pre-create a file with 3 records
        let file_0 = tmp.path().join("0.jsonl");
        let mut f = File::create(&file_0).unwrap();
        for i in 0..3 {
            let op = SyncOp::add_drink(format!("Drink {}", i), 63, Utc::now());
            writeln!(f, "{}", serde_json::to_string(&op).unwrap()).unwrap();
        }
        drop(f);

        // New SyncLog should start at seq 3
        let mut log = SyncLog::new(tmp.path()).unwrap();
        log.track(SyncOp::add_drink("New Drink".to_string(), 63, Utc::now()));
        log.export().unwrap();

        let ops = log.read_from(0).unwrap();
        assert_eq!(ops.len(), 4);
        assert_eq!(ops[3].0, 3);
    }

    #[test]
    fn test_multiple_exports_append_to_same_file() {
        let tmp = TempDir::new().unwrap();
        let mut log = SyncLog::new(tmp.path()).unwrap();

        // First export: 10 records
        for i in 0..10 {
            log.track(SyncOp::add_drink(format!("Drink {}", i), 63, Utc::now()));
        }
        log.export().unwrap();

        // Second export: 10 more records
        for i in 10..20 {
            log.track(SyncOp::add_drink(format!("Drink {}", i), 63, Utc::now()));
        }
        log.export().unwrap();

        let ops = log.read_from(0).unwrap();
        assert_eq!(ops.len(), 20);
        assert_eq!(ops[0].0, 0);
        assert_eq!(ops[19].0, 19);
    }
}
