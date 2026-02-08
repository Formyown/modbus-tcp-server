// Modbus TCP Script Example
// ---------------------------------------------
// Available helpers:
// writeCoils, writeDiscreteInputs, writeHoldingRegs, writeInputRegs, onChange, log, setTimeout,
// setInterval, sleep

// Listen to coils updates and mirror them to discrete inputs (DI)
const stop = onChange(async ({ area, offset, values }) => {
  if (area !== "coils") {
    return;
  }

  const diValues = values.map((value) => Boolean(value));
  await writeDiscreteInputs(offset, diValues);
  log("Synced coils -> DI", "offset", offset, "values", diValues);
});

// Example: seed some coils to see the sync in action
await writeCoils(0, [true, false, true, true]);

// setInterval: periodic log
setInterval(() => {
  log("Heartbeat", new Date().toLocaleTimeString());
}, 1000);

// setTimeout: one-shot log
setTimeout(() => {
  log("Timeout fired after 3s");
}, 3000);

// Stop listening when needed
// stop();
