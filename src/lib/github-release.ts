import type { paths } from "@octokit/openapi-types";

export type LatestReleaseResponse =
  paths["/repos/{owner}/{repo}/releases/latest"]["get"]["responses"]["200"]["content"]["application/json"];

export type LatestReleaseProvider = () => LatestReleaseResponse | null;

const providerCache = new Map<string, LatestReleaseProvider>();

const refreshIntervalMs = 20 * 60 * 1_000;

const fetchLatestRelease = async (
  releaseUrl: string,
): Promise<LatestReleaseResponse> =>
  (await (await fetch(releaseUrl)).json()) as LatestReleaseResponse;

export const getLatestReleaseProvider = (
  releaseUrl: string,
): LatestReleaseProvider => {
  const cachedProvider = providerCache.get(releaseUrl);
  if (cachedProvider != null) {
    return cachedProvider;
  }

  let latestRelease: LatestReleaseResponse | null = null;

  const refresh = async (): Promise<void> => {
    latestRelease = await fetchLatestRelease(releaseUrl);
  };

  void refresh().catch(console.error);
  setInterval(() => {
    void refresh().catch(console.error);
  }, refreshIntervalMs);

  const provider = (): LatestReleaseResponse | null => latestRelease;
  providerCache.set(releaseUrl, provider);

  return provider;
};
