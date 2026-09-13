// @ts-nocheck
import assert from "node:assert/strict";
import { test } from "node:test";
import http from "node:http";

const EXT = "./voice-speak.ts";

/** A daemon stand-in that records one posted turn. */
function intake() {
  const posts: { path: string; body: string }[] = [];
  const server = http.createServer((request, response) => {
    const chunks: Buffer[] = [];
    request.on("data", (chunk) => chunks.push(chunk));
    request.on("end", () => {
      posts.push({ path: request.url, body: Buffer.concat(chunks).toString("utf8") });
      response.writeHead(204, { connection: "close" });
      response.end();
    });
  });
  return {
    posts,
    listen: () =>
      new Promise<number>((resolve) => {
        server.listen(0, "127.0.0.1", () => resolve(server.address().port));
      }),
    close: () => new Promise<void>((resolve) => server.close(() => resolve())),
  };
}

/** Load the extension with the guard set to `port`, and collect its handlers. */
async function load(port?: number) {
  if (port === undefined) delete process.env.MAESTRO_VOICE_PORT;
  else process.env.MAESTRO_VOICE_PORT = String(port);
  const mod = await import(EXT + `?v=${Math.random()}`);
  const handlers = new Map();
  mod.default({ on: (name, handler) => handlers.set(name, handler) });
  return { mod, handlers };
}

const assistant = (...texts: string[]) => ({
  role: "assistant",
  content: texts.map((text) => ({ type: "text", text })),
});

/** Wait until the stand-in has recorded a post, or give up. */
async function settled(posts: unknown[]) {
  for (let i = 0; i < 100 && posts.length === 0; i++) {
    await new Promise((resolve) => setTimeout(resolve, 10));
  }
}

test("it does nothing at all outside the voice pane", async () => {
  const { handlers } = await load(undefined);
  assert.equal(handlers.size, 0, "an ordinary pi session must not be touched");

  const { handlers: bad } = await load("not-a-port" as unknown as number);
  assert.equal(bad.size, 0, "a guard that is not a port is not a guard");
});

test("a finished turn reaches the daemon as the text the agent wrote", async () => {
  const daemon = intake();
  const port = await daemon.listen();
  const { handlers } = await load(port);

  assert.ok(handlers.has("agent_end"), "the text lives on agent_end");
  assert.ok(handlers.has("agent_settled"), "the turn is final on agent_settled");

  handlers.get("agent_end")({ messages: [assistant("Done. <speak>Tests pass.</speak>")] });
  handlers.get("agent_settled")({});
  await settled(daemon.posts);
  await daemon.close();

  assert.equal(daemon.posts.length, 1);
  assert.equal(daemon.posts[0].path, "/turn");
  assert.equal(daemon.posts[0].body, "Done. <speak>Tests pass.</speak>");
});

test("a turn that said nothing is still posted, so it can be counted", async () => {
  const daemon = intake();
  const port = await daemon.listen();
  const { handlers } = await load(port);

  // The agent ended on a tool call: an assistant message with no text in it.
  handlers.get("agent_end")({
    messages: [{ role: "assistant", content: [{ type: "toolCall", name: "bash" }] }],
  });
  handlers.get("agent_settled")({});
  await settled(daemon.posts);
  await daemon.close();

  assert.equal(daemon.posts.length, 1, "a silent turn must not be skipped");
  assert.equal(daemon.posts[0].body, "");
});

test("only the last assistant message counts, and only its text", async () => {
  const { mod } = await load(1);
  const { finalAssistantText } = mod.__testing;

  assert.equal(
    finalAssistantText([assistant("an earlier answer"), { role: "user", content: "again" }, assistant("the final answer")]),
    "the final answer",
  );
  assert.equal(
    finalAssistantText([
      {
        role: "assistant",
        content: [
          { type: "thinking", thinking: "never spoken" },
          { type: "text", text: "said" },
          { type: "toolCall", name: "bash" },
        ],
      },
    ]),
    "said",
  );
  assert.equal(finalAssistantText([]), "");
  assert.equal(finalAssistantText(undefined), "");
});

test("an overlong reply is cut to its tail and stays valid text", async () => {
  const { mod } = await load(1);
  const { tail, MAX_BODY } = mod.__testing;

  // A multi-byte character straddling the cut must not survive as half of
  // itself: the daemon would have to refuse the whole post as unreadable.
  const long = "e\u00e9".repeat(MAX_BODY) + "<speak>fin</speak>";
  const cut = tail(long);

  assert.ok(cut.length <= MAX_BODY, `cut to the bound, was ${cut.length}`);
  assert.equal(Buffer.compare(Buffer.from(cut.toString("utf8"), "utf8"), cut), 0, "the cut must leave valid text");
  assert.ok(cut.toString("utf8").endsWith("<speak>fin</speak>"), "the block is at the end and must survive");
});

test("a daemon that is not listening is not an error the agent ever sees", async () => {
  const daemon = intake();
  const port = await daemon.listen();
  await daemon.close();
  const { mod } = await load(port);

  // Resolving rather than rejecting is the whole contract: the agent's turn
  // must not fail because the speaker was unavailable.
  await mod.__testing.post(Buffer.from("nobody is home", "utf8"));
});
