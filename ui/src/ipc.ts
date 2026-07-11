// WebSocket client for keyscape-core. Auto-reconnects; requests are matched
// to replies via an echoed "req" id; preview frames arrive as binary.

export type Json = Record<string, any>;

const URL = "ws://127.0.0.1:53971";

type Pending = { resolve: (v: Json) => void; timer: number };

class CoreClient {
  private ws: WebSocket | null = null;
  private pending = new Map<number, Pending>();
  private seq = 1;
  private frameCbs = new Set<(b: Uint8Array) => void>();
  private stateCbs = new Set<(online: boolean) => void>();
  private wantPreview = false;
  private retryTimer = 0;
  online = false;

  connect() {
    if (this.ws) return;
    const ws = new WebSocket(URL);
    ws.binaryType = "arraybuffer";
    this.ws = ws;

    ws.onopen = () => {
      this.online = true;
      this.stateCbs.forEach((cb) => cb(true));
      if (this.wantPreview) void this.req("subscribe_preview");
    };
    ws.onmessage = (ev) => {
      if (ev.data instanceof ArrayBuffer) {
        const bytes = new Uint8Array(ev.data);
        this.frameCbs.forEach((cb) => cb(bytes));
        return;
      }
      try {
        const msg = JSON.parse(ev.data as string);
        const id = msg.req as number | undefined;
        if (id !== undefined && this.pending.has(id)) {
          const p = this.pending.get(id)!;
          clearTimeout(p.timer);
          this.pending.delete(id);
          p.resolve(msg);
        }
      } catch {
        /* ignore malformed */
      }
    };
    const drop = () => {
      if (this.ws !== ws) return;
      this.ws = null;
      if (this.online) {
        this.online = false;
        this.stateCbs.forEach((cb) => cb(false));
      }
      clearTimeout(this.retryTimer);
      this.retryTimer = window.setTimeout(() => this.connect(), 1600);
    };
    ws.onclose = drop;
    ws.onerror = drop;
  }

  req(op: string, extra: Json = {}): Promise<Json> {
    return new Promise((resolve) => {
      if (!this.ws || this.ws.readyState !== WebSocket.OPEN) {
        resolve({ ok: false, error: "offline" });
        return;
      }
      const id = this.seq++;
      const timer = window.setTimeout(() => {
        this.pending.delete(id);
        resolve({ ok: false, error: "timeout" });
      }, 2500);
      this.pending.set(id, { resolve, timer });
      this.ws.send(JSON.stringify({ op, req: id, ...extra }));
    });
  }

  subscribePreview() {
    this.wantPreview = true;
    if (this.online) void this.req("subscribe_preview");
  }

  onFrame(cb: (b: Uint8Array) => void) {
    this.frameCbs.add(cb);
  }

  onState(cb: (online: boolean) => void) {
    this.stateCbs.add(cb);
  }
}

export const core = new CoreClient();
