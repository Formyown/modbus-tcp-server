use std::env;
use std::error::Error;
use std::net::SocketAddr;

use tokio::time::{sleep, Duration};
use tokio_modbus::client::tcp;
use tokio_modbus::prelude::*;

const IO_COUNT: usize = 32;
const STEP_DELAY_MS: u64 = 200;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!(
            "Usage: {} <ip> <port> [unit_id]\nExample: {} 127.0.0.1 502 1",
            args.get(0).map(String::as_str).unwrap_or("modbus_client"),
            args.get(0).map(String::as_str).unwrap_or("modbus_client")
        );
        return Ok(());
    }

    let ip = &args[1];
    let port: u16 = args[2].parse()?;
    let unit_id: u8 = if args.len() > 3 { args[3].parse()? } else { 1 };
    let socket_addr: SocketAddr = format!("{ip}:{port}").parse()?;

    println!("Connecting to {socket_addr} (unit id {unit_id})...");
    let mut ctx = tcp::connect_slave(socket_addr, Slave(unit_id)).await?;

    let mut output_index = 0usize;
    let mut last_inputs = vec![false; IO_COUNT];

    loop {
        let mut outputs = vec![false; IO_COUNT];
        outputs[output_index] = true;

        ctx.write_multiple_coils(0, &outputs).await??;
        output_index = (output_index + 1) % IO_COUNT;

        let inputs = ctx.read_discrete_inputs(0, IO_COUNT as u16).await??;
        for (idx, value) in inputs.iter().enumerate() {
            if *value != last_inputs[idx] {
                println!(
                    "DI[{}] {} -> {}",
                    idx,
                    if last_inputs[idx] { 1 } else { 0 },
                    if *value { 1 } else { 0 }
                );
                last_inputs[idx] = *value;
            }
        }

        sleep(Duration::from_millis(STEP_DELAY_MS)).await;
    }
}
