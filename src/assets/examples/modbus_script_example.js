// Modbus TCP Script Example
// ---------------------------------------------
// This is a starter template for automation logic.
// Replace the pseudo APIs with your actual runtime helpers.

// 1) Connect to server
// const client = connect({ host: "127.0.0.1", port: 1502, unitId: 1 });

// 2) Write single register
// await client.writeHoldingRegister(0, 123);

// 3) Write multiple registers
// await client.writeHoldingRegisters(10, [11, 22, 33, 44]);

// 4) Read registers and log
// const values = await client.readHoldingRegisters(0, 8);
// log("Holding[0..7]", values);

// 5) Poll loop template
// while (true) {
//   const snapshot = await client.readInputRegisters(0, 4);
//   log("Input[0..3]", snapshot);
//   await sleep(1000);
// }

// 6) Close connection
// await client.close();
