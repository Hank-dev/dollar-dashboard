import Anthropic from "@anthropic-ai/sdk";

export const anthropic = new Anthropic({
  apiKey: process.env.ANTHROPIC_API_KEY,
});

// One place for model choice. Each is overridable from the environment so a
// model change is a deploy setting, not a code edit.
const DEFAULT_MODEL = "claude-sonnet-4-6";
export const MODELS = {
  explain: process.env.EXPLAIN_MODEL ?? DEFAULT_MODEL,
  ask: process.env.ASK_MODEL ?? DEFAULT_MODEL,
  briefing: process.env.BRIEFING_MODEL ?? process.env.EXPLAIN_MODEL ?? DEFAULT_MODEL,
  interpretation: process.env.INTERPRETATION_MODEL ?? "claude-haiku-4-5",
};

/** Text of a non-streaming reply. */
export function messageText(message: Anthropic.Message, joiner = "\n"): string {
  return message.content
    .filter((block): block is Anthropic.TextBlock => block.type === "text")
    .map((block) => block.text)
    .join(joiner)
    .trim();
}

/** A short reason safe to show a visitor; details stay in the server log. */
export function describeAnthropicError(err: unknown): { status: number; message: string } {
  if (err instanceof Anthropic.AuthenticationError) {
    return { status: 503, message: "The AI provider rejected this server's API key." };
  }
  if (err instanceof Anthropic.RateLimitError) {
    return { status: 429, message: "The AI provider is rate-limiting this server. Try again shortly." };
  }
  if (err instanceof Anthropic.APIConnectionError) {
    return { status: 502, message: "Could not reach the AI provider." };
  }
  if (err instanceof Anthropic.APIError) {
    return { status: 502, message: "The AI provider returned an error." };
  }
  return { status: 500, message: "Could not generate a response." };
}
