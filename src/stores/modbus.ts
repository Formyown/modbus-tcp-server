import { defineStore } from "pinia";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

export type DataArea = "coils" | "discrete" | "input" | "holding";

export interface ServerConfig {
  host: string;
  port: number;
  unit_id: number;
}

export interface ServerStatus {
  running: boolean;
  bind: string;
  connections: number;
  last_error?: string | null;
}

interface UpdatePayload {
  area: DataArea;
  offset: number;
  values: number[];
}

const STORE_SIZE = 1000;
const DEFAULT_PAGE_SIZE = 20;

export const useModbusStore = defineStore("modbus", {
  state: () => ({
    config: {
      host: "0.0.0.0",
      port: 502,
      unit_id: 1,
    } as ServerConfig,
    status: {
      running: false,
      bind: "",
      connections: 0,
      last_error: null,
    } as ServerStatus,
    area: "holding" as DataArea,
    startAddress: 0,
    pageSize: DEFAULT_PAGE_SIZE,
    values: [] as number[],
    initialized: false,
  }),
  getters: {
    rows: (state) =>
      state.values.map((value, index) => ({
        address: state.startAddress + index,
        value,
      })),
    isBitArea: (state) => state.area === "coils" || state.area === "discrete",
  },
  actions: {
    async initialize() {
      if (this.initialized) {
        return;
      }
      this.initialized = true;
      await this.refreshStatus();
      await this.fetchSnapshot();
      this.setupListeners();
    },
    async refreshStatus() {
      this.status = (await invoke("server_status")) as ServerStatus;
    },
    async startServer() {
      try {
        this.status = (await invoke("server_start", {
          config: this.config,
        })) as ServerStatus;
      } catch (error) {
        this.status = {
          ...this.status,
          last_error: String(error),
        };
      }
    },
    async stopServer() {
      try {
        this.status = (await invoke("server_stop")) as ServerStatus;
      } catch (error) {
        this.status = {
          ...this.status,
          last_error: String(error),
        };
      }
    },
    async fetchSnapshot() {
      const pageSize = this.normalizePageSize(this.pageSize);
      const startAddress = this.normalizeStart(this.startAddress, pageSize);
      this.pageSize = pageSize;
      this.startAddress = startAddress;
      this.values = (await invoke("register_snapshot", {
        area: this.area,
        offset: startAddress,
        len: pageSize,
      })) as number[];
    },
    async setValue(address: number, value: number | boolean) {
      const index = address - this.startAddress;
      const numeric = typeof value === "boolean" ? (value ? 1 : 0) : value;
      if (index >= 0 && index < this.values.length) {
        this.values[index] = numeric;
      }

      const payload = this.isBitArea ? Boolean(value) : numeric;
      await invoke("register_set", {
        area: this.area,
        offset: address,
        value: payload,
      });
    },
    async setRange(offset: number, values: Array<number | boolean>) {
      const payload = this.isBitArea
        ? values.map((value) => Boolean(value))
        : values.map((value) => Number(value));

      await invoke("register_set_range", {
        area: this.area,
        offset,
        values: payload,
      });
    },
    async setArea(area: DataArea) {
      this.area = area;
      await this.fetchSnapshot();
    },
    async applyRange(startAddress: number, pageSize: number) {
      this.startAddress = startAddress;
      this.pageSize = pageSize;
      await this.fetchSnapshot();
    },
    async nextPage() {
      await this.applyRange(this.startAddress + this.pageSize, this.pageSize);
    },
    async prevPage() {
      await this.applyRange(Math.max(0, this.startAddress - this.pageSize), this.pageSize);
    },
    setupListeners() {
      void listen<UpdatePayload>("modbus://updated", (event) => {
        this.applyUpdate(event.payload);
      });
      void listen<ServerStatus>("modbus://status", (event) => {
        this.status = event.payload;
      });
    },
    applyUpdate(payload: UpdatePayload) {
      if (payload.area !== this.area) {
        return;
      }
      const start = this.startAddress;
      const end = this.startAddress + this.pageSize;
      payload.values.forEach((value, index) => {
        const address = payload.offset + index;
        if (address >= start && address < end) {
          this.values[address - start] = value;
        }
      });
    },
    normalizePageSize(value: number) {
      const safe = Math.max(1, Math.min(100, Math.floor(value || DEFAULT_PAGE_SIZE)));
      return Math.min(safe, STORE_SIZE);
    },
    normalizeStart(value: number, pageSize: number) {
      const maxStart = Math.max(0, STORE_SIZE - pageSize);
      return Math.max(0, Math.min(maxStart, Math.floor(value || 0)));
    },
  },
});
