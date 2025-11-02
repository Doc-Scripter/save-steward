use crate::detection::DetectionError;
use sha2::{Sha256, Digest};
use std::fs;
use std::path::Path;
use tokio::task;
use chrono::{DateTime, Utc};

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ExecutableSignature {
    pub file_hash: String,           // SHA-256 hash
    pub file_size: u64,
    pub product_name: String,
    pub company_name: String,
    pub file_version: String,
    pub product_version: String,
    pub original_filename: String,
    pub digital_signature: Option<String>,
    pub analyzed_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct ExecutableAnalyzer {
    cache: std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, ExecutableSignature>>>,
}

impl ExecutableAnalyzer {
    pub fn new() -> Self {
        Self {
            cache: std::sync::Arc::new(tokio::sync::RwLock::new(std::collections::HashMap::new())),
        }
    }

    pub async fn analyze_executable(&self, file_path: &str) -> Result<ExecutableSignature, DetectionError> {
        // Check cache first
        {
            let cache = self.cache.read().await;
            if let Some(signature) = cache.get(file_path) {
                return Ok(signature.clone());
            }
        }

        // Perform analysis
        let signature = self.analyze_executable_impl(file_path).await?;

        // Cache the result
        {
            let mut cache = self.cache.write().await;
            cache.insert(file_path.to_string(), signature.clone());
        }

        Ok(signature)
    }

    async fn analyze_executable_impl(&self, file_path: &str) -> Result<ExecutableSignature, DetectionError> {
        let path = Path::new(file_path);

        if !path.exists() {
            return Err(DetectionError::ExecutableAnalysisError(format!("File not found: {}", file_path)));
        }

        if !path.is_file() {
            return Err(DetectionError::ExecutableAnalysisError(format!("Path is not a file: {}", file_path)));
        }

        // Get file size
        let metadata = fs::metadata(path)?;
        let file_size = metadata.len();

        // Calculate SHA-256 hash
        let file_hash = self.calculate_sha256(file_path).await?;

        // Analyze file metadata (version info, etc.)
        let (product_name, company_name, file_version, product_version, original_filename, digital_signature) =
            self.extract_metadata(file_path).await?;

        Ok(ExecutableSignature {
            file_hash,
            file_size,
            product_name,
            company_name,
            file_version,
            product_version,
            original_filename,
            digital_signature,
            analyzed_at: Utc::now(),
        })
    }

    async fn calculate_sha256(&self, file_path: &str) -> Result<String, DetectionError> {
        let file_path = file_path.to_string();
        task::spawn_blocking(move || {
            let mut file = fs::File::open(&file_path)?;
            let mut hasher = Sha256::new();
            std::io::copy(&mut file, &mut hasher)?;
            let hash = hasher.finalize();
            Ok(hex::encode(hash))
        }).await.map_err(|e| DetectionError::ExecutableAnalysisError(format!("Hash calculation failed: {}", e)))?
    }

    async fn extract_metadata(&self, file_path: &str) -> Result<(String, String, String, String, String, Option<String>), DetectionError> {
        let path_clone = file_path.to_string();

        task::spawn_blocking(move || -> Result<_, DetectionError> {
            Self::extract_metadata_sync(&path_clone)
        }).await.unwrap_or_else(|_| Ok((
            "Unknown".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
            "Unknown".to_string(),
            None,
        )))
    }

    fn extract_metadata_sync(file_path: &str) -> Result<(String, String, String, String, String, Option<String>), DetectionError> {
        #[cfg(target_os = "windows")]
        {
            use winapi::um::winver::{GetFileVersionInfoA, GetFileVersionInfoSizeA, VerQueryValueA};
            use winapi::um::sysinfoapi::GetSystemDirectoryA;
            use std::ffi::{CStr, CString};
            use std::ptr;

            let file_path_cstr = CString::new(file_path)?;

            unsafe {
                // Get version info size
                let size = GetFileVersionInfoSizeA(file_path_cstr.as_ptr(), ptr::null_mut());
                if size == 0 {
                    return Ok((
                        "Unknown (No version info)".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        None,
                    ));
                }

                // Allocate buffer and get version info
                let mut buffer: Vec<u8> = vec![0; size as usize];
                let mut handle: u32 = 0;

                if GetFileVersionInfoA(file_path_cstr.as_ptr(), 0, size, buffer.as_mut_ptr() as *mut _) == 0 {
                    return Ok((
                        "Unknown (Failed to get version info)".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        "Unknown".to_string(),
                        None,
                    ));
                }

                let mut product_name = "Unknown".to_string();
                let mut company_name = "Unknown".to_string();
                let mut file_version = "Unknown".to_string();
                let mut product_version = "Unknown".to_string();
                let mut original_filename = Path::new(file_path)
                    .file_name()
                    .map(|n| n.to_string_lossy().to_string())
                    .unwrap_or_else(|| "Unknown".to_string());

                // Extract strings using VerQueryValue
                let queries = [
                    ("\\StringFileInfo\\040904b0\\ProductName", &mut product_name),
                    ("\\StringFileInfo\\040904b0\\CompanyName", &mut company_name),
                    ("\\StringFileInfo\\040904b0\\FileVersion", &mut file_version),
                    ("\\StringFileInfo\\040904b0\\ProductVersion", &mut product_version),
                    ("\\StringFileInfo\\040904b0\\OriginalFilename", &mut original_filename),
                ];

                for (query, target) in queries {
                    let query_cstr = CString::new(query)?;
                    let mut value_ptr: *mut std::ffi::c_void = ptr::null_mut();

                    if VerQueryValueA(buffer.as_ptr() as *const _, query_cstr.as_ptr(), &mut value_ptr, ptr::null_mut()) != 0
                        && !value_ptr.is_null() {
                        let value = CStr::from_ptr(value_ptr as *const i8);
                        **target = value.to_string_lossy().to_string();
                    }
                }

                // TODO: Extract digital signature information
                // This would require additional WinAPI calls for Authenticode verification

                Ok((product_name, company_name, file_version, product_version, original_filename, None))
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS implementation using plist parsing or similar
            // This would use macOS specific APIs to read bundle metadata
            let file_name = Path::new(file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Ok((
                "Unknown (macOS info not extracted)".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                file_name,
                None,
            ))
        }

        #[cfg(target_os = "linux")]
        {
            // Linux implementation using file(1) command or libmagic
            // This is a simplified implementation
            let file_name = Path::new(file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Ok((
                "Unknown (Linux info not extracted)".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                file_name,
                None,
            ))
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            let file_name = Path::new(file_path)
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "Unknown".to_string());

            Ok((
                "Unknown (Platform not supported)".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                "Unknown".to_string(),
                file_name,
                None,
            ))
        }
    }

    pub async fn compare_signatures(&self, sig1: &ExecutableSignature, sig2: &ExecutableSignature) -> f32 {
        let mut score: f32 = 0.0;

        // Exact hash match = definitive match
        if sig1.file_hash == sig2.file_hash {
            return 100.0;
        }

        // Same file size (good indicator)
        if sig1.file_size == sig2.file_size {
            score += 20.0;
        }

        // Same product name (strong indicator)
        if sig1.product_name == sig2.product_name && sig1.product_name != "Unknown" {
            score += 30.0;
        }

        // Same company name (moderate indicator)
        if sig1.company_name == sig2.company_name && sig1.company_name != "Unknown" {
            score += 15.0;
        }

        // Same original filename (good indicator)
        if sig1.original_filename == sig2.original_filename && sig1.original_filename != "Unknown" {
            score += 20.0;
        }

        // Similar version numbers (weak indicator but useful for patches)
        if sig1.file_version == sig2.file_version && sig1.file_version != "Unknown" {
            score += 5.0;
        }

        score.min(95.0) // Cap at 95% unless it's an exact hash match
    }

    pub async fn clear_cache(&self) {
        let mut cache = self.cache.write().await;
        cache.clear();
    }

    pub async fn get_cached_signature(&self, file_path: &str) -> Option<ExecutableSignature> {
        let cache = self.cache.read().await;
        cache.get(file_path).cloned()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::NamedTempFile;
    use std::io::Write;

    #[tokio::test]
    async fn test_sha256_calculation() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let test_data = b"Hello, World!";
        temp_file.write_all(test_data).unwrap();

        let analyzer = ExecutableAnalyzer::new();
        let hash_result = analyzer.calculate_sha256(temp_file.path().to_str().unwrap()).await.unwrap();

        // SHA-256 of "Hello, World!" (with newline)
        let expected = "dffd6021bb2bd5b0af676290809ec3a53191dd81c7f70a4b28688a362182986f";
        assert_eq!(hash_result, expected);
    }

    #[tokio::test]
    async fn test_nonexistent_file() {
        let analyzer = ExecutableAnalyzer::new();
        let result = analyzer.analyze_executable("/nonexistent/file").await;
        assert!(result.is_err());
    }
}
