export interface FrontierModelCandidate {
  id: string;
  label: string;
  provider: string;
  aiApiCostPath: string;
  openRouterId?: string;
}

export interface FrontierModelPoint {
  id: string;
  label: string;
  provider: string;
  intelligenceIndex: number;
  inputUsdPerMillion: number;
  outputUsdPerMillion: number;
  blendedUsdPerMillion: number;
  blendedUsdPerToken: number;
  sourceUrl: string;
  pricingSource: "AI API Cost" | "OpenRouter";
}

export interface FrontierModelsResponse {
  capturedAt: string;
  points: FrontierModelPoint[];
  errors: { id: string; message: string }[];
}

export const FRONTIER_MODEL_CANDIDATES: FrontierModelCandidate[] = [
  {
    id: "gpt-5-5",
    label: "GPT-5.5",
    provider: "OpenAI",
    aiApiCostPath: "/models/openai/gpt-5-5",
    openRouterId: "openai/gpt-5.5",
  },
  {
    id: "claude-opus-4-7",
    label: "Claude Opus 4.7",
    provider: "Anthropic",
    aiApiCostPath: "/models/anthropic/claude-opus-4-7",
    openRouterId: "anthropic/claude-opus-4.7",
  },
  {
    id: "qwen3-7-max",
    label: "Qwen3.7 Max",
    provider: "Alibaba",
    aiApiCostPath: "/models/alibaba/qwen3-7-max",
    openRouterId: "qwen/qwen3.7-max",
  },
  {
    id: "grok-4-3",
    label: "Grok 4.3",
    provider: "xAI",
    aiApiCostPath: "/models/xai/grok-4-3",
    openRouterId: "x-ai/grok-4.3",
  },
  {
    id: "deepseek-v4-pro",
    label: "DeepSeek V4 Pro",
    provider: "DeepSeek",
    aiApiCostPath: "/models/deepseek/deepseek-v4-pro",
    openRouterId: "deepseek/deepseek-v4-pro",
  },
  {
    id: "gemini-3-pro",
    label: "Gemini 3 Pro",
    provider: "Google",
    aiApiCostPath: "/models/google/gemini-3-pro",
  },
  {
    id: "claude-sonnet-4-6",
    label: "Claude Sonnet 4.6",
    provider: "Anthropic",
    aiApiCostPath: "/models/anthropic/claude-sonnet-4-6",
    openRouterId: "anthropic/claude-sonnet-4.6",
  },
  {
    id: "mistral-large-3",
    label: "Mistral Large 3",
    provider: "Mistral",
    aiApiCostPath: "/models/mistral/mistral-large-3",
  },
];
