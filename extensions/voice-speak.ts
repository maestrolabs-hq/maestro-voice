// user hook: hands a finished turn to the maestro-voice daemon so the spoken
// block in the reply can be read aloud.
//
// Active only in the pane the daemon started, and best-effort by design: if the
// daemon is not listening, this does nothing and says nothing. The agent's turn
// is never delayed, retried into, or failed because the speaker was unavailable.
//
// It makes no judgement about the reply. The whole tail of the final assistant
// message is posted and the daemon decides whether it holds a spoken block,
// because those rules belong in one place and that place is the side with the
// tests. See src/speak.rs.
// @ts-nocheck

import http from "node:http";

/**
 * The daemon's intake port. Its presence is also what marks this pane as the
 * voice pane: the daemon sets it on the agent it starts, exactly as Herdr sets
 * HERDR_ENV on the panes it manages, so an ordinary pi session elsewhere on the
 * machine loads this file and does nothing.
 */
const PORT = process.env.MAESTRO_VOICE_PORT;

/** Matches MAX_BODY in src/intake.rs. A larger post is refused there. */
const MAX_BODY = 16 * 1024;

/** A local post of a few kilobytes; a second is already generous. */
const TIMEOUT_MS = 1000;

function enabled(): boolean {
  return typeof PORT === "string" && /^[0-9]{1,5}$/.test(PORT);
}

/**
 * The text of the last assistant message, which is where a spoken block lives.
 *
 * Only `text` parts count. Thinking is not for the speaker and tool calls are
 * not prose. A turn whose last assistant message is all tool calls yields an
 * empty string, which is the correct answer: nothing was said.
 */
function finalAssistantText(messages: unknown): string {
  if (!Array.isArray(messages)) return "";
  for (let i = messages.length - 1; i >= 0; i--) {
    const message = messages[i];
    if (!message || message.role !== "assistant") continue;
    if (!Array.isArray(message.content)) return "";
    return message.content
      .filter((part) => part && part.type === "text" && typeof part.text === "string")
      .map((part) => part.text)
      .join("");
  }
  return "";
}

/**
 * The last MAX_BODY bytes, still valid UTF-8.
 *
 * The spoken block is written last, so the tail is the part that matters and
 * cutting the front loses nothing the daemon reads. Cutting by bytes can land
 * inside a multi-byte character, which would reach the daemon as text it must
 * refuse, so any leading continuation bytes are dropped.
 */
function tail(text: string): Buffer {
  let bytes = Buffer.from(text, "utf8");
  if (bytes.length > MAX_BODY) bytes = bytes.subarray(bytes.length - MAX_BODY);
  let start = 0;
  while (start < bytes.length && (bytes[start] & 0xc0) === 0x80) start++;
  return bytes.subarray(start);
}

/** Post one finished turn. Never throws, never waits on the agent. */
function post(body: Buffer): Promise<void> {
  return new Promise((resolve) => {
    let settled = false;
    const finish = () => {
      if (settled) return;
      settled = true;
      resolve();
    };

    const request = http.request(
      {
        host: "127.0.0.1",
        port: Number(PORT),
        path: "/turn",
        method: "POST",
        headers: {
          "content-type": "text/plain; charset=utf-8",
          "content-length": body.length,
          connection: "close",
        },
      },
      (response) => {
        response.resume();
        response.on("end", finish);
        response.on("error", finish);
      },
    );

    request.setTimeout(TIMEOUT_MS, () => {
      request.destroy();
      finish();
    });
    request.on("error", finish);
    request.end(body);
  });
}

export default function (pi) {
  if (!enabled()) return;

  // agent_settled is the event that means the turn is really over, with no
  // retry, compaction or queued continuation still to come -- but it carries
  // only its own type. The text lives on agent_end, which can fire more than
  // once before a turn settles. So the latest one is kept and posted when the
  // turn settles, which is the only point at which the reply is final.
  let latest = "";

  pi.on("agent_end", (event) => {
    latest = finalAssistantText(event?.messages);
  });

  pi.on("agent_settled", () => {
    const body = tail(latest);
    // A turn that settles with no spoken block, or with no assistant text at
    // all, is posted as an empty body rather than skipped. The design accepted
    // that the block depends on the agent remembering it, on the condition that
    // a turn without one is countable instead of silent.
    latest = "";
    void post(body);
  });
}

export const __testing = { enabled, finalAssistantText, tail, post, MAX_BODY };
