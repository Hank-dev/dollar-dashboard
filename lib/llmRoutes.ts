// Shared bodies for the ask (streaming) and explain (cached JSON) routes, so
// every dashboard gets the same validation, spend limits and error handling.
import { anthropic, describeAnthropicError, messageText, MODELS } from "./anthropic";
import { llmGuard, readQuestion } from "./llmGuard";
import { cached } from "./routeCache";

export async function answerQuestion(
  req: Request,
  label: string,
  maxTokens: number,
  buildSystem: () => Promise<string> | string,
): Promise<Response> {
  const question = await readQuestion(req);
  if (question instanceof Response) return question;
  const refusal = llmGuard(req);
  if (refusal) return refusal;

  let system: string;
  try {
    system = await buildSystem();
  } catch (err) {
    console.error(`${label}: could not build context:`, err);
    return Response.json({ error: "Dashboard data is unavailable right now." }, { status: 502 });
  }

  const stream = anthropic.messages.stream({
    model: MODELS.ask,
    max_tokens: maxTokens,
    system,
    messages: [{ role: "user", content: question }],
  });

  const encoder = new TextEncoder();
  const body = new ReadableStream<Uint8Array>({
    start(controller) {
      stream.on("text", (text) => controller.enqueue(encoder.encode(text)));
      stream.finalMessage().then(
        () => controller.close(),
        (err) => {
          // The 200 is already sent, so say what happened in the answer itself
          // instead of leaving a silently broken stream.
          console.error(`${label} stream error:`, err);
          controller.enqueue(
            encoder.encode(`\n\n[Answer interrupted: ${describeAnthropicError(err).message}]`),
          );
          controller.close();
        },
      );
    },
    cancel() {
      stream.abort();
    },
  });

  return new Response(body, {
    headers: {
      "Content-Type": "text/plain; charset=utf-8",
      "Cache-Control": "no-store",
    },
  });
}

class Refused extends Error {
  constructor(readonly response: Response) {
    super("refused");
  }
}

/**
 * One explanation per `cacheKey` per half hour. A repeat click is served from
 * memory and does not count against the request limits.
 */
export async function explainItem(
  req: Request,
  label: string,
  cacheKey: string,
  maxTokens: number,
  system: string,
  userContent: string,
): Promise<Response> {
  try {
    const hit = await cached(`explain:${label}:${cacheKey}`, { ttlSeconds: 1800 }, async () => {
      const refusal = llmGuard(req);
      if (refusal) throw new Refused(refusal);
      const msg = await anthropic.messages.create({
        model: MODELS.explain,
        max_tokens: maxTokens,
        system,
        messages: [{ role: "user", content: userContent }],
      });
      const text = messageText(msg);
      if (!text) throw new Error("empty explanation");
      return text;
    });
    return Response.json({ explanation: hit.value });
  } catch (err) {
    if (err instanceof Refused) return err.response;
    console.error(`${label} explain error:`, err);
    const { status, message } = describeAnthropicError(err);
    return Response.json({ error: message }, { status });
  }
}
