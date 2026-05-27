export interface XSummaryCitation {
  title: string;
  url?: string;
}

export interface XFeedItem {
  title: string;
  url: string;
  handle?: string;
}

export interface XSummaryResponse {
  checkedAt: string;
  nextRefreshAt: string;
  refreshCadenceHours: number;
  cacheStatus: "fresh" | "cached" | "stale";
  fromDate: string;
  toDate: string;
  model: string;
  summary: string;
  citations: XSummaryCitation[];
  feed: XFeedItem[];
}

export interface XSummaryError {
  error: string;
  setup?: string;
}
