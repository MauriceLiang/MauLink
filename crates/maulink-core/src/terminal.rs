use std::{
    collections::{HashMap, VecDeque},
    sync::{Arc, Mutex as StdMutex, MutexGuard as StdMutexGuard},
    time::Duration,
};

use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64};
use russh::{ChannelMsg, client};
use serde::{Deserialize, Serialize};
use tokio::sync::{Mutex, Notify, Semaphore, mpsc};
use tokio_util::sync::CancellationToken;
use ts_rs::TS;
use uuid::Uuid;

use crate::{
    AppError, ConnectionState, DecimalU64, ErrorCode, SshConnectionManager, TerminalAckPayload,
    TerminalIdPayload, TerminalOpenPayload, TerminalResizePayload, TerminalWritePayload, storage,
};

const MAX_TERMINALS_PER_CONNECTION: usize = 8;
const MAX_ACTIVE_TERMINALS: usize = 32;
const TERMINAL_HISTORY_CAPACITY: usize = 100;
const OUTPUT_CHUNK_BYTES: usize = 32 * 1024;
const OUTPUT_IN_FLIGHT_BYTES: usize = 128 * 1024;
const OUTPUT_BUFFER_BYTES: usize = 512 * 1024;
const OUTPUT_EVENT_CAPACITY: usize = 4;
const INPUT_REQUEST_BYTES: usize = 64 * 1024;
const INPUT_QUEUE_BYTES: usize = 256 * 1024;
const CONSUMER_STALL_TIMEOUT: Duration = Duration::from_secs(15);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub enum TerminalState {
    Opening,
    Running,
    Closing,
    Closed,
    Failed,
}

impl TerminalState {
    const fn is_terminal(self) -> bool {
        matches!(self, Self::Closed | Self::Failed)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSize {
    pub columns: u32,
    pub rows: u32,
    pub pixel_width: Option<u32>,
    pub pixel_height: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalSnapshot {
    pub terminal_id: String,
    pub connection_id: String,
    pub stream_id: String,
    pub state: TerminalState,
    pub size: TerminalSize,
    pub exit_status: Option<u32>,
    pub exit_signal: Option<String>,
    pub error: Option<AppError>,
    #[ts(type = "number")]
    pub created_at_ms: i64,
    #[ts(type = "number")]
    pub updated_at_ms: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalOpenResult {
    pub terminal_id: String,
    pub stream_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalChunk {
    pub terminal_id: String,
    pub stream_id: String,
    pub seq: DecimalU64,
    pub byte_length: u32,
    pub data_base64: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct TerminalWriteResult {
    pub input_seq: DecimalU64,
    pub duplicate: bool,
}

#[derive(Clone)]
pub struct TerminalManager {
    connections: SshConnectionManager,
    registry: Arc<StdMutex<TerminalRegistry>>,
    open_lock: Arc<Mutex<()>>,
}

struct TerminalRegistry {
    entries: HashMap<String, Arc<Terminal>>,
    order: VecDeque<String>,
}

struct Terminal {
    snapshot: StdMutex<TerminalSnapshot>,
    writer: Mutex<Option<russh::ChannelWriteHalf<client::Msg>>>,
    output: OutputWindow,
    input: Mutex<InputState>,
    input_budget: Arc<Semaphore>,
    cancellation: CancellationToken,
}

struct InputState {
    next_seq: u64,
}

#[derive(Clone)]
struct OutputWindow {
    state: Arc<Mutex<OutputState>>,
    changed: Arc<Notify>,
}

struct OutputState {
    next_seq: u64,
    acked_seq: u64,
    highest_sent_seq: u64,
    in_flight_bytes: usize,
    chunks: VecDeque<(u64, usize)>,
}

impl TerminalManager {
    pub fn new(connections: SshConnectionManager) -> Self {
        Self {
            connections,
            registry: Arc::new(StdMutex::new(TerminalRegistry {
                entries: HashMap::new(),
                order: VecDeque::new(),
            })),
            open_lock: Arc::new(Mutex::new(())),
        }
    }

    pub async fn open(
        &self,
        payload: TerminalOpenPayload,
    ) -> Result<(TerminalOpenResult, mpsc::Receiver<TerminalChunk>), AppError> {
        let size = validate_size(
            payload.columns,
            payload.rows,
            payload.pixel_width,
            payload.pixel_height,
        )?;
        let _open_guard = self.open_lock.lock().await;
        let connection = self.connections.get(&payload.connection_id)?;
        if connection.state != ConnectionState::Ready {
            return Err(validation("connectionId", "errors.connectionNotReady"));
        }
        self.ensure_capacity(&payload.connection_id)?;
        let (reader, writer) = self
            .connections
            .open_terminal_channel(
                &payload.connection_id,
                size.columns,
                size.rows,
                size.pixel_width.unwrap_or(0),
                size.pixel_height.unwrap_or(0),
            )
            .await?;
        let now = storage::now_ms()?;
        let terminal_id = Uuid::new_v4().to_string();
        let stream_id = Uuid::new_v4().to_string();
        let terminal = Arc::new(Terminal {
            snapshot: StdMutex::new(TerminalSnapshot {
                terminal_id: terminal_id.clone(),
                connection_id: payload.connection_id,
                stream_id: stream_id.clone(),
                state: TerminalState::Running,
                size,
                exit_status: None,
                exit_signal: None,
                error: None,
                created_at_ms: now,
                updated_at_ms: now,
            }),
            writer: Mutex::new(Some(writer)),
            output: OutputWindow::new(),
            input: Mutex::new(InputState { next_seq: 1 }),
            input_budget: Arc::new(Semaphore::new(INPUT_QUEUE_BYTES)),
            cancellation: CancellationToken::new(),
        });
        if let Err(error) = self.insert(terminal.clone()) {
            terminal.close_writer().await;
            return Err(error);
        }
        let (sender, receiver) = mpsc::channel(OUTPUT_EVENT_CAPACITY);
        let connections = self.connections.clone();
        tokio::spawn(run_output(reader, terminal, sender, connections));
        Ok((
            TerminalOpenResult {
                terminal_id,
                stream_id,
            },
            receiver,
        ))
    }

    pub fn get(&self, payload: &TerminalIdPayload) -> Result<TerminalSnapshot, AppError> {
        Ok(self.terminal(&payload.terminal_id)?.snapshot()?.clone())
    }

    pub async fn ack(&self, payload: TerminalAckPayload) -> Result<(), AppError> {
        let terminal = self.terminal(&payload.terminal_id)?;
        let stream_matches = {
            let snapshot = terminal.snapshot()?;
            snapshot.stream_id == payload.stream_id
        };
        if !stream_matches {
            return Err(AppError::new(
                ErrorCode::TerminalAckInvalid,
                "errors.terminalStreamMismatch",
            ));
        }
        terminal.output.ack(payload.seq.0).await
    }

    pub async fn write(
        &self,
        payload: TerminalWritePayload,
    ) -> Result<TerminalWriteResult, AppError> {
        let terminal = self.terminal(&payload.terminal_id)?;
        {
            let input = terminal.input.lock().await;
            if payload.input_seq.0 < input.next_seq {
                return Ok(TerminalWriteResult {
                    input_seq: payload.input_seq,
                    duplicate: true,
                });
            }
            if payload.input_seq.0 != input.next_seq {
                return Err(input_sequence_invalid(input.next_seq, payload.input_seq.0));
            }
        }
        terminal.ensure_running()?;
        let max_encoded_length =
            base64::encoded_len(INPUT_REQUEST_BYTES, true).unwrap_or(usize::MAX);
        if payload.data_base64.len() > max_encoded_length {
            return Err(validation("dataBase64", "errors.terminalInputTooLarge"));
        }
        let data = BASE64
            .decode(payload.data_base64.as_bytes())
            .map_err(|_| validation("dataBase64", "errors.base64Invalid"))?;
        if data.len() > INPUT_REQUEST_BYTES {
            return Err(validation("dataBase64", "errors.terminalInputTooLarge"));
        }
        let permits = u32::try_from(data.len())
            .map_err(|_| validation("dataBase64", "errors.terminalInputTooLarge"))?;
        let permit = terminal
            .input_budget
            .clone()
            .try_acquire_many_owned(permits)
            .map_err(|_| {
                AppError::new(ErrorCode::InputBackpressure, "errors.inputBackpressure").with_retry()
            })?;
        let mut input = terminal.input.lock().await;
        if payload.input_seq.0 < input.next_seq {
            return Ok(TerminalWriteResult {
                input_seq: payload.input_seq,
                duplicate: true,
            });
        }
        if payload.input_seq.0 != input.next_seq {
            return Err(input_sequence_invalid(input.next_seq, payload.input_seq.0));
        }
        let next_seq = input.next_seq.checked_add(1).ok_or_else(|| {
            AppError::new(
                ErrorCode::ResourceLimit,
                "errors.terminalInputSequenceExhausted",
            )
        })?;
        let writer_guard = terminal.writer.lock().await;
        let writer = writer_guard.as_ref().ok_or_else(terminal_closed)?;
        if let Err(_error) = writer.data_bytes(data).await {
            drop(writer_guard);
            terminal.fail(connection_lost("writingTerminal"))?;
            return Err(connection_lost("writingTerminal"));
        }
        input.next_seq = next_seq;
        drop(permit);
        Ok(TerminalWriteResult {
            input_seq: payload.input_seq,
            duplicate: false,
        })
    }

    pub async fn resize(&self, payload: TerminalResizePayload) -> Result<TerminalSize, AppError> {
        let terminal = self.terminal(&payload.terminal_id)?;
        terminal.ensure_running()?;
        let size = validate_size(
            payload.columns,
            payload.rows,
            payload.pixel_width,
            payload.pixel_height,
        )?;
        let writer_guard = terminal.writer.lock().await;
        let writer = writer_guard.as_ref().ok_or_else(terminal_closed)?;
        let result = writer
            .window_change(
                size.columns,
                size.rows,
                size.pixel_width.unwrap_or(0),
                size.pixel_height.unwrap_or(0),
            )
            .await;
        if result.is_err() {
            drop(writer_guard);
            let error = connection_lost("resizingTerminal");
            terminal.fail(error.clone())?;
            return Err(error);
        }
        drop(writer_guard);
        terminal.update_snapshot(|snapshot| snapshot.size = size)?;
        Ok(size)
    }

    pub async fn close(&self, payload: &TerminalIdPayload) -> Result<TerminalSnapshot, AppError> {
        let terminal = self.terminal(&payload.terminal_id)?;
        {
            let mut snapshot = terminal.snapshot()?;
            if snapshot.state.is_terminal() {
                return Ok(snapshot.clone());
            }
            snapshot.state = TerminalState::Closing;
            snapshot.updated_at_ms = storage::now_ms()?;
        }
        terminal.cancellation.cancel();
        if let Some(writer) = terminal.writer.lock().await.take() {
            let _ = writer.eof().await;
            let _ = writer.close().await;
        }
        terminal.update_snapshot(|snapshot| snapshot.state = TerminalState::Closed)?;
        Ok(terminal.snapshot()?.clone())
    }

    pub async fn shutdown(&self) {
        let terminals = self
            .registry
            .lock()
            .map(|registry| registry.entries.values().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        for terminal in terminals {
            terminal.cancellation.cancel();
            if let Some(writer) = terminal.writer.lock().await.take() {
                let _ = writer.eof().await;
                let _ = writer.close().await;
            }
            let _ = terminal.update_snapshot(|snapshot| {
                if !snapshot.state.is_terminal() {
                    snapshot.state = TerminalState::Closed;
                }
            });
        }
    }

    fn ensure_capacity(&self, connection_id: &str) -> Result<(), AppError> {
        let registry = self.registry()?;
        let mut active = 0;
        let mut connection_active = 0;
        for terminal in registry.entries.values() {
            let snapshot = terminal.snapshot()?;
            if snapshot.state.is_terminal() {
                continue;
            }
            active += 1;
            if snapshot.connection_id == connection_id {
                connection_active += 1;
            }
        }
        if active >= MAX_ACTIVE_TERMINALS {
            return Err(
                AppError::new(ErrorCode::ResourceLimit, "errors.terminalLimitReached")
                    .with_param("limit", MAX_ACTIVE_TERMINALS.to_string()),
            );
        }
        if connection_active >= MAX_TERMINALS_PER_CONNECTION {
            return Err(AppError::new(
                ErrorCode::ResourceLimit,
                "errors.connectionTerminalLimitReached",
            )
            .with_param("limit", MAX_TERMINALS_PER_CONNECTION.to_string()));
        }
        Ok(())
    }

    fn insert(&self, terminal: Arc<Terminal>) -> Result<(), AppError> {
        let terminal_id = terminal.snapshot()?.terminal_id.clone();
        let mut registry = self.registry()?;
        while registry.entries.len() >= TERMINAL_HISTORY_CAPACITY {
            let removable = registry.order.iter().position(|candidate| {
                registry
                    .entries
                    .get(candidate)
                    .and_then(|terminal| terminal.snapshot().ok())
                    .is_some_and(|snapshot| snapshot.state.is_terminal())
            });
            let Some(position) = removable else {
                break;
            };
            if let Some(candidate) = registry.order.remove(position) {
                registry.entries.remove(&candidate);
            }
        }
        if registry.entries.len() >= TERMINAL_HISTORY_CAPACITY {
            return Err(AppError::new(
                ErrorCode::ResourceLimit,
                "errors.terminalHistoryLimitReached",
            ));
        }
        registry.order.push_back(terminal_id.clone());
        registry.entries.insert(terminal_id, terminal);
        Ok(())
    }

    fn terminal(&self, terminal_id: &str) -> Result<Arc<Terminal>, AppError> {
        validate_uuid(terminal_id, "terminalId")?;
        self.registry()?
            .entries
            .get(terminal_id)
            .cloned()
            .ok_or_else(terminal_not_found)
    }

    fn registry(&self) -> Result<StdMutexGuard<'_, TerminalRegistry>, AppError> {
        self.registry
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal, "errors.terminalRegistryUnavailable"))
    }
}

impl Terminal {
    fn snapshot(&self) -> Result<StdMutexGuard<'_, TerminalSnapshot>, AppError> {
        self.snapshot
            .lock()
            .map_err(|_| AppError::new(ErrorCode::Internal, "errors.terminalStateUnavailable"))
    }

    fn ensure_running(&self) -> Result<(), AppError> {
        if self.snapshot()?.state != TerminalState::Running {
            return Err(terminal_closed());
        }
        Ok(())
    }

    fn update_snapshot(&self, update: impl FnOnce(&mut TerminalSnapshot)) -> Result<(), AppError> {
        let mut snapshot = self.snapshot()?;
        update(&mut snapshot);
        snapshot.updated_at_ms = storage::now_ms()?;
        Ok(())
    }

    fn fail(&self, error: AppError) -> Result<(), AppError> {
        self.cancellation.cancel();
        self.update_snapshot(|snapshot| {
            if !snapshot.state.is_terminal() {
                snapshot.state = TerminalState::Failed;
                snapshot.error = Some(error);
            }
        })
    }

    async fn close_writer(&self) {
        if let Some(writer) = self.writer.lock().await.take() {
            let _ = writer.eof().await;
            let _ = writer.close().await;
        }
    }
}

impl OutputWindow {
    fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(OutputState {
                next_seq: 1,
                acked_seq: 0,
                highest_sent_seq: 0,
                in_flight_bytes: 0,
                chunks: VecDeque::new(),
            })),
            changed: Arc::new(Notify::new()),
        }
    }

    #[cfg(test)]
    async fn reserve(&self, byte_length: usize) -> Result<u64, AppError> {
        self.reserve_with_timeout(byte_length, CONSUMER_STALL_TIMEOUT)
            .await
    }

    #[cfg(test)]
    async fn reserve_with_timeout(
        &self,
        byte_length: usize,
        timeout: Duration,
    ) -> Result<u64, AppError> {
        let deadline = tokio::time::Instant::now() + timeout;
        loop {
            let notified = self.changed.notified();
            if let Some(seq) = self.try_reserve(byte_length).await? {
                return Ok(seq);
            }
            tokio::time::timeout_at(deadline, notified)
                .await
                .map_err(|_| consumer_stalled())?;
        }
    }

    async fn try_reserve(&self, byte_length: usize) -> Result<Option<u64>, AppError> {
        let mut state = self.state.lock().await;
        if state.in_flight_bytes + byte_length > OUTPUT_IN_FLIGHT_BYTES {
            return Ok(None);
        }
        let seq = state.next_seq;
        state.next_seq = state.next_seq.checked_add(1).ok_or_else(|| {
            AppError::new(ErrorCode::ResourceLimit, "errors.terminalOutputExhausted")
        })?;
        state.highest_sent_seq = seq;
        state.in_flight_bytes += byte_length;
        state.chunks.push_back((seq, byte_length));
        Ok(Some(seq))
    }

    async fn ack_state(&self) -> (u64, usize) {
        let state = self.state.lock().await;
        (state.acked_seq, state.in_flight_bytes)
    }

    async fn ack(&self, seq: u64) -> Result<(), AppError> {
        ack_locked(self.state.lock().await, seq)?;
        self.changed.notify_one();
        Ok(())
    }
}

fn ack_locked(
    mut state: tokio::sync::MutexGuard<'_, OutputState>,
    seq: u64,
) -> Result<(), AppError> {
    if seq > state.highest_sent_seq {
        return Err(
            AppError::new(ErrorCode::TerminalAckInvalid, "errors.terminalAckAhead")
                .with_param("highestSent", state.highest_sent_seq.to_string())
                .with_param("actual", seq.to_string()),
        );
    }
    if seq <= state.acked_seq {
        return Ok(());
    }
    while state
        .chunks
        .front()
        .is_some_and(|(chunk_seq, _)| *chunk_seq <= seq)
    {
        if let Some((_, bytes)) = state.chunks.pop_front() {
            state.in_flight_bytes -= bytes;
        }
    }
    state.acked_seq = seq;
    Ok(())
}

async fn run_output(
    mut reader: russh::ChannelReadHalf,
    terminal: Arc<Terminal>,
    sender: mpsc::Sender<TerminalChunk>,
    connections: SshConnectionManager,
) {
    let mut pending = VecDeque::<Vec<u8>>::new();
    let mut pending_bytes = 0;
    let mut reader_closed = false;
    let mut last_acked_seq = 0;
    let mut stalled_at = None;

    loop {
        if sender.is_closed() {
            let _ = terminal.fail(consumer_stalled());
            terminal.close_writer().await;
            return;
        }

        while !pending.is_empty() {
            let permit = match sender.try_reserve() {
                Ok(permit) => permit,
                Err(mpsc::error::TrySendError::Full(())) => break,
                Err(mpsc::error::TrySendError::Closed(())) => {
                    let _ = terminal.fail(consumer_stalled());
                    terminal.close_writer().await;
                    return;
                }
            };
            match send_pending_chunk(&terminal, &mut pending, &mut pending_bytes, permit).await {
                Ok(true) => {}
                Ok(false) => break,
                Err(error) => {
                    let _ = terminal.fail(error);
                    terminal.close_writer().await;
                    return;
                }
            }
        }

        if reader_closed && pending.is_empty() {
            let connection_error = terminal
                .snapshot()
                .ok()
                .and_then(|snapshot| connections.get(&snapshot.connection_id).ok())
                .filter(|snapshot| snapshot.state == ConnectionState::Failed)
                .and_then(|snapshot| snapshot.error);
            if let Some(error) = connection_error {
                let _ = terminal.fail(error);
            } else {
                let _ = terminal.update_snapshot(|snapshot| {
                    if !snapshot.state.is_terminal() {
                        snapshot.state = TerminalState::Closed;
                    }
                });
            }
            break;
        }

        let (acked_seq, in_flight_bytes) = terminal.output.ack_state().await;
        if acked_seq > last_acked_seq {
            last_acked_seq = acked_seq;
            stalled_at =
                (in_flight_bytes > 0).then(|| tokio::time::Instant::now() + CONSUMER_STALL_TIMEOUT);
        } else if in_flight_bytes > 0 && stalled_at.is_none() {
            stalled_at = Some(tokio::time::Instant::now() + CONSUMER_STALL_TIMEOUT);
        } else if in_flight_bytes == 0 {
            stalled_at = None;
        }

        let can_emit = pending
            .front()
            .is_some_and(|bytes| in_flight_bytes + bytes.len() <= OUTPUT_IN_FLIGHT_BYTES);
        let ack_changed = terminal.output.changed.notified();
        tokio::select! {
            _ = terminal.cancellation.cancelled() => break,
            _ = sender.closed() => {
                let _ = terminal.fail(consumer_stalled());
                terminal.close_writer().await;
                return;
            }
            _ = async {
                if let Some(deadline) = stalled_at {
                    tokio::time::sleep_until(deadline).await;
                } else {
                    std::future::pending::<()>().await;
                }
            } => {
                let _ = terminal.fail(consumer_stalled());
                terminal.close_writer().await;
                return;
            }
            permit = sender.reserve(), if can_emit => {
                match permit {
                    Ok(permit) => match send_pending_chunk(&terminal, &mut pending, &mut pending_bytes, permit).await {
                        Ok(true) | Ok(false) => {}
                        Err(error) => {
                            let _ = terminal.fail(error);
                            terminal.close_writer().await;
                            return;
                        }
                    },
                    Err(_) => {
                        let _ = terminal.fail(consumer_stalled());
                        terminal.close_writer().await;
                        return;
                    }
                }
            }
            _ = ack_changed => {}
            message = reader.wait(), if !reader_closed => {
                match message {
                    Some(ChannelMsg::Data { data }) | Some(ChannelMsg::ExtendedData { data, .. }) => {
                        let (acked_seq, in_flight_bytes) = terminal.output.ack_state().await;
                        last_acked_seq = last_acked_seq.max(acked_seq);
                        for bytes in output_chunks(&data) {
                            if pending_bytes + in_flight_bytes + bytes.len() > OUTPUT_BUFFER_BYTES {
                                let _ = terminal.fail(consumer_stalled());
                                terminal.close_writer().await;
                                return;
                            }
                            pending.push_back(bytes.to_vec());
                            pending_bytes += bytes.len();
                        }
                    }
                    Some(ChannelMsg::ExitStatus { exit_status }) => {
                        let _ = terminal.update_snapshot(|snapshot| {
                            snapshot.exit_status = Some(exit_status);
                        });
                    }
                    Some(ChannelMsg::ExitSignal { signal_name, .. }) => {
                        let _ = terminal.update_snapshot(|snapshot| {
                            snapshot.exit_signal = Some(signal_name_text(signal_name));
                        });
                    }
                    Some(ChannelMsg::Close) | None => reader_closed = true,
                    Some(ChannelMsg::Eof) | Some(_) => {}
                }
            }
        }
    }
}

async fn send_pending_chunk(
    terminal: &Terminal,
    pending: &mut VecDeque<Vec<u8>>,
    pending_bytes: &mut usize,
    permit: mpsc::Permit<'_, TerminalChunk>,
) -> Result<bool, AppError> {
    let Some(bytes) = pending.front() else {
        return Ok(false);
    };
    let Some(seq) = terminal.output.try_reserve(bytes.len()).await? else {
        return Ok(false);
    };
    let bytes = pending.pop_front().expect("pending terminal chunk");
    *pending_bytes -= bytes.len();
    let snapshot = terminal.snapshot()?.clone();
    permit.send(TerminalChunk {
        terminal_id: snapshot.terminal_id,
        stream_id: snapshot.stream_id,
        seq: seq.into(),
        byte_length: bytes.len() as u32,
        data_base64: BASE64.encode(bytes),
    });
    Ok(true)
}

fn validate_size(
    columns: u32,
    rows: u32,
    pixel_width: Option<u32>,
    pixel_height: Option<u32>,
) -> Result<TerminalSize, AppError> {
    if !(1..=1_000).contains(&columns) {
        return Err(validation("columns", "errors.terminalColumnsInvalid"));
    }
    if !(1..=1_000).contains(&rows) {
        return Err(validation("rows", "errors.terminalRowsInvalid"));
    }
    if pixel_width.is_some_and(|value| value > 100_000) {
        return Err(validation("pixelWidth", "errors.terminalPixelWidthInvalid"));
    }
    if pixel_height.is_some_and(|value| value > 100_000) {
        return Err(validation(
            "pixelHeight",
            "errors.terminalPixelHeightInvalid",
        ));
    }
    Ok(TerminalSize {
        columns,
        rows,
        pixel_width,
        pixel_height,
    })
}

fn output_chunks(data: &[u8]) -> impl Iterator<Item = &[u8]> {
    data.chunks(OUTPUT_CHUNK_BYTES)
}

fn validate_uuid(value: &str, field: &'static str) -> Result<(), AppError> {
    Uuid::parse_str(value)
        .map(|_| ())
        .map_err(|_| validation(field, "errors.resourceIdInvalid"))
}

fn validation(field: &'static str, message_key: &'static str) -> AppError {
    AppError::new(ErrorCode::ValidationFailed, message_key).with_param("field", field)
}

fn terminal_not_found() -> AppError {
    AppError::new(ErrorCode::ResourceNotFound, "errors.terminalNotFound")
}

fn terminal_closed() -> AppError {
    AppError::new(ErrorCode::ResourceClosed, "errors.terminalClosed")
}

fn consumer_stalled() -> AppError {
    AppError::new(
        ErrorCode::TerminalConsumerStalled,
        "errors.terminalConsumerStalled",
    )
    .with_stage("streamingTerminal")
}

fn connection_lost(stage: &'static str) -> AppError {
    AppError::new(ErrorCode::ConnectionLost, "errors.connectionLost")
        .with_retry()
        .with_stage(stage)
}

fn input_sequence_invalid(expected: u64, actual: u64) -> AppError {
    AppError::new(
        ErrorCode::TerminalInputSequenceInvalid,
        "errors.terminalInputSequenceInvalid",
    )
    .with_param("expected", expected.to_string())
    .with_param("actual", actual.to_string())
}

fn signal_name_text(signal: russh::Sig) -> String {
    match signal {
        russh::Sig::ABRT => "ABRT".to_owned(),
        russh::Sig::ALRM => "ALRM".to_owned(),
        russh::Sig::FPE => "FPE".to_owned(),
        russh::Sig::HUP => "HUP".to_owned(),
        russh::Sig::ILL => "ILL".to_owned(),
        russh::Sig::INT => "INT".to_owned(),
        russh::Sig::KILL => "KILL".to_owned(),
        russh::Sig::PIPE => "PIPE".to_owned(),
        russh::Sig::QUIT => "QUIT".to_owned(),
        russh::Sig::SEGV => "SEGV".to_owned(),
        russh::Sig::TERM => "TERM".to_owned(),
        russh::Sig::USR1 => "USR1".to_owned(),
        russh::Sig::Custom(name) => name,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn validates_terminal_dimensions() {
        assert!(validate_size(80, 24, None, None).is_ok());
        assert_eq!(
            validate_size(0, 24, None, None)
                .expect_err("zero columns")
                .code,
            ErrorCode::ValidationFailed
        );
        assert_eq!(
            validate_size(80, 1_001, None, None)
                .expect_err("too many rows")
                .code,
            ErrorCode::ValidationFailed
        );
    }

    #[tokio::test]
    async fn output_window_requires_cumulative_ack_and_rejects_future_ack() {
        let window = OutputWindow::new();
        for expected in 1..=4 {
            assert_eq!(
                window.reserve(OUTPUT_CHUNK_BYTES).await.expect("reserve"),
                expected
            );
        }
        assert!(
            tokio::time::timeout(
                Duration::from_millis(10),
                window.reserve(OUTPUT_CHUNK_BYTES)
            )
            .await
            .is_err()
        );
        window.ack(2).await.expect("cumulative ack");
        assert_eq!(
            window.reserve(OUTPUT_CHUNK_BYTES).await.expect("reserve"),
            5
        );
        window.ack(2).await.expect("duplicate ack is idempotent");
        assert_eq!(
            window.ack(6).await.expect_err("future ack").code,
            ErrorCode::TerminalAckInvalid
        );
    }

    #[tokio::test]
    async fn output_window_times_out_a_live_consumer_that_never_acks() {
        let window = OutputWindow::new();
        for _ in 0..4 {
            window
                .reserve(OUTPUT_CHUNK_BYTES)
                .await
                .expect("fill output window");
        }
        assert_eq!(
            window
                .reserve_with_timeout(OUTPUT_CHUNK_BYTES, Duration::from_millis(10))
                .await
                .expect_err("unacknowledged output must time out")
                .code,
            ErrorCode::TerminalConsumerStalled
        );
    }

    #[test]
    fn output_chunking_preserves_utf8_and_ansi_bytes_across_boundaries() {
        let mut input = vec![b'x'; OUTPUT_CHUNK_BYTES - 2];
        input.extend_from_slice(b"\x1b[31m");
        input.extend(std::iter::repeat_n(b'y', OUTPUT_CHUNK_BYTES - 4));
        input.extend_from_slice(b"\xe4\xb8\xad\x1b[0m");
        input.extend(std::iter::repeat_n(b'y', OUTPUT_CHUNK_BYTES));
        let chunks = output_chunks(&input).collect::<Vec<_>>();
        assert_eq!(chunks[0].len(), OUTPUT_CHUNK_BYTES);
        assert_eq!(&chunks[0][OUTPUT_CHUNK_BYTES - 2..], b"\x1b[");
        assert_eq!(&chunks[1][..3], b"31m");
        assert_eq!(*chunks[1].last().expect("second chunk is non-empty"), 0xe4);
        assert_eq!(&chunks[2][..2], b"\xb8\xad");
        let reconstructed = chunks.concat();
        assert_eq!(reconstructed, input);
        assert!(std::str::from_utf8(&reconstructed).is_ok());
    }
}
