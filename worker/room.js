// Salle de coop en ligne (objet durable, une par code) : relaie les messages entre 4 joueurs au plus.
// Les jeux font tourner la partie eux-memes ; la salle ne fait que :
//   - donner une place (0-3) a chaque arrivant, refuser une salle pleine, une partie lancee ou une autre
//     version du jeu ;
//   - prefixer chaque message par la place de son expediteur (« 2 {...} ») et l'envoyer aux autres ;
//   - annoncer arrivees et departs (« S {...} »).
// WebSockets en hibernation : la salle ne coute rien tant que personne ne parle ; « ping » recoit « pong »
// sans la reveiller (mesure de l'aller-retour dans le salon).

const MAX = 4;
/// Taille maximale d'un message et nombre de messages par seconde et par joueur (au-dela : deconnecte).
const MAX_LEN = 16384;
const MAX_RATE = 120;

function server(ws, m) {
  try {
    ws.send(`S ${JSON.stringify(m)}`);
  } catch {
    // socket deja fermee
  }
}

export class Room {
  constructor(ctx, env) {
    this.ctx = ctx;
    this.env = env;
    // debit de chaque place (en memoire : remis a zero si la salle hiberne, sans consequence)
    this.rate = new Map();
    if (globalThis.WebSocketRequestResponsePair) {
      ctx.setWebSocketAutoResponse(new WebSocketRequestResponsePair("ping", "pong"));
    }
  }

  /// Joueurs presents (sans la socket qui se ferme).
  present(except) {
    return this.ctx.getWebSockets().filter((ws) => ws !== except && ws.readyState !== 3);
  }

  async fetch(request) {
    const url = new URL(request.url);
    const room = url.searchParams.get("room") || "";
    const version = url.searchParams.get("v") || "";
    const create = url.searchParams.get("create") === "1";
    const socks = this.present();
    // code deja pris : le Worker en tire un autre
    if (create && socks.length > 0) return new Response("taken", { status: 409 });
    const pair = new WebSocketPair();
    const [client, sock] = Object.values(pair);
    this.ctx.acceptWebSocket(sock);
    const used = socks.map((ws) => ws.deserializeAttachment() || {});
    let error = null;
    if (!create && socks.length === 0) error = "no_room";
    else if (used.length >= MAX) error = "full";
    else if (used.some((u) => u.v !== version)) error = "version";
    else if (await this.ctx.storage.get("started")) error = "started";
    if (error) {
      server(sock, { t: "error", e: error });
      sock.close(4000, error);
      return new Response(null, { status: 101, webSocket: client });
    }
    if (create) await this.ctx.storage.deleteAll();
    const taken = new Set(used.map((u) => u.slot));
    let slot = 0;
    while (taken.has(slot)) slot++;
    sock.serializeAttachment({ slot, v: version });
    server(sock, { t: "hello", slot, room, peers: [...taken] });
    for (const ws of socks) server(ws, { t: "join", s: slot });
    return new Response(null, { status: 101, webSocket: client });
  }

  async webSocketMessage(ws, msg) {
    const me = ws.deserializeAttachment();
    if (!me || typeof msg !== "string") return;
    const now = Math.floor(Date.now() / 1000);
    const r = this.rate.get(me.slot);
    const n = r && r.t === now ? r.n + 1 : 1;
    this.rate.set(me.slot, { t: now, n });
    if (msg.length > MAX_LEN || n > MAX_RATE) {
      ws.close(4001, "flood");
      this.leave(ws);
      return;
    }
    // partie lancee : plus personne n'entre jusqu'au retour au salon
    if (msg.startsWith('{"t":"start"')) await this.ctx.storage.put("started", true);
    else if (msg.startsWith('{"t":"lobby"')) await this.ctx.storage.delete("started");
    const out = `${me.slot} ${msg}`;
    for (const other of this.present(ws)) {
      try {
        other.send(out);
      } catch {
        // l'autre joueur vient de partir : sa fermeture est annoncee a part
      }
    }
  }

  async webSocketClose(ws, code) {
    await this.leave(ws);
    try {
      ws.close(code === 1005 ? 1000 : code, "bye");
    } catch {
      // deja fermee
    }
  }

  async webSocketError(ws) {
    await this.leave(ws);
  }

  async leave(ws) {
    const me = ws.deserializeAttachment();
    if (!me || me.gone) return;
    ws.serializeAttachment({ ...me, gone: true });
    const rest = this.present(ws).filter((o) => !(o.deserializeAttachment() || {}).gone);
    for (const other of rest) server(other, { t: "leave", s: me.slot });
    if (rest.length === 0) await this.ctx.storage.deleteAll();
  }
}

const CODE_CHARS = "ABCDEFGHJKLMNPQRSTUVWXYZ";

/// /api/coop/new?v=... (cree une salle) ou /api/coop/ABCD?v=... (la rejoint) : WebSocket vers la salle.
export async function coop(request, env, url) {
  if (request.headers.get("Upgrade") !== "websocket") return new Response("websocket", { status: 426 });
  const want = url.pathname.slice("/api/coop/".length);
  const version = (url.searchParams.get("v") || "").slice(0, 16);
  const open = async (code, create) => {
    const target = new URL(request.url);
    target.search = `?room=${code}&v=${encodeURIComponent(version)}&create=${create ? 1 : 0}`;
    return env.ROOMS.get(env.ROOMS.idFromName(code)).fetch(new Request(target, request));
  };
  if (want === "new") {
    for (let k = 0; k < 8; k++) {
      let code = "";
      const bytes = crypto.getRandomValues(new Uint8Array(4));
      for (const b of bytes) code += CODE_CHARS[b % CODE_CHARS.length];
      const res = await open(code, true);
      if (res.status !== 409) return res;
    }
    return new Response("busy", { status: 503 });
  }
  if (!new RegExp(`^[${CODE_CHARS}]{4}$`).test(want)) return new Response("code", { status: 400 });
  return open(want, false);
}
