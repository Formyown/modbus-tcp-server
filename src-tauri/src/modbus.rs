use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, RwLock};

use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter};
use tokio_modbus::server::Service;
use tokio_modbus::{ExceptionCode, Request, Response, SlaveRequest};

pub const STORE_SIZE: usize = 1000;

#[derive(Debug)]
pub struct ModbusStore {
    pub coils: Vec<bool>,
    pub discrete_inputs: Vec<bool>,
    pub input_registers: Vec<u16>,
    pub holding_registers: Vec<u16>,
}

impl ModbusStore {
    pub fn new(size: usize) -> Self {
        Self {
            coils: vec![false; size],
            discrete_inputs: vec![false; size],
            input_registers: vec![0; size],
            holding_registers: vec![0; size],
        }
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize, PartialEq, Eq)]
pub enum DataArea {
    #[serde(rename = "coils")]
    Coils,
    #[serde(rename = "discrete")]
    DiscreteInputs,
    #[serde(rename = "input")]
    InputRegisters,
    #[serde(rename = "holding")]
    HoldingRegisters,
}

#[derive(Clone)]
pub struct ModbusService {
    store: Arc<RwLock<ModbusStore>>,
    app: AppHandle,
    unit_id: u8,
}

impl ModbusService {
    pub fn new(store: Arc<RwLock<ModbusStore>>, app: AppHandle, unit_id: u8) -> Self {
        Self {
            store,
            app,
            unit_id,
        }
    }
}

pub struct ConnectionService {
    inner: ModbusService,
    connections: Arc<AtomicUsize>,
    on_status_update: Arc<dyn Fn() + Send + Sync>,
}

impl ConnectionService {
    pub fn new(
        inner: ModbusService,
        connections: Arc<AtomicUsize>,
        on_status_update: Arc<dyn Fn() + Send + Sync>,
    ) -> Self {
        Self {
            inner,
            connections,
            on_status_update,
        }
    }
}

impl Drop for ConnectionService {
    fn drop(&mut self) {
        self.connections.fetch_sub(1, Ordering::SeqCst);
        (self.on_status_update)();
    }
}

#[derive(Clone, Serialize)]
struct UpdatePayload {
    area: DataArea,
    offset: u16,
    values: Vec<u16>,
}

impl Service for ConnectionService {
    type Request = SlaveRequest<'static>;
    type Response = Option<Response>;
    type Exception = ExceptionCode;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Exception>> + Send>>;

    fn call(&self, req: Self::Request) -> Self::Future {
        let store = self.inner.store.clone();
        let app = self.inner.app.clone();
        let unit_id = self.inner.unit_id;
        Box::pin(async move { handle_request(store, app, unit_id, req) })
    }
}

fn handle_request(
    store: Arc<RwLock<ModbusStore>>,
    app: AppHandle,
    unit_id: u8,
    req: SlaveRequest<'static>,
) -> Result<Option<Response>, ExceptionCode> {
    if unit_id != 0 && req.slave != unit_id {
        return Ok(None);
    }

    match req.request {
        Request::ReadCoils(addr, qty) => {
            let store = store.read().map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let values = slice_bool(&store.coils, addr, qty)?;
            Ok(Some(Response::ReadCoils(values)))
        }
        Request::ReadDiscreteInputs(addr, qty) => {
            let store = store.read().map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let values = slice_bool(&store.discrete_inputs, addr, qty)?;
            Ok(Some(Response::ReadDiscreteInputs(values)))
        }
        Request::ReadInputRegisters(addr, qty) => {
            let store = store.read().map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let values = slice_u16(&store.input_registers, addr, qty)?;
            Ok(Some(Response::ReadInputRegisters(values)))
        }
        Request::ReadHoldingRegisters(addr, qty) => {
            let store = store.read().map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let values = slice_u16(&store.holding_registers, addr, qty)?;
            Ok(Some(Response::ReadHoldingRegisters(values)))
        }
        Request::WriteSingleCoil(addr, coil) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            write_bool(&mut store.coils, addr, coil)?;
            emit_update(&app, DataArea::Coils, addr, vec![if coil { 1 } else { 0 }]);
            Ok(Some(Response::WriteSingleCoil(addr, coil)))
        }
        Request::WriteMultipleCoils(addr, coils) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let written = write_bools(&mut store.coils, addr, &coils)?;
            emit_update(&app, DataArea::Coils, addr, bools_to_u16(&coils));
            Ok(Some(Response::WriteMultipleCoils(addr, written)))
        }
        Request::WriteSingleRegister(addr, word) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            write_u16(&mut store.holding_registers, addr, word)?;
            emit_update(&app, DataArea::HoldingRegisters, addr, vec![word]);
            Ok(Some(Response::WriteSingleRegister(addr, word)))
        }
        Request::WriteMultipleRegisters(addr, words) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let written = write_u16s(&mut store.holding_registers, addr, &words)?;
            emit_update(&app, DataArea::HoldingRegisters, addr, words.to_vec());
            Ok(Some(Response::WriteMultipleRegisters(addr, written)))
        }
        Request::MaskWriteRegister(addr, and_mask, or_mask) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            let current = read_single_u16(&store.holding_registers, addr)?;
            let next = (current & and_mask) | (or_mask);
            write_u16(&mut store.holding_registers, addr, next)?;
            emit_update(&app, DataArea::HoldingRegisters, addr, vec![next]);
            Ok(Some(Response::MaskWriteRegister(addr, and_mask, or_mask)))
        }
        Request::ReadWriteMultipleRegisters(read_addr, read_qty, write_addr, words) => {
            let mut store = store
                .write()
                .map_err(|_| ExceptionCode::ServerDeviceFailure)?;
            write_u16s(&mut store.holding_registers, write_addr, &words)?;
            emit_update(&app, DataArea::HoldingRegisters, write_addr, words.to_vec());
            let values = slice_u16(&store.holding_registers, read_addr, read_qty)?;
            Ok(Some(Response::ReadWriteMultipleRegisters(values)))
        }
        Request::ReportServerId
        | Request::ReadDeviceIdentification(_, _)
        | Request::Custom(_, _) => Err(ExceptionCode::IllegalFunction),
    }
}

fn emit_update(app: &AppHandle, area: DataArea, offset: u16, values: Vec<u16>) {
    let payload = UpdatePayload {
        area,
        offset,
        values,
    };
    let _ = app.emit("modbus://updated", payload);
}

fn slice_bool(values: &[bool], addr: u16, qty: u16) -> Result<Vec<bool>, ExceptionCode> {
    let (start, end) = range(values.len(), addr, qty)?;
    Ok(values[start..end].to_vec())
}

fn slice_u16(values: &[u16], addr: u16, qty: u16) -> Result<Vec<u16>, ExceptionCode> {
    let (start, end) = range(values.len(), addr, qty)?;
    Ok(values[start..end].to_vec())
}

fn read_single_u16(values: &[u16], addr: u16) -> Result<u16, ExceptionCode> {
    let index = addr as usize;
    values
        .get(index)
        .copied()
        .ok_or(ExceptionCode::IllegalDataAddress)
}

fn write_bool(values: &mut [bool], addr: u16, value: bool) -> Result<(), ExceptionCode> {
    let index = addr as usize;
    if index >= values.len() {
        return Err(ExceptionCode::IllegalDataAddress);
    }
    values[index] = value;
    Ok(())
}

fn write_bools(values: &mut [bool], addr: u16, data: &[bool]) -> Result<u16, ExceptionCode> {
    let start = addr as usize;
    let end = start + data.len();
    if end > values.len() {
        return Err(ExceptionCode::IllegalDataAddress);
    }
    values[start..end].copy_from_slice(data);
    Ok(data.len() as u16)
}

fn write_u16(values: &mut [u16], addr: u16, value: u16) -> Result<(), ExceptionCode> {
    let index = addr as usize;
    if index >= values.len() {
        return Err(ExceptionCode::IllegalDataAddress);
    }
    values[index] = value;
    Ok(())
}

fn write_u16s(values: &mut [u16], addr: u16, data: &[u16]) -> Result<u16, ExceptionCode> {
    let start = addr as usize;
    let end = start + data.len();
    if end > values.len() {
        return Err(ExceptionCode::IllegalDataAddress);
    }
    values[start..end].copy_from_slice(data);
    Ok(data.len() as u16)
}

fn range(len: usize, addr: u16, qty: u16) -> Result<(usize, usize), ExceptionCode> {
    let start = addr as usize;
    let end = start + qty as usize;
    if end > len {
        return Err(ExceptionCode::IllegalDataAddress);
    }
    Ok((start, end))
}

fn bools_to_u16(values: &[bool]) -> Vec<u16> {
    values.iter().map(|value| if *value { 1 } else { 0 }).collect()
}
