use crate::detection::{ProcessInfo, DetectionError};
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{self, Duration};
use chrono::{DateTime, Utc};
use sysinfo::{System, ProcessRefreshKind, ProcessesToUpdate};

#[cfg(target_os = "windows")]
use winapi::um::winuser::{FindWindowA, GetWindowTextA};

#[derive(Debug, Clone)]
pub struct ProcessMonitor {
    system: Arc<RwLock<System>>,
    monitored_processes: Arc<RwLock<HashMap<u32, ProcessInfo>>>,
    is_monitoring: Arc<RwLock<bool>>,
}

impl ProcessMonitor {
    pub fn new() -> Self {
        let mut system = System::new();
        system.refresh_processes(ProcessesToUpdate::All, false);

        Self {
            system: Arc::new(RwLock::new(system)),
            monitored_processes: Arc::new(RwLock::new(HashMap::new())),
            is_monitoring: Arc::new(RwLock::new(false)),
        }
    }

    pub async fn start_monitoring(&self) -> Result<(), DetectionError> {
        let mut is_monitoring = self.is_monitoring.write().await;
        if *is_monitoring {
            return Ok(());
        }
        *is_monitoring = true;

        // Start background monitoring task
        let system = Arc::clone(&self.system);
        let monitored_processes = Arc::clone(&self.monitored_processes);
        let is_monitoring = Arc::clone(&self.is_monitoring);

        tokio::spawn(async move {
            while *is_monitoring.read().await {
                Self::update_process_info(&system, &monitored_processes).await;
                time::sleep(Duration::from_millis(1000)).await; // Update every second
            }
        });

        Ok(())
    }

    pub async fn stop_monitoring(&self) {
        let mut is_monitoring = self.is_monitoring.write().await;
        *is_monitoring = false;
    }

    pub async fn get_process_info(&self, pid: u32) -> Result<ProcessInfo, DetectionError> {
        let system = self.system.read().await;

        if let Some(process) = system.process(sysinfo::Pid::from_u32(pid)) {
            let window_title = self.get_window_title(pid).await;

            Ok(ProcessInfo {
                pid,
                name: process.name().to_string_lossy().to_string(),
                executable_path: process.exe().map(|p| p.display().to_string()).unwrap_or_default(),
                window_title,
                parent_pid: process.parent().map(|p| p.as_u32()),
                creation_time: Some(DateTime::from_timestamp(process.start_time() as i64, 0).unwrap_or_else(Utc::now)),
                memory_usage: process.memory(),
                cpu_usage: process.cpu_usage(),
            })
        } else {
            Err(DetectionError::ProcessMonitoringError(format!("Process {} not found", pid)))
        }
    }

    pub async fn get_running_processes(&self) -> Result<Vec<ProcessInfo>, DetectionError> {
        let system = self.system.read().await;
        let mut processes = Vec::new();

        for (pid, process) in system.processes() {
            let window_title = self.get_window_title(pid.as_u32()).await;

            processes.push(ProcessInfo {
                pid: pid.as_u32(),
                name: process.name().to_string_lossy().to_string(),
                executable_path: process.exe().map(|p| p.display().to_string()).unwrap_or_default(),
                window_title,
                parent_pid: process.parent().map(|p| p.as_u32()),
                creation_time: Some(DateTime::from_timestamp(process.start_time() as i64, 0).unwrap_or_else(Utc::now)),
                memory_usage: process.memory(),
                cpu_usage: process.cpu_usage(),
            });
        }

        Ok(processes)
    }

    pub async fn is_process_running(&self, pid: u32) -> bool {
        let system = self.system.read().await;
        system.process(sysinfo::Pid::from_u32(pid)).is_some()
    }

    pub async fn get_process_by_name(&self, name: &str) -> Result<Vec<ProcessInfo>, DetectionError> {
        let all_processes = self.get_running_processes().await?;
        Ok(all_processes.into_iter()
            .filter(|p| p.name.to_lowercase().contains(&name.to_lowercase()))
            .collect())
    }

    pub async fn get_child_processes(&self, parent_pid: u32) -> Result<Vec<ProcessInfo>, DetectionError> {
        let all_processes = self.get_running_processes().await?;
        Ok(all_processes.into_iter()
            .filter(|p| p.parent_pid == Some(parent_pid))
            .collect())
    }

    pub async fn get_process_tree(&self, root_pid: u32) -> Result<HashMap<u32, Vec<ProcessInfo>>, DetectionError> {
        let all_processes = self.get_running_processes().await?;
        let mut tree = HashMap::new();

        // Build parent -> children mapping
        for process in &all_processes {
            if let Some(parent) = process.parent_pid {
                tree.entry(parent).or_insert_with(Vec::new).push(process.clone());
            }
        }

        // Keep only processes in the tree of root_pid
        let mut to_visit = vec![root_pid];
        let mut visited = std::collections::HashSet::new();
        let mut result = HashMap::new();

        while let Some(pid) = to_visit.pop() {
            if !visited.insert(pid) {
                continue;
            }

            if let Some(children) = tree.get(&pid) {
                result.insert(pid, children.clone());
                to_visit.extend(children.iter().map(|p| p.pid));
            }
        }

        Ok(result)
    }

    async fn get_window_title(&self, pid: u32) -> Option<String> {
        #[cfg(target_os = "windows")]
        {
            use std::ffi::CString;

            // This is a simplified Windows window title detection
            // In a real implementation, you'd need more sophisticated logic
            // to map windows to processes, possibly using additional libraries

            // For now, return None - full window title detection is complex
            // and would require additional dependencies like winapi utilities
            None
        }

        #[cfg(target_os = "macos")]
        {
            // macOS implementation using NSWorkspace or similar
            // This is simplified and would need proper implementation
            None
        }

        #[cfg(target_os = "linux")]
        {
            // Linux implementation using X11 or Wayland APIs
            // This is simplified and would need proper implementation
            None
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    async fn update_process_info(
        system: &Arc<RwLock<System>>,
        monitored_processes: &Arc<RwLock<HashMap<u32, ProcessInfo>>>,
    ) {
        let process_refresh = ProcessRefreshKind::everything();

        {
            let mut sys = system.write().await;
            sys.refresh_processes_specifics(ProcessesToUpdate::All, false, process_refresh);
        }

        let sys = system.read().await;
        let mut monitored = monitored_processes.write().await;

        // Update existing monitored processes
        let mut to_remove = Vec::new();
        for (pid, process_info) in monitored.iter_mut() {
            if let Some(process) = sys.process(sysinfo::Pid::from_u32(*pid)) {
                process_info.memory_usage = process.memory();
                process_info.cpu_usage = process.cpu_usage();

                // Update window title if needed
                if process_info.window_title.is_none() {
                    process_info.window_title = Self::get_window_title_static(*pid).await;
                }
            } else {
                to_remove.push(*pid);
            }
        }

        // Remove dead processes
        for pid in to_remove {
            monitored.remove(&pid);
        }
    }

    async fn get_window_title_static(pid: u32) -> Option<String> {
        // Static version that doesn't require self reference
        #[cfg(target_os = "windows")]
        {
            None // Simplified implementation
        }

        #[cfg(target_os = "macos")]
        {
            None // Simplified implementation
        }

        #[cfg(target_os = "linux")]
        {
            None // Simplified implementation
        }

        #[cfg(not(any(target_os = "windows", target_os = "macos", target_os = "linux")))]
        {
            None
        }
    }

    pub async fn get_process_status_summary(&self) -> ProcessStatusSummary {
        let processes = self.get_running_processes().await.unwrap_or_default();

        let total_processes = processes.len();
        let total_memory = processes.iter().map(|p| p.memory_usage).sum::<u64>() as f64 / 1024.0 / 1024.0; // MB
        let total_cpu = processes.iter().map(|p| p.cpu_usage).sum::<f32>();

        // Count processes by type (basic categorization)
        let mut system_processes = 0;
        let mut game_processes = 0;
        let mut other_processes = 0;

        let system_process_names = [
            "system", "init", "systemd", "launchd", "winlogon", "csrss", "smss",
            "services", "lsass", "wininit", "kernel_task", "WindowServer", "Dock"
        ];

        let game_extension_indicators = [
            ".exe", ".app", ".bin", ".game"
        ];

        for process in &processes {
            let name = process.name.to_lowercase();
            if system_process_names.iter().any(|&sys| name.contains(sys)) {
                system_processes += 1;
            } else if game_extension_indicators.iter().any(|&ext| name.ends_with(ext))
                || process.memory_usage > 100_000_000 // 100MB heuristic
                || process.cpu_usage > 10.0 { // High CPU usage heuristic
                game_processes += 1;
            } else {
                other_processes += 1;
            }
        }

        ProcessStatusSummary {
            total_processes,
            system_processes,
            game_processes,
            other_processes,
            total_memory_mb: total_memory,
            total_cpu_percent: total_cpu,
        }
    }
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct ProcessStatusSummary {
    pub total_processes: usize,
    pub system_processes: usize,
    pub game_processes: usize,
    pub other_processes: usize,
    pub total_memory_mb: f64,
    pub total_cpu_percent: f32,
}
