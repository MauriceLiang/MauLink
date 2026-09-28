use std::{
    collections::{BTreeMap, HashMap, HashSet, VecDeque},
    sync::{
        Arc, Mutex as StdMutex,
        atomic::{AtomicBool, Ordering},
    },
    time::{Duration, Instant},
};

use tokio::sync::{Mutex, RwLock};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use uuid::Uuid;

use crate::{
    AppError, ConnectionState, DecimalU64, ErrorCode, MonitorCpuSnapshot, MonitorDiskSnapshot,
    MonitorGetHistoryPayload, MonitorHistoryMetric, MonitorHistoryPage, MonitorHistorySample,
    MonitorLoadSnapshot, MonitorMemorySnapshot, MonitorMetricQuality, MonitorNetworkInterface,
    MonitorNetworkSnapshot, MonitorQualityStatus, MonitorRefreshPayload, MonitorSnapshot,
    MonitorSystemSnapshot, MonitorUptimeSnapshot, SshConnectionManager,
};

const MAX_HISTORY_SAMPLES: usize = 900;
const DEFAULT_HISTORY_WINDOW: std::time::Duration = std::time::Duration::from_secs(15 * 60);
const MONITOR_BATCH_TIMEOUT: Duration = Duration::from_secs(3);

#[derive(Debug, Clone, Copy, Default)]
pub(crate) struct CollectionMask {
    cpu: bool,
    memory: bool,
    disk: bool,
    network: bool,
    load: bool,
    uptime: bool,
    system: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum MonitorParseError {
    Unsupported,
    Invalid,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct CpuCounters {
    total: u64,
    idle_all: u64,
    logical_cores: Option<u32>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct MemoryCounters {
    total_bytes: u64,
    available_bytes: u64,
    used_bytes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct DiskCounters {
    source: String,
    mount: String,
    total_bytes: u64,
    used_bytes: u64,
    available_bytes: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct NetworkCounters {
    received_bytes: u64,
    transmitted_bytes: u64,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedMonitorOutput {
    pub(crate) cpu: Result<CpuCounters, MonitorParseError>,
    pub(crate) memory: Result<MemoryCounters, MonitorParseError>,
    pub(crate) disk: Result<DiskCounters, MonitorParseError>,
    pub(crate) network: Result<BTreeMap<String, NetworkCounters>, MonitorParseError>,
    pub(crate) load: Result<[f64; 3], MonitorParseError>,
    pub(crate) uptime_seconds: Result<u64, MonitorParseError>,
    pub(crate) system: Result<ParsedSystemInfo, MonitorParseError>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ParsedSystemInfo {
    pub(crate) hostname: String,
    pub(crate) os: Option<String>,
    pub(crate) kernel: String,
    pub(crate) architecture: String,
}

pub(crate) fn fixed_collection_script(mask: CollectionMask) -> String {
    let mut script = String::from("LC_ALL=C; export LC_ALL\n");
    if mask.cpu {
        script.push_str(
            "printf '%s\\n' '@@CPU@@'; if [ -r /proc/stat ]; then head -n 1024 /proc/stat; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_CPU@@'\n",
        );
    }
    if mask.memory {
        script.push_str(
            "printf '%s\\n' '@@MEMORY@@'; if [ -r /proc/meminfo ]; then cat /proc/meminfo; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_MEMORY@@'\n",
        );
    }
    if mask.disk {
        script.push_str(
            "printf '%s\\n' '@@DISK@@'; if command -v df >/dev/null 2>&1; then df -kP /; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_DISK@@'\n",
        );
    }
    if mask.network {
        script.push_str(
            "printf '%s\\n' '@@NETWORK@@'; if [ -r /proc/net/dev ]; then cat /proc/net/dev; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_NETWORK@@'\n",
        );
    }
    if mask.load {
        script.push_str(
            "printf '%s\\n' '@@LOAD@@'; if [ -r /proc/loadavg ]; then cat /proc/loadavg; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_LOAD@@'\n",
        );
    }
    if mask.uptime {
        script.push_str(
            "printf '%s\\n' '@@UPTIME@@'; if [ -r /proc/uptime ]; then cat /proc/uptime; else printf '%s\\n' '__UNSUPPORTED__'; fi; printf '%s\\n' '@@END_UPTIME@@'\n",
        );
    }
    if mask.system {
        script.push_str(
            "printf '%s\\n' '@@SYSTEM@@'; if command -v uname >/dev/null 2>&1; then uname -snrm; else printf '%s\\n' '__UNSUPPORTED__'; fi; if [ -r /etc/os-release ]; then grep -E '^(PRETTY_NAME|NAME)=' /etc/os-release; fi; printf '%s\\n' '@@END_SYSTEM@@'\n",
        );
    }
    script
}

pub(crate) fn parse_monitor_output(output: &str) -> ParsedMonitorOutput {
    let sections = parse_sections(output);
    ParsedMonitorOutput {
        cpu: section(&sections, "CPU").and_then(parse_cpu_counters),
        memory: section(&sections, "MEMORY").and_then(parse_memory_counters),
        disk: section(&sections, "DISK").and_then(parse_disk_counters),
        network: section(&sections, "NETWORK").and_then(parse_network_counters),
        load: section(&sections, "LOAD").and_then(parse_load_average),
        uptime_seconds: section(&sections, "UPTIME").and_then(parse_uptime),
        system: section(&sections, "SYSTEM").and_then(parse_system_info),
    }
}

fn parse_sections(output: &str) -> BTreeMap<String, String> {
    let mut sections = BTreeMap::new();
    let mut active: Option<String> = None;
    for line in output.lines() {
        if let Some(name) = line
            .strip_prefix("@@")
            .and_then(|line| line.strip_suffix("@@"))
            .and_then(|line| line.strip_prefix("END_").map(|end| (line, end)))
        {
            if active.as_deref() == Some(name.1) {
                active = None;
            }
            continue;
        }
        if let Some(name) = line
            .strip_prefix("@@")
            .and_then(|line| line.strip_suffix("@@"))
        {
            active = Some(name.to_owned());
            sections.entry(name.to_owned()).or_insert_with(String::new);
            continue;
        }
        if let Some(active) = active.as_ref() {
            sections.entry(active.clone()).or_default().push_str(line);
            sections.entry(active.clone()).or_default().push('\n');
        }
    }
    sections
}

fn section<'a>(
    sections: &'a BTreeMap<String, String>,
    name: &str,
) -> Result<&'a str, MonitorParseError> {
    let contents = sections.get(name).ok_or(MonitorParseError::Unsupported)?;
    if contents
        .lines()
        .any(|line| line.trim() == "__UNSUPPORTED__")
    {
        return Err(MonitorParseError::Unsupported);
    }
    Ok(contents)
}

pub(crate) fn parse_cpu_counters(contents: &str) -> Result<CpuCounters, MonitorParseError> {
    let aggregate = contents
        .lines()
        .find(|line| line.starts_with("cpu "))
        .ok_or(MonitorParseError::Invalid)?;
    let values = aggregate
        .split_whitespace()
        .skip(1)
        .map(|value| value.parse::<u64>().map_err(|_| MonitorParseError::Invalid))
        .collect::<Result<Vec<_>, _>>()?;
    if values.len() < 8 {
        return Err(MonitorParseError::Invalid);
    }
    let total = values[..8]
        .iter()
        .try_fold(0_u64, |total, value| total.checked_add(*value))
        .ok_or(MonitorParseError::Invalid)?;
    let idle_all = values[3]
        .checked_add(values[4])
        .ok_or(MonitorParseError::Invalid)?;
    let logical_cores = contents
        .lines()
        .filter(|line| {
            line.split_whitespace()
                .next()
                .unwrap_or_default()
                .strip_prefix("cpu")
                .is_some_and(|suffix| {
                    !suffix.is_empty() && suffix.bytes().all(|b| b.is_ascii_digit())
                })
        })
        .count();
    Ok(CpuCounters {
        total,
        idle_all,
        logical_cores: u32::try_from(logical_cores).ok().filter(|cores| *cores > 0),
    })
}

pub(crate) fn parse_memory_counters(contents: &str) -> Result<MemoryCounters, MonitorParseError> {
    let mut fields = BTreeMap::new();
    for line in contents.lines() {
        let Some((name, rest)) = line.split_once(':') else {
            continue;
        };
        let mut values = rest.split_whitespace();
        let Some(value) = values.next() else {
            continue;
        };
        let value = value
            .parse::<u64>()
            .map_err(|_| MonitorParseError::Invalid)?;
        let value_bytes = if values.next() == Some("kB") {
            value.checked_mul(1024).ok_or(MonitorParseError::Invalid)?
        } else {
            value
        };
        fields.insert(name, value_bytes);
    }
    let total_bytes = *fields.get("MemTotal").ok_or(MonitorParseError::Invalid)?;
    let available_bytes = *fields
        .get("MemAvailable")
        .ok_or(MonitorParseError::Unsupported)?;
    if total_bytes == 0 || available_bytes > total_bytes {
        return Err(MonitorParseError::Invalid);
    }
    Ok(MemoryCounters {
        total_bytes,
        available_bytes,
        used_bytes: total_bytes - available_bytes,
    })
}

pub(crate) fn parse_disk_counters(contents: &str) -> Result<DiskCounters, MonitorParseError> {
    for line in contents.lines().skip(1) {
        let fields = line.split_whitespace().collect::<Vec<_>>();
        if fields.len() < 6 {
            continue;
        }
        // `df -kP` can include inode columns on some BSD systems. The root
        // mount is still the final field because this collector asks for `/`.
        if fields.last() != Some(&"/") {
            continue;
        }
        let size_kib = fields[1]
            .parse::<u64>()
            .map_err(|_| MonitorParseError::Invalid)?;
        let used_kib = fields[2]
            .parse::<u64>()
            .map_err(|_| MonitorParseError::Invalid)?;
        let available_kib = fields[3]
            .parse::<u64>()
            .map_err(|_| MonitorParseError::Invalid)?;
        let total_bytes = size_kib
            .checked_mul(1024)
            .ok_or(MonitorParseError::Invalid)?;
        let used_bytes = used_kib
            .checked_mul(1024)
            .ok_or(MonitorParseError::Invalid)?;
        let available_bytes = available_kib
            .checked_mul(1024)
            .ok_or(MonitorParseError::Invalid)?;
        if total_bytes == 0 || used_bytes > total_bytes || available_bytes > total_bytes {
            return Err(MonitorParseError::Invalid);
        }
        return Ok(DiskCounters {
            source: fields[0].to_owned(),
            mount: "/".to_owned(),
            total_bytes,
            used_bytes,
            available_bytes,
        });
    }
    Err(MonitorParseError::Unsupported)
}

pub(crate) fn parse_network_counters(
    contents: &str,
) -> Result<BTreeMap<String, NetworkCounters>, MonitorParseError> {
    let mut interfaces = BTreeMap::new();
    for line in contents.lines().skip(2) {
        let Some((name, counters)) = line.split_once(':') else {
            continue;
        };
        let name = name.trim();
        let values = counters
            .split_whitespace()
            .map(|value| value.parse::<u64>().map_err(|_| MonitorParseError::Invalid))
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() < 9 || name.is_empty() {
            return Err(MonitorParseError::Invalid);
        }
        interfaces.insert(
            name.to_owned(),
            NetworkCounters {
                received_bytes: values[0],
                transmitted_bytes: values[8],
            },
        );
    }
    if interfaces.is_empty() {
        return Err(MonitorParseError::Unsupported);
    }
    Ok(interfaces)
}

pub(crate) fn parse_load_average(contents: &str) -> Result<[f64; 3], MonitorParseError> {
    let values = contents
        .split_whitespace()
        .take(3)
        .map(|value| parse_finite_nonnegative(value))
        .collect::<Result<Vec<_>, _>>()?;
    values.try_into().map_err(|_| MonitorParseError::Invalid)
}

pub(crate) fn parse_uptime(contents: &str) -> Result<u64, MonitorParseError> {
    let value = contents
        .split_whitespace()
        .next()
        .ok_or(MonitorParseError::Invalid)?;
    let seconds = parse_finite_nonnegative(value)?;
    if seconds > u64::MAX as f64 {
        return Err(MonitorParseError::Invalid);
    }
    Ok(seconds as u64)
}

pub(crate) fn parse_system_info(contents: &str) -> Result<ParsedSystemInfo, MonitorParseError> {
    let uname = contents
        .lines()
        .find(|line| !line.trim().is_empty() && !line.contains('='))
        .ok_or(MonitorParseError::Invalid)?;
    let fields = uname.split_whitespace().collect::<Vec<_>>();
    if fields.len() != 4 {
        return Err(MonitorParseError::Invalid);
    }
    let os = contents
        .lines()
        .find_map(|line| {
            line.strip_prefix("PRETTY_NAME=")
                .or_else(|| line.strip_prefix("NAME="))
        })
        .map(parse_os_release_value)
        .filter(|value| !value.is_empty());
    Ok(ParsedSystemInfo {
        kernel: format!("{} {}", fields[0], fields[2]),
        hostname: fields[1].to_owned(),
        architecture: fields[3].to_owned(),
        os,
    })
}

fn parse_os_release_value(value: &str) -> String {
    let value = value.trim();
    if value.len() >= 2
        && ((value.starts_with('"') && value.ends_with('"'))
            || (value.starts_with('\'') && value.ends_with('\'')))
    {
        value[1..value.len() - 1].to_owned()
    } else {
        value.to_owned()
    }
}

fn parse_finite_nonnegative(value: &str) -> Result<f64, MonitorParseError> {
    let value = value
        .parse::<f64>()
        .map_err(|_| MonitorParseError::Invalid)?;
    if !value.is_finite() || value < 0.0 {
        return Err(MonitorParseError::Invalid);
    }
    Ok(value)
}

pub(crate) fn cpu_usage_percent(previous: CpuCounters, current: CpuCounters) -> Option<f64> {
    if previous.logical_cores != current.logical_cores
        || current.total <= previous.total
        || current.idle_all < previous.idle_all
    {
        return None;
    }
    let delta_total = current.total - previous.total;
    let delta_idle = current.idle_all - previous.idle_all;
    if delta_idle > delta_total {
        return None;
    }
    Some(100.0 * (delta_total - delta_idle) as f64 / delta_total as f64)
}

pub(crate) fn network_rates(
    previous: &BTreeMap<String, NetworkCounters>,
    current: &BTreeMap<String, NetworkCounters>,
    interval_seconds: f64,
) -> Option<(BTreeMap<String, (f64, f64)>, f64, f64)> {
    if !interval_seconds.is_finite()
        || interval_seconds <= 0.0
        || previous.keys().ne(current.keys())
    {
        return None;
    }
    let mut rates = BTreeMap::new();
    let mut total_received = 0.0;
    let mut total_transmitted = 0.0;
    for (name, current) in current {
        let previous = previous.get(name)?;
        if current.received_bytes < previous.received_bytes
            || current.transmitted_bytes < previous.transmitted_bytes
        {
            return None;
        }
        let received = (current.received_bytes - previous.received_bytes) as f64 / interval_seconds;
        let transmitted =
            (current.transmitted_bytes - previous.transmitted_bytes) as f64 / interval_seconds;
        rates.insert(name.clone(), (received, transmitted));
        if name != "lo" {
            total_received += received;
            total_transmitted += transmitted;
        }
    }
    Some((rates, total_received, total_transmitted))
}

fn quality(status: MonitorQualityStatus) -> MonitorMetricQuality {
    MonitorMetricQuality {
        status,
        sampled_at_ms: None,
        collection_duration_ms: None,
        error_code: None,
    }
}

fn initial_snapshot(connection_id: &str) -> MonitorSnapshot {
    let warming = || quality(MonitorQualityStatus::WarmingUp);
    MonitorSnapshot {
        connection_id: connection_id.to_owned(),
        cpu: MonitorCpuSnapshot {
            usage_percent: None,
            logical_cores: None,
            quality: warming(),
        },
        load: MonitorLoadSnapshot {
            one_minute: None,
            five_minutes: None,
            fifteen_minutes: None,
            quality: warming(),
        },
        memory: MonitorMemorySnapshot {
            used_bytes: None,
            total_bytes: None,
            available_bytes: None,
            used_percent: None,
            quality: warming(),
        },
        disk: MonitorDiskSnapshot {
            used_bytes: None,
            total_bytes: None,
            available_bytes: None,
            used_percent: None,
            source: None,
            mount: None,
            quality: warming(),
        },
        network: MonitorNetworkSnapshot {
            received_bytes_per_second: None,
            transmitted_bytes_per_second: None,
            interfaces: Vec::new(),
            quality: warming(),
        },
        uptime: MonitorUptimeSnapshot {
            seconds: None,
            quality: warming(),
        },
        system: MonitorSystemSnapshot {
            hostname: None,
            os: None,
            kernel: None,
            architecture: None,
            quality: warming(),
        },
    }
}

fn monitor_error(code: ErrorCode, message_key: &'static str) -> AppError {
    AppError::new(code, message_key).with_stage("collectingMonitorMetrics")
}

#[derive(Clone)]
pub struct MonitorManager {
    connections: SshConnectionManager,
    sessions: Arc<Mutex<HashMap<String, Arc<Mutex<MonitorSession>>>>>,
    activity: Arc<RwLock<MonitorActivity>>,
    shutdown: CancellationToken,
    scheduler_started: Arc<AtomicBool>,
    scheduler_task: Arc<StdMutex<Option<JoinHandle<()>>>>,
}

#[derive(Default, Clone)]
struct MonitorActivity {
    active_connection_id: Option<String>,
    monitor_visible: bool,
    window_minimized: bool,
}

struct MonitorSession {
    snapshot: MonitorSnapshot,
    cpu_baseline: Option<(CpuCounters, Instant)>,
    network_baseline: Option<(BTreeMap<String, NetworkCounters>, Instant)>,
    last_cpu: Option<Instant>,
    last_network: Option<Instant>,
    last_load: Option<Instant>,
    last_uptime: Option<Instant>,
    last_memory: Option<Instant>,
    last_disk: Option<Instant>,
    last_system: Option<Instant>,
    retry_at: Option<Instant>,
    consecutive_failures: u8,
    in_flight: bool,
    history: BTreeMap<MonitorHistoryMetric, VecDeque<MonitorHistorySample>>,
}

impl MonitorSession {
    fn new(connection_id: &str) -> Self {
        Self {
            snapshot: initial_snapshot(connection_id),
            cpu_baseline: None,
            network_baseline: None,
            last_cpu: None,
            last_network: None,
            last_load: None,
            last_uptime: None,
            last_memory: None,
            last_disk: None,
            last_system: None,
            retry_at: None,
            consecutive_failures: 0,
            in_flight: false,
            history: BTreeMap::new(),
        }
    }
}

impl MonitorManager {
    pub fn new(connections: SshConnectionManager) -> Self {
        Self {
            connections,
            sessions: Arc::new(Mutex::new(HashMap::new())),
            activity: Arc::new(RwLock::new(MonitorActivity::default())),
            shutdown: CancellationToken::new(),
            scheduler_started: Arc::new(AtomicBool::new(false)),
            scheduler_task: Arc::new(StdMutex::new(None)),
        }
    }

    pub async fn get_snapshot(
        &self,
        payload: crate::MonitorGetSnapshotPayload,
    ) -> Result<MonitorSnapshot, AppError> {
        let connection = self.connections.get(&payload.connection_id)?;
        if connection.state != ConnectionState::Ready {
            return Err(monitor_validation(
                "connectionId",
                "errors.connectionNotReady",
            ));
        }
        let session = self.session(&payload.connection_id).await;
        Ok(session.lock().await.snapshot.clone())
    }

    pub async fn get_history(
        &self,
        payload: MonitorGetHistoryPayload,
    ) -> Result<MonitorHistoryPage, AppError> {
        validate_monitor_connection_id(&payload.connection_id)?;
        let connection = self.connections.get(&payload.connection_id)?;
        if connection.state != ConnectionState::Ready {
            return Err(monitor_validation(
                "connectionId",
                "errors.connectionNotReady",
            ));
        }
        let now_ms = crate::storage::now_ms()?;
        let from_ms = payload
            .from_ms
            .unwrap_or_else(|| now_ms.saturating_sub(DEFAULT_HISTORY_WINDOW.as_millis() as i64));
        let to_ms = payload.to_ms.unwrap_or(now_ms);
        if from_ms < 0 || to_ms < from_ms {
            return Err(monitor_validation(
                "historyRange",
                "errors.monitorHistoryRangeInvalid",
            ));
        }
        let limit = usize::from(payload.limit.unwrap_or(MAX_HISTORY_SAMPLES as u16))
            .clamp(1, MAX_HISTORY_SAMPLES);
        let session = self.session(&payload.connection_id).await;
        let session = session.lock().await;
        let samples = session
            .history
            .get(&payload.metric)
            .into_iter()
            .flat_map(|samples| samples.iter())
            .filter(|sample| sample.sampled_at_ms >= from_ms && sample.sampled_at_ms <= to_ms)
            .cloned()
            .collect::<Vec<_>>();
        let start = samples.len().saturating_sub(limit);
        Ok(MonitorHistoryPage {
            connection_id: payload.connection_id,
            metric: payload.metric,
            samples: samples.into_iter().skip(start).collect(),
        })
    }

    pub async fn refresh(
        &self,
        payload: MonitorRefreshPayload,
    ) -> Result<MonitorSnapshot, AppError> {
        let connection = self.connections.get(&payload.connection_id)?;
        if connection.state != ConnectionState::Ready {
            return Err(monitor_validation(
                "connectionId",
                "errors.connectionNotReady",
            ));
        }
        let session = self.session(&payload.connection_id).await;
        {
            let mut session = session.lock().await;
            session.retry_at = None;
            session.consecutive_failures = 0;
            session.last_cpu = None;
            session.last_network = None;
            session.last_load = None;
            session.last_uptime = None;
            session.last_memory = None;
            session.last_disk = None;
            session.last_system = None;
            session.cpu_baseline = None;
            session.network_baseline = None;
        }
        self.collect(&payload.connection_id, CollectionMask::all())
            .await?;
        self.get_snapshot(crate::MonitorGetSnapshotPayload {
            connection_id: payload.connection_id,
        })
        .await
    }

    pub async fn set_activity(
        &self,
        payload: crate::WorkspaceActivityPayload,
    ) -> Result<(), AppError> {
        if let Some(connection_id) = payload.active_connection_id.as_deref() {
            validate_monitor_connection_id(connection_id)?;
            self.connections.get(connection_id)?;
        }
        let should_refresh = {
            let mut activity = self.activity.write().await;
            let changed_to_visible = payload.monitor_visible
                && (activity.active_connection_id != payload.active_connection_id
                    || !activity.monitor_visible);
            activity.active_connection_id = payload.active_connection_id.clone();
            activity.monitor_visible = payload.monitor_visible;
            changed_to_visible
        };
        self.ensure_scheduler();
        if should_refresh && let Some(connection_id) = payload.active_connection_id {
            let manager = self.clone();
            tokio::spawn(async move {
                manager.refresh_foreground(&connection_id).await;
            });
        }
        Ok(())
    }

    pub async fn set_window_minimized(&self, minimized: bool) {
        let restored = {
            let mut activity = self.activity.write().await;
            let restored = activity.window_minimized && !minimized;
            activity.window_minimized = minimized;
            restored
        };
        if restored {
            let sessions = self
                .sessions
                .lock()
                .await
                .values()
                .cloned()
                .collect::<Vec<_>>();
            for session in sessions {
                let mut session = session.lock().await;
                session.last_cpu = None;
                session.last_network = None;
                session.last_load = None;
                session.last_uptime = None;
                session.last_memory = None;
                session.last_disk = None;
                session.cpu_baseline = None;
                session.network_baseline = None;
            }
        }
    }

    pub async fn close_connection(&self, connection_id: &str) {
        self.sessions.lock().await.remove(connection_id);
        let mut activity = self.activity.write().await;
        if activity.active_connection_id.as_deref() == Some(connection_id) {
            activity.active_connection_id = None;
            activity.monitor_visible = false;
        }
    }

    pub async fn shutdown(&self) {
        self.shutdown.cancel();
        let task = self
            .scheduler_task
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner())
            .take();
        if let Some(task) = task {
            let _ = task.await;
        }
        self.sessions.lock().await.clear();
    }

    async fn refresh_foreground(&self, connection_id: &str) {
        let session = self.session(connection_id).await;
        let mask = {
            let mut session = session.lock().await;
            session.retry_at = None;
            session.consecutive_failures = 0;
            session.cpu_baseline = None;
            session.network_baseline = None;
            session.last_cpu = None;
            session.last_network = None;
            session.last_load = None;
            session.last_uptime = None;
            session.last_memory = None;
            session.last_disk = None;
            CollectionMask {
                system: session.last_system.is_none(),
                ..CollectionMask::all()
            }
        };
        let _ = self.collect(connection_id, mask).await;
    }

    async fn session(&self, connection_id: &str) -> Arc<Mutex<MonitorSession>> {
        let mut sessions = self.sessions.lock().await;
        sessions
            .entry(connection_id.to_owned())
            .or_insert_with(|| Arc::new(Mutex::new(MonitorSession::new(connection_id))))
            .clone()
    }

    fn ensure_scheduler(&self) {
        if self
            .scheduler_started
            .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
            .is_err()
        {
            return;
        }
        let Ok(runtime) = tokio::runtime::Handle::try_current() else {
            self.scheduler_started.store(false, Ordering::Release);
            return;
        };
        let manager = self.clone();
        let task = runtime.spawn(async move { manager.scheduler_loop().await });
        *self
            .scheduler_task
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner()) = Some(task);
    }

    async fn scheduler_loop(&self) {
        let mut ticker = tokio::time::interval(Duration::from_secs(1));
        ticker.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
        loop {
            tokio::select! {
                _ = self.shutdown.cancelled() => break,
                _ = ticker.tick() => {
                    if self.shutdown.is_cancelled() { break; }
                    let activity = self.activity.read().await.clone();
                    if activity.window_minimized { continue; }
                    let Ok(connections) = self.connections.connection_snapshots() else { continue; };
                    let ready = connections.into_iter()
                        .filter(|connection| connection.state == ConnectionState::Ready)
                        .collect::<Vec<_>>();
                    let ready_ids = ready.iter()
                        .map(|connection| connection.connection_id.as_str())
                        .collect::<HashSet<_>>();
                    self.sessions.lock().await.retain(|connection_id, _| ready_ids.contains(connection_id.as_str()));
                    for connection in ready {
                        let foreground = activity.monitor_visible
                            && activity.active_connection_id.as_deref() == Some(&connection.connection_id);
                        let manager = self.clone();
                        let connection_id = connection.connection_id;
                        tokio::spawn(async move {
                            let mask = manager.due_mask(&connection_id, foreground).await;
                            if mask.has_any() {
                                let _ = manager.collect(&connection_id, mask).await;
                            }
                        });
                    }
                }
            }
        }
    }

    async fn due_mask(&self, connection_id: &str, foreground: bool) -> CollectionMask {
        let session = self.session(connection_id).await;
        let session = session.lock().await;
        due_mask_for_session(&session, foreground, Instant::now())
    }

    async fn collect(&self, connection_id: &str, mask: CollectionMask) -> Result<(), AppError> {
        if !mask.has_any() {
            return Ok(());
        }
        validate_monitor_connection_id(connection_id)?;
        if self.connections.get(connection_id)?.state != ConnectionState::Ready {
            return Err(monitor_validation(
                "connectionId",
                "errors.connectionNotReady",
            ));
        }
        let session = self.session(connection_id).await;
        {
            let mut session = session.lock().await;
            if session.in_flight {
                return Ok(());
            }
            session.in_flight = true;
            let sampled_at = Instant::now();
            if mask.cpu {
                session.last_cpu = Some(sampled_at);
            }
            if mask.network {
                session.last_network = Some(sampled_at);
            }
            if mask.load {
                session.last_load = Some(sampled_at);
            }
            if mask.uptime {
                session.last_uptime = Some(sampled_at);
            }
            if mask.memory {
                session.last_memory = Some(sampled_at);
            }
            if mask.disk {
                session.last_disk = Some(sampled_at);
            }
            if mask.system {
                session.last_system = Some(sampled_at);
            }
        }
        let started = Instant::now();
        let script = fixed_collection_script(mask);
        let result = self
            .connections
            .run_fixed_command(connection_id, &script, MONITOR_BATCH_TIMEOUT)
            .await;
        let mut session = session.lock().await;
        session.in_flight = false;
        match result {
            Ok(output) if output.exit_status.is_none_or(|status| status == 0) => {
                let parsed = parse_monitor_output(&output.stdout);
                let collection_duration_ms = duration_ms(started.elapsed());
                let sampled_at_ms = crate::storage::now_ms()?
                    .saturating_sub(i64::try_from(collection_duration_ms / 2).unwrap_or(i64::MAX));
                apply_collection(
                    &mut session,
                    mask,
                    parsed,
                    sampled_at_ms,
                    collection_duration_ms,
                    started + started.elapsed() / 2,
                );
                session.consecutive_failures = 0;
                session.retry_at = None;
                Ok(())
            }
            Ok(_) => {
                let error = monitor_error(
                    ErrorCode::MonitorCollectionFailed,
                    "errors.monitorCollectionFailed",
                );
                record_collection_failure(&mut session, mask, &error);
                Err(error)
            }
            Err(error) => {
                record_collection_failure(&mut session, mask, &error);
                Err(error)
            }
        }
    }
}

impl CollectionMask {
    fn all() -> Self {
        Self {
            cpu: true,
            memory: true,
            disk: true,
            network: true,
            load: true,
            uptime: true,
            system: true,
        }
    }

    fn has_any(self) -> bool {
        self.cpu
            || self.memory
            || self.disk
            || self.network
            || self.load
            || self.uptime
            || self.system
    }
}

fn due(last_sample: Option<Instant>, interval: Duration, now: Instant) -> bool {
    last_sample.is_none_or(|last_sample| now.duration_since(last_sample) >= interval)
}

fn due_mask_for_session(
    session: &MonitorSession,
    foreground: bool,
    now: Instant,
) -> CollectionMask {
    if session.in_flight || session.retry_at.is_some_and(|retry_at| retry_at > now) {
        return CollectionMask::default();
    }
    let fast_interval = Duration::from_secs(if foreground { 1 } else { 10 });
    let memory_interval = Duration::from_secs(if foreground { 2 } else { 10 });
    let disk_interval = Duration::from_secs(if foreground { 5 } else { 30 });
    CollectionMask {
        cpu: not_unsupported(&session.snapshot.cpu.quality)
            && due(session.last_cpu, fast_interval, now),
        network: not_unsupported(&session.snapshot.network.quality)
            && due(session.last_network, fast_interval, now),
        load: not_unsupported(&session.snapshot.load.quality)
            && due(session.last_load, fast_interval, now),
        uptime: not_unsupported(&session.snapshot.uptime.quality)
            && due(session.last_uptime, fast_interval, now),
        memory: not_unsupported(&session.snapshot.memory.quality)
            && due(session.last_memory, memory_interval, now),
        disk: not_unsupported(&session.snapshot.disk.quality)
            && due(session.last_disk, disk_interval, now),
        system: not_unsupported(&session.snapshot.system.quality) && session.last_system.is_none(),
    }
}

fn not_unsupported(quality: &MonitorMetricQuality) -> bool {
    quality.status != MonitorQualityStatus::Unsupported
}

fn validate_monitor_connection_id(connection_id: &str) -> Result<(), AppError> {
    Uuid::parse_str(connection_id)
        .map(|_| ())
        .map_err(|_| monitor_validation("connectionId", "errors.resourceIdInvalid"))
}

fn monitor_validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

fn duration_ms(duration: Duration) -> u64 {
    u64::try_from(duration.as_millis()).unwrap_or(u64::MAX)
}

fn apply_collection(
    session: &mut MonitorSession,
    mask: CollectionMask,
    parsed: ParsedMonitorOutput,
    sampled_at_ms: i64,
    collection_duration_ms: u64,
    sampled_at: Instant,
) {
    if mask.cpu {
        match parsed.cpu {
            Ok(current) => {
                let usage = session
                    .cpu_baseline
                    .as_ref()
                    .and_then(|(previous, _)| cpu_usage_percent(*previous, current));
                session.cpu_baseline = Some((current, sampled_at));
                session.snapshot.cpu.logical_cores = current.logical_cores;
                session.snapshot.cpu.usage_percent = usage;
                set_quality(
                    &mut session.snapshot.cpu.quality,
                    if usage.is_some() {
                        MonitorQualityStatus::Ok
                    } else {
                        MonitorQualityStatus::WarmingUp
                    },
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
                if let Some(value) = usage {
                    push_history(
                        session,
                        MonitorHistoryMetric::CpuUsage,
                        sampled_at_ms,
                        value,
                    );
                }
            }
            Err(error) => {
                session.cpu_baseline = None;
                record_parse_failure(
                    &mut session.snapshot.cpu.quality,
                    session.snapshot.cpu.usage_percent.is_some()
                        || session.snapshot.cpu.logical_cores.is_some(),
                    error,
                    sampled_at_ms,
                    collection_duration_ms,
                );
            }
        }
    }
    if mask.load {
        match parsed.load {
            Ok(values) => {
                session.snapshot.load.one_minute = Some(values[0]);
                session.snapshot.load.five_minutes = Some(values[1]);
                session.snapshot.load.fifteen_minutes = Some(values[2]);
                set_quality(
                    &mut session.snapshot.load.quality,
                    MonitorQualityStatus::Ok,
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
                push_history(
                    session,
                    MonitorHistoryMetric::LoadOneMinute,
                    sampled_at_ms,
                    values[0],
                );
            }
            Err(error) => record_parse_failure(
                &mut session.snapshot.load.quality,
                session.snapshot.load.one_minute.is_some(),
                error,
                sampled_at_ms,
                collection_duration_ms,
            ),
        }
    }
    if mask.memory {
        match parsed.memory {
            Ok(memory) => {
                let percent = 100.0 * memory.used_bytes as f64 / memory.total_bytes as f64;
                session.snapshot.memory.used_bytes = Some(DecimalU64(memory.used_bytes));
                session.snapshot.memory.total_bytes = Some(DecimalU64(memory.total_bytes));
                session.snapshot.memory.available_bytes = Some(DecimalU64(memory.available_bytes));
                session.snapshot.memory.used_percent = Some(percent);
                set_quality(
                    &mut session.snapshot.memory.quality,
                    MonitorQualityStatus::Ok,
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
                push_history(
                    session,
                    MonitorHistoryMetric::MemoryUsage,
                    sampled_at_ms,
                    percent,
                );
            }
            Err(error) => record_parse_failure(
                &mut session.snapshot.memory.quality,
                session.snapshot.memory.total_bytes.is_some(),
                error,
                sampled_at_ms,
                collection_duration_ms,
            ),
        }
    }
    if mask.disk {
        match parsed.disk {
            Ok(disk) => {
                let percent = 100.0 * disk.used_bytes as f64 / disk.total_bytes as f64;
                session.snapshot.disk.used_bytes = Some(DecimalU64(disk.used_bytes));
                session.snapshot.disk.total_bytes = Some(DecimalU64(disk.total_bytes));
                session.snapshot.disk.available_bytes = Some(DecimalU64(disk.available_bytes));
                session.snapshot.disk.used_percent = Some(percent);
                session.snapshot.disk.source = Some(disk.source);
                session.snapshot.disk.mount = Some(disk.mount);
                set_quality(
                    &mut session.snapshot.disk.quality,
                    MonitorQualityStatus::Ok,
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
                push_history(
                    session,
                    MonitorHistoryMetric::DiskUsage,
                    sampled_at_ms,
                    percent,
                );
            }
            Err(error) => record_parse_failure(
                &mut session.snapshot.disk.quality,
                session.snapshot.disk.total_bytes.is_some(),
                error,
                sampled_at_ms,
                collection_duration_ms,
            ),
        }
    }
    if mask.network {
        match parsed.network {
            Ok(current) => {
                let rates =
                    session
                        .network_baseline
                        .as_ref()
                        .and_then(|(previous, previous_at)| {
                            let interval = sampled_at
                                .saturating_duration_since(*previous_at)
                                .as_secs_f64();
                            network_rates(previous, &current, interval)
                        });
                session.network_baseline = Some((current.clone(), sampled_at));
                let interfaces = current
                    .iter()
                    .map(|(name, counters)| {
                        let rates = rates
                            .as_ref()
                            .and_then(|(rates, _, _)| rates.get(name).copied());
                        MonitorNetworkInterface {
                            name: name.clone(),
                            received_bytes: DecimalU64(counters.received_bytes),
                            transmitted_bytes: DecimalU64(counters.transmitted_bytes),
                            received_bytes_per_second: rates.map(|(received, _)| received),
                            transmitted_bytes_per_second: rates.map(|(_, transmitted)| transmitted),
                        }
                    })
                    .collect();
                session.snapshot.network.interfaces = interfaces;
                session.snapshot.network.received_bytes_per_second =
                    rates.as_ref().map(|(_, receive, _)| *receive);
                session.snapshot.network.transmitted_bytes_per_second =
                    rates.as_ref().map(|(_, _, transmit)| *transmit);
                set_quality(
                    &mut session.snapshot.network.quality,
                    if rates.is_some() {
                        MonitorQualityStatus::Ok
                    } else {
                        MonitorQualityStatus::WarmingUp
                    },
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
                if let Some((_, receive, transmit)) = rates {
                    push_history(
                        session,
                        MonitorHistoryMetric::NetworkReceiveRate,
                        sampled_at_ms,
                        receive,
                    );
                    push_history(
                        session,
                        MonitorHistoryMetric::NetworkTransmitRate,
                        sampled_at_ms,
                        transmit,
                    );
                }
            }
            Err(error) => {
                session.network_baseline = None;
                record_parse_failure(
                    &mut session.snapshot.network.quality,
                    !session.snapshot.network.interfaces.is_empty(),
                    error,
                    sampled_at_ms,
                    collection_duration_ms,
                );
            }
        }
    }
    if mask.uptime {
        match parsed.uptime_seconds {
            Ok(seconds) => {
                session.snapshot.uptime.seconds = Some(DecimalU64(seconds));
                set_quality(
                    &mut session.snapshot.uptime.quality,
                    MonitorQualityStatus::Ok,
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
            }
            Err(error) => record_parse_failure(
                &mut session.snapshot.uptime.quality,
                session.snapshot.uptime.seconds.is_some(),
                error,
                sampled_at_ms,
                collection_duration_ms,
            ),
        }
    }
    if mask.system {
        match parsed.system {
            Ok(system) => {
                session.snapshot.system.hostname = Some(system.hostname);
                session.snapshot.system.os = system.os;
                session.snapshot.system.kernel = Some(system.kernel);
                session.snapshot.system.architecture = Some(system.architecture);
                set_quality(
                    &mut session.snapshot.system.quality,
                    MonitorQualityStatus::Ok,
                    sampled_at_ms,
                    collection_duration_ms,
                    None,
                );
            }
            Err(error) => record_parse_failure(
                &mut session.snapshot.system.quality,
                session.snapshot.system.hostname.is_some(),
                error,
                sampled_at_ms,
                collection_duration_ms,
            ),
        }
    }
}

fn set_quality(
    quality: &mut MonitorMetricQuality,
    status: MonitorQualityStatus,
    sampled_at_ms: i64,
    collection_duration_ms: u64,
    error_code: Option<&str>,
) {
    quality.status = status;
    if status != MonitorQualityStatus::Stale {
        quality.sampled_at_ms = Some(sampled_at_ms);
    }
    quality.collection_duration_ms = Some(collection_duration_ms);
    quality.error_code = error_code.map(str::to_owned);
}

fn record_parse_failure(
    quality: &mut MonitorMetricQuality,
    has_value: bool,
    error: MonitorParseError,
    sampled_at_ms: i64,
    collection_duration_ms: u64,
) {
    let (status, error_code) = match error {
        MonitorParseError::Unsupported => {
            (MonitorQualityStatus::Unsupported, "MONITOR_UNSUPPORTED")
        }
        MonitorParseError::Invalid if has_value => {
            (MonitorQualityStatus::Stale, "MONITOR_METRIC_INVALID")
        }
        MonitorParseError::Invalid => (MonitorQualityStatus::Error, "MONITOR_METRIC_INVALID"),
    };
    set_quality(
        quality,
        status,
        sampled_at_ms,
        collection_duration_ms,
        Some(error_code),
    );
}

fn record_collection_failure(session: &mut MonitorSession, mask: CollectionMask, error: &AppError) {
    let sampled_at_ms = crate::storage::now_ms().unwrap_or_default();
    let duration = 0;
    let code = error.code.as_str();
    let stale = |quality: &mut MonitorMetricQuality, has_value: bool| {
        set_quality(
            quality,
            if has_value {
                MonitorQualityStatus::Stale
            } else {
                MonitorQualityStatus::Error
            },
            sampled_at_ms,
            duration,
            Some(code),
        );
    };
    if mask.cpu {
        stale(
            &mut session.snapshot.cpu.quality,
            session.snapshot.cpu.usage_percent.is_some(),
        );
        session.cpu_baseline = None;
    }
    if mask.memory {
        stale(
            &mut session.snapshot.memory.quality,
            session.snapshot.memory.total_bytes.is_some(),
        );
    }
    if mask.disk {
        stale(
            &mut session.snapshot.disk.quality,
            session.snapshot.disk.total_bytes.is_some(),
        );
    }
    if mask.network {
        stale(
            &mut session.snapshot.network.quality,
            !session.snapshot.network.interfaces.is_empty(),
        );
        session.network_baseline = None;
    }
    if mask.load {
        stale(
            &mut session.snapshot.load.quality,
            session.snapshot.load.one_minute.is_some(),
        );
    }
    if mask.uptime {
        stale(
            &mut session.snapshot.uptime.quality,
            session.snapshot.uptime.seconds.is_some(),
        );
    }
    if mask.system {
        stale(
            &mut session.snapshot.system.quality,
            session.snapshot.system.hostname.is_some(),
        );
        session.last_system = None;
    }
    if error.code != ErrorCode::Cancelled {
        session.consecutive_failures = session.consecutive_failures.saturating_add(1);
        let retry_seconds = match session.consecutive_failures {
            0..=1 => 5,
            2 => 15,
            _ => 30,
        };
        session.retry_at = Some(Instant::now() + Duration::from_secs(retry_seconds));
    }
}

fn push_history(
    session: &mut MonitorSession,
    metric: MonitorHistoryMetric,
    sampled_at_ms: i64,
    value: f64,
) {
    if !value.is_finite() || sampled_at_ms < 0 {
        return;
    }
    let samples = session.history.entry(metric).or_default();
    samples.push_back(MonitorHistorySample {
        sampled_at_ms,
        value,
    });
    let cutoff = sampled_at_ms.saturating_sub(DEFAULT_HISTORY_WINDOW.as_millis() as i64);
    while samples.len() > MAX_HISTORY_SAMPLES
        || samples
            .front()
            .is_some_and(|sample| sample.sampled_at_ms < cutoff)
    {
        samples.pop_front();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cpu_calculation_excludes_guest_and_rejects_counter_regression() {
        let first = parse_cpu_counters("cpu  100 10 20 200 5 3 2 1 50 20\ncpu0 0\ncpu1 0\n")
            .expect("parse first CPU sample");
        let second = parse_cpu_counters("cpu  140 15 30 240 10 4 3 2 70 30\ncpu0 0\ncpu1 0\n")
            .expect("parse second CPU sample");
        assert_eq!(first.total, 341);
        assert_eq!(first.logical_cores, Some(2));
        let usage = cpu_usage_percent(first, second).expect("valid CPU counter delta");
        assert!((usage - 58.0 * 100.0 / 103.0).abs() < 0.0001);

        let regressed = CpuCounters { total: 1, ..second };
        assert_eq!(cpu_usage_percent(second, regressed), None);
    }

    #[test]
    fn memory_requires_mem_available_and_reports_exact_byte_counts() {
        let parsed = parse_memory_counters("MemTotal: 1000 kB\nMemAvailable: 250 kB\n")
            .expect("parse memory counters");
        assert_eq!(parsed.total_bytes, 1_024_000);
        assert_eq!(parsed.available_bytes, 256_000);
        assert_eq!(parsed.used_bytes, 768_000);
        assert_eq!(
            parse_memory_counters("MemTotal: 1000 kB\n"),
            Err(MonitorParseError::Unsupported)
        );
    }

    #[test]
    fn disk_parser_uses_root_row_values_and_handles_spaces_in_mount_field() {
        let parsed = parse_disk_counters(
            "Filesystem 1024-blocks Used Available Capacity Mounted on\n/dev/disk 1000 650 350 66% /\n",
        )
        .expect("parse root disk row");
        assert_eq!(parsed.total_bytes, 1_024_000);
        assert_eq!(parsed.used_bytes, 665_600);
        assert_eq!(parsed.available_bytes, 358_400);
        assert_eq!(parsed.mount, "/");

        let bsd = parse_disk_counters(
            "Filesystem 1024-blocks Used Available Capacity iused ifree %iused Mounted on\n/dev/disk 1000 650 350 66% 10 20 33% /\n",
        )
        .expect("parse root disk row with BSD inode columns");
        assert_eq!(bsd.mount, "/");
        assert_eq!(bsd.total_bytes, 1_024_000);
    }

    #[test]
    fn network_rates_reset_on_interface_changes_or_counter_wrap_and_ignore_loopback_totals() {
        let previous = BTreeMap::from([
            (
                "eth0".to_owned(),
                NetworkCounters {
                    received_bytes: 100,
                    transmitted_bytes: 200,
                },
            ),
            (
                "lo".to_owned(),
                NetworkCounters {
                    received_bytes: 10,
                    transmitted_bytes: 20,
                },
            ),
        ]);
        let current = BTreeMap::from([
            (
                "eth0".to_owned(),
                NetworkCounters {
                    received_bytes: 500,
                    transmitted_bytes: 800,
                },
            ),
            (
                "lo".to_owned(),
                NetworkCounters {
                    received_bytes: 90,
                    transmitted_bytes: 120,
                },
            ),
        ]);
        let (_, receive, transmit) =
            network_rates(&previous, &current, 2.0).expect("compute network rates");
        assert_eq!(receive, 200.0);
        assert_eq!(transmit, 300.0);

        let changed = BTreeMap::from([("eth1".to_owned(), current["eth0"])]);
        assert_eq!(network_rates(&previous, &changed, 1.0), None);
        assert_eq!(network_rates(&current, &previous, 1.0), None);
    }

    #[test]
    fn parser_keeps_os_release_text_literal_and_missing_memavailable_unsupported() {
        let output = "@@SYSTEM@@\nLinux host-a 6.8.0 x86_64\nPRETTY_NAME=\"Linux $(touch /tmp/should-not-run)\"\n@@END_SYSTEM@@\n@@MEMORY@@\nMemTotal: 8 kB\n@@END_MEMORY@@\n";
        let parsed = parse_monitor_output(output);
        assert_eq!(
            parsed.system.expect("system information").os.as_deref(),
            Some("Linux $(touch /tmp/should-not-run)")
        );
        assert_eq!(parsed.memory, Err(MonitorParseError::Unsupported));
    }

    #[test]
    fn collection_script_contains_only_fixed_commands_and_selected_sections() {
        let script = fixed_collection_script(CollectionMask {
            cpu: true,
            uptime: true,
            ..CollectionMask::default()
        });
        assert!(script.contains("@@CPU@@"));
        assert!(script.contains("@@UPTIME@@"));
        assert!(!script.contains("@@MEMORY@@"));
        assert!(!script.contains("eval"));
    }

    #[test]
    fn monitor_fixtures_cover_linux_proc_and_posix_disk_variants() {
        for fixture in [
            include_str!("../tests/fixtures/monitor/debian-12.txt"),
            include_str!("../tests/fixtures/monitor/alpine-3.txt"),
        ] {
            let parsed = parse_monitor_output(fixture);
            assert!(parsed.cpu.is_ok());
            assert!(parsed.memory.is_ok());
            assert!(parsed.disk.is_ok());
            assert!(parsed.network.is_ok());
            assert!(parsed.load.is_ok());
            assert!(parsed.uptime_seconds.is_ok());
            assert!(parsed.system.is_ok());
        }
    }

    #[test]
    fn failed_static_collection_retries_without_replacing_last_sample_time() {
        let mut session = MonitorSession::new("connection");
        session.snapshot.system.hostname = Some("host-a".to_owned());
        session.snapshot.system.quality.status = MonitorQualityStatus::Ok;
        session.snapshot.system.quality.sampled_at_ms = Some(1234);
        session.last_system = Some(Instant::now());
        let error = monitor_error(ErrorCode::MonitorTimeout, "errors.monitorTimeout");

        record_collection_failure(
            &mut session,
            CollectionMask {
                system: true,
                ..CollectionMask::default()
            },
            &error,
        );

        assert_eq!(session.last_system, None);
        assert_eq!(
            session.snapshot.system.quality.status,
            MonitorQualityStatus::Stale
        );
        assert_eq!(session.snapshot.system.quality.sampled_at_ms, Some(1234));
        assert_eq!(
            session.snapshot.system.quality.error_code.as_deref(),
            Some("MONITOR_TIMEOUT")
        );
        assert!(session.retry_at.is_some());
    }

    #[test]
    fn monitor_history_enforces_sample_cap_and_rejects_non_finite_values() {
        let mut session = MonitorSession::new("connection");
        for timestamp in 0..=MAX_HISTORY_SAMPLES as i64 {
            push_history(
                &mut session,
                MonitorHistoryMetric::CpuUsage,
                timestamp,
                timestamp as f64,
            );
        }
        push_history(&mut session, MonitorHistoryMetric::CpuUsage, 1000, f64::NAN);
        let samples = session
            .history
            .get(&MonitorHistoryMetric::CpuUsage)
            .expect("CPU history");
        assert_eq!(samples.len(), MAX_HISTORY_SAMPLES);
        assert_eq!(
            samples
                .front()
                .expect("oldest retained sample")
                .sampled_at_ms,
            1
        );
        assert_eq!(samples.back().expect("newest sample").sampled_at_ms, 900);
    }

    #[test]
    fn sampling_policy_uses_foreground_background_and_static_intervals() {
        let now = Instant::now();
        let mut session = MonitorSession::new("connection");
        session.last_cpu = Some(now - Duration::from_secs(2));
        session.last_network = Some(now - Duration::from_secs(2));
        session.last_load = Some(now - Duration::from_secs(2));
        session.last_uptime = Some(now - Duration::from_secs(2));
        session.last_memory = Some(now - Duration::from_secs(1));
        session.last_disk = Some(now - Duration::from_secs(4));

        let foreground = due_mask_for_session(&session, true, now);
        assert!(foreground.cpu && foreground.network && foreground.load && foreground.uptime);
        assert!(!foreground.memory && !foreground.disk);
        assert!(foreground.system);

        session.last_memory = Some(now - Duration::from_secs(2));
        session.last_disk = Some(now - Duration::from_secs(5));
        let foreground = due_mask_for_session(&session, true, now);
        assert!(foreground.memory && foreground.disk);

        session.last_cpu = Some(now - Duration::from_secs(9));
        session.last_network = Some(now - Duration::from_secs(10));
        session.last_load = Some(now - Duration::from_secs(10));
        session.last_uptime = Some(now - Duration::from_secs(10));
        session.last_memory = Some(now - Duration::from_secs(10));
        session.last_disk = Some(now - Duration::from_secs(29));
        session.snapshot.cpu.quality.status = MonitorQualityStatus::Unsupported;
        let background = due_mask_for_session(&session, false, now);
        assert!(!background.cpu);
        assert!(background.network && background.load && background.uptime && background.memory);
        assert!(!background.disk);
        assert!(background.system);
    }

    #[test]
    fn sampling_policy_skips_in_flight_work_and_applies_retry_backoff() {
        let now = Instant::now();
        let mut session = MonitorSession::new("connection");
        session.in_flight = true;
        assert!(!due_mask_for_session(&session, true, now).has_any());
        session.in_flight = false;
        session.retry_at = Some(now + Duration::from_secs(5));
        assert!(!due_mask_for_session(&session, true, now).has_any());

        let error = monitor_error(ErrorCode::MonitorTimeout, "errors.monitorTimeout");
        for expected_seconds in [5, 15, 30] {
            record_collection_failure(
                &mut session,
                CollectionMask {
                    cpu: true,
                    ..CollectionMask::default()
                },
                &error,
            );
            let remaining = session
                .retry_at
                .expect("retry schedule")
                .saturating_duration_since(Instant::now());
            assert!(remaining <= Duration::from_secs(expected_seconds));
            assert!(remaining > Duration::from_secs(expected_seconds - 1));
        }

        session.retry_at = None;
        let cancelled = monitor_error(ErrorCode::Cancelled, "errors.cancelled");
        record_collection_failure(
            &mut session,
            CollectionMask {
                cpu: true,
                ..CollectionMask::default()
            },
            &cancelled,
        );
        assert_eq!(session.retry_at, None);
        assert_eq!(session.consecutive_failures, 3);
    }
}
