use std::net::SocketAddr;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, Manager, State};
use tokio::net::TcpListener;
use tokio_util::sync::CancellationToken;
use tokio_modbus::server::tcp::Server;

mod modbus;

use modbus::{ConnectionService, DataArea, ModbusService, ModbusStore, STORE_SIZE};

#[derive(Clone)]
struct AppState {
    app: AppHandle,
    store: Arc<RwLock<ModbusStore>>,
    server: Arc<Mutex<ServerRuntimeState>>,
}

#[derive(Default)]
struct ServerRuntimeState {
    runtime: Option<RuntimeState>,
    last_error: Option<String>,
}

struct RuntimeState {
    cancel: CancellationToken,
    handle: tauri::async_runtime::JoinHandle<()>,
    bind: String,
    connections: Arc<AtomicUsize>,
}

#[derive(Serialize, Clone)]
struct ServerStatus {
    running: bool,
    bind: String,
    connections: usize,
    last_error: Option<String>,
}

#[derive(Deserialize)]
struct ServerConfig {
    host: String,
    port: u16,
    unit_id: u8,
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RegisterValue {
    Bool(bool),
    Number(u16),
}

#[derive(Deserialize)]
#[serde(untagged)]
enum RegisterValues {
    Bools(Vec<bool>),
    Numbers(Vec<u16>),
}

#[tauri::command]
async fn server_start(config: ServerConfig, state: State<'_, AppState>) -> Result<ServerStatus, String> {
    {
        let mut server_state =
            state.server.lock().map_err(|_| "State lock poisoned".to_string())?;
        if server_state.runtime.is_some() {
            return Ok(build_status(&server_state));
        }
        server_state.last_error = None;
    }

    let addr: SocketAddr = format!("{}:{}", config.host, config.port)
        .parse()
        .map_err(|err: std::net::AddrParseError| err.to_string())?;

    let listener = match TcpListener::bind(addr).await {
        Ok(listener) => listener,
        Err(err) => {
            let mut server_state = state
                .server
                .lock()
                .map_err(|_| "State lock poisoned".to_string())?;
            server_state.last_error = Some(err.to_string());
            let status = build_status(&server_state);
            let _ = state.app.emit("modbus://status", status.clone());
            return Err(err.to_string());
        }
    };

    let bind = listener
        .local_addr()
        .map_err(|err| err.to_string())?
        .to_string();
    let cancel = CancellationToken::new();
    let cancel_for_task = cancel.clone();
    let connections = Arc::new(AtomicUsize::new(0));
    let connections_for_runtime = connections.clone();
    let app = state.app.clone();
    let store = state.store.clone();
    let server_state = state.server.clone();
    let unit_id = config.unit_id;

    let task = tauri::async_runtime::spawn(async move {
        let base_service = ModbusService::new(store, app.clone(), unit_id);
        let on_connected = move |stream, _socket_addr| {
            let base_service = base_service.clone();
            let connections = connections.clone();
            async move {
                connections.fetch_add(1, Ordering::SeqCst);
                Ok(Some((ConnectionService::new(base_service, connections), stream)))
            }
        };

        let on_error = {
            let app = app.clone();
            let server_state = server_state.clone();
            move |err: std::io::Error| {
                let mut state = server_state.lock().unwrap();
                state.last_error = Some(err.to_string());
                let status = build_status(&state);
                let _ = app.emit("modbus://status", status);
            }
        };

        let abort_signal = async move {
            cancel_for_task.cancelled().await;
        };
        let result = Server::new(listener)
            .serve_until(&on_connected, on_error, abort_signal)
            .await;

        let mut state = server_state.lock().unwrap();
        if let Err(err) = result {
            state.last_error = Some(err.to_string());
        }
        state.runtime = None;
        let status = build_status(&state);
        let _ = app.emit("modbus://status", status);
    });

    let mut server_state = state
        .server
        .lock()
        .map_err(|_| "State lock poisoned".to_string())?;
    server_state.runtime = Some(RuntimeState {
        cancel,
        handle: task,
        bind: bind.clone(),
        connections: connections_for_runtime,
    });

    let status = build_status(&server_state);
    let _ = state.app.emit("modbus://status", status.clone());
    Ok(status)
}

#[tauri::command]
async fn server_stop(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    let runtime = {
        let mut server_state = state
            .server
            .lock()
            .map_err(|_| "State lock poisoned".to_string())?;
        server_state.runtime.take()
    };

    if let Some(runtime) = runtime {
        runtime.cancel.cancel();
        runtime.handle.abort();
    }

    let server_state = state
        .server
        .lock()
        .map_err(|_| "State lock poisoned".to_string())?;
    let status = build_status(&server_state);
    let _ = state.app.emit("modbus://status", status.clone());
    Ok(status)
}

#[tauri::command]
fn server_status(state: State<'_, AppState>) -> Result<ServerStatus, String> {
    let server_state = state
        .server
        .lock()
        .map_err(|_| "State lock poisoned".to_string())?;
    Ok(build_status(&server_state))
}

#[tauri::command]
fn register_snapshot(
    area: DataArea,
    offset: u16,
    len: u16,
    state: State<'_, AppState>,
) -> Result<Vec<u16>, String> {
    let store = state
        .store
        .read()
        .map_err(|_| "Store lock poisoned".to_string())?;
    let start = offset as usize;
    let end = start + len as usize;
    if end > STORE_SIZE {
        return Err("Requested range is out of bounds".to_string());
    }

    let values = match area {
        DataArea::Coils => store.coils[start..end]
            .iter()
            .map(|value| if *value { 1 } else { 0 })
            .collect(),
        DataArea::DiscreteInputs => store.discrete_inputs[start..end]
            .iter()
            .map(|value| if *value { 1 } else { 0 })
            .collect(),
        DataArea::InputRegisters => store.input_registers[start..end].to_vec(),
        DataArea::HoldingRegisters => store.holding_registers[start..end].to_vec(),
    };

    Ok(values)
}

#[tauri::command]
fn register_set(
    area: DataArea,
    offset: u16,
    value: RegisterValue,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut store = state
        .store
        .write()
        .map_err(|_| "Store lock poisoned".to_string())?;
    let index = offset as usize;
    if index >= STORE_SIZE {
        return Err("Offset is out of bounds".to_string());
    }

    match area {
        DataArea::Coils => {
            store.coils[index] = value.as_bool();
        }
        DataArea::DiscreteInputs => {
            store.discrete_inputs[index] = value.as_bool();
        }
        DataArea::InputRegisters => {
            store.input_registers[index] = value.as_u16();
        }
        DataArea::HoldingRegisters => {
            store.holding_registers[index] = value.as_u16();
        }
    }

    Ok(())
}

#[tauri::command]
fn register_set_range(
    area: DataArea,
    offset: u16,
    values: RegisterValues,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let mut store = state
        .store
        .write()
        .map_err(|_| "Store lock poisoned".to_string())?;
    let start = offset as usize;

    match area {
        DataArea::Coils => {
            let data = values.into_bools();
            let end = start + data.len();
            if end > STORE_SIZE {
                return Err("Range is out of bounds".to_string());
            }
            store.coils[start..end].copy_from_slice(&data);
        }
        DataArea::DiscreteInputs => {
            let data = values.into_bools();
            let end = start + data.len();
            if end > STORE_SIZE {
                return Err("Range is out of bounds".to_string());
            }
            store.discrete_inputs[start..end].copy_from_slice(&data);
        }
        DataArea::InputRegisters => {
            let data = values.into_u16s();
            let end = start + data.len();
            if end > STORE_SIZE {
                return Err("Range is out of bounds".to_string());
            }
            store.input_registers[start..end].copy_from_slice(&data);
        }
        DataArea::HoldingRegisters => {
            let data = values.into_u16s();
            let end = start + data.len();
            if end > STORE_SIZE {
                return Err("Range is out of bounds".to_string());
            }
            store.holding_registers[start..end].copy_from_slice(&data);
        }
    }

    Ok(())
}

impl RegisterValue {
    fn as_bool(&self) -> bool {
        match self {
            RegisterValue::Bool(value) => *value,
            RegisterValue::Number(value) => *value != 0,
        }
    }

    fn as_u16(&self) -> u16 {
        match self {
            RegisterValue::Bool(value) => if *value { 1 } else { 0 },
            RegisterValue::Number(value) => *value,
        }
    }
}

impl RegisterValues {
    fn into_bools(self) -> Vec<bool> {
        match self {
            RegisterValues::Bools(values) => values,
            RegisterValues::Numbers(values) => values.into_iter().map(|value| value != 0).collect(),
        }
    }

    fn into_u16s(self) -> Vec<u16> {
        match self {
            RegisterValues::Bools(values) => values.into_iter().map(|value| if value { 1 } else { 0 }).collect(),
            RegisterValues::Numbers(values) => values,
        }
    }
}

fn build_status(state: &ServerRuntimeState) -> ServerStatus {
    if let Some(runtime) = &state.runtime {
        ServerStatus {
            running: true,
            bind: runtime.bind.clone(),
            connections: runtime.connections.load(Ordering::SeqCst),
            last_error: state.last_error.clone(),
        }
    } else {
        ServerStatus {
            running: false,
            bind: String::new(),
            connections: 0,
            last_error: state.last_error.clone(),
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let store = Arc::new(RwLock::new(ModbusStore::new(STORE_SIZE)));
            let server = Arc::new(Mutex::new(ServerRuntimeState::default()));
            app.manage(AppState {
                app: app.handle().clone(),
                store,
                server,
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            server_start,
            server_stop,
            server_status,
            register_snapshot,
            register_set,
            register_set_range
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
