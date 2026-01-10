import type { paths } from "@octokit/openapi-types";

type LatestReleaseResponse =
  paths["/repos/{owner}/{repo}/releases/latest"]["get"]["responses"]["200"]["content"]["application/json"];

async function fetchData(): Promise<LatestReleaseResponse> {
  return (await (
    await fetch(
      "https://api.github.com/repos/SkinsRestorer/SkinsRestorer/releases/latest",
    )
  ).json()) as LatestReleaseResponse;
}

let metadata: LatestReleaseResponse = await fetchData();
setInterval(
  () => {
    void fetchData()
      .then((data) => {
        metadata = data;
      })
      .catch(console.error);
  },
  20 * 60 * 1_000,
); // 3 times per hour

export function getMetadata(): LatestReleaseResponse {
  return metadata;
}
